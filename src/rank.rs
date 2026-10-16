//! Real OmaStat world rank for users who opted in to the survey.
//! Without opt-in the engine never contacts OmaStat and keeps the offline estimate.

use crate::score::OmaRankResult;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// A successful rank lookup is reused for a day (keyed by score).
const RANK_CACHE_MAX_AGE_SECS: u64 = 24 * 3600;
/// After a failed lookup, wait this long before trying again so an offline bar poll
/// does not pay the network timeout every time.
const RANK_RETRY_AFTER_SECS: u64 = 15 * 60;
/// Hard cap on the time spent querying all endpoints together.
const RANK_TOTAL_BUDGET: Duration = Duration::from_millis(2500);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct RankCache {
    score: u32,
    rank: u32,
    total: u32,
    percentile: f64,
    fetched_epoch: u64,
    ok: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct RankResponse {
    score: u32,
    rank: u32,
    total: u32,
    percentile: Option<f64>,
}

fn rank_cache_path() -> Option<std::path::PathBuf> {
    crate::securefs::xdg_path("XDG_CACHE_HOME", ".cache", "rank.json")
}

fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Some(true): usable rank; Some(false): recent failure, do not retry yet; None: stale/other score.
fn cache_state(cache: &RankCache, score: u32, now: u64) -> Option<bool> {
    if cache.score != score || cache.fetched_epoch > now {
        return None;
    }
    let age = now - cache.fetched_epoch;
    if cache.ok && age < RANK_CACHE_MAX_AGE_SECS {
        Some(true)
    } else if !cache.ok && age < RANK_RETRY_AFTER_SECS {
        Some(false)
    } else {
        None
    }
}

/// Validates the server answer; a rank outside 1..=total or a mismatched score is rejected.
fn parse_rank_response(body: &[u8], score: u32) -> Option<RankCache> {
    let resp: RankResponse = serde_json::from_slice(body).ok()?;
    if resp.score != score || resp.rank == 0 || resp.total == 0 || resp.rank > resp.total {
        return None;
    }
    let percentile = match resp.percentile {
        Some(p) if p.is_finite() && (0.0..=100.0).contains(&p) => p,
        Some(_) => return None,
        None => resp.rank as f64 / resp.total as f64 * 100.0,
    };
    Some(RankCache {
        score,
        rank: resp.rank,
        total: resp.total,
        percentile,
        fetched_epoch: 0,
        ok: true,
    })
}

fn fetch_rank(score: u32) -> Option<RankCache> {
    let overall_deadline = Instant::now() + RANK_TOTAL_BUDGET;
    let user_agent = format!("User-Agent: OmaRank-Engine/{}", env!("CARGO_PKG_VERSION"));
    let proxies = crate::survey::proxy_envs();
    let envs: Vec<(&str, &str)> = proxies.iter().map(|(k, v)| (*k, v.as_str())).collect();

    for base in crate::survey::omastat_bases() {
        if overall_deadline.saturating_duration_since(Instant::now()) < Duration::from_millis(200) {
            break;
        }
        let url = format!("{}/api/rank/v1?score={}", base, score);
        let body = crate::subproc::run_cmd_bounded(
            "/usr/bin/curl",
            &["-s", "-f", "--max-time", "2", "-H", &user_agent, &url],
            &envs,
            overall_deadline,
            4096,
        );
        if let Some(rank) = body.and_then(|b| parse_rank_response(&b, score)) {
            return Some(rank);
        }
    }
    None
}

fn apply(result: &mut OmaRankResult, rank: &RankCache) {
    result.global_rank = rank.rank;
    result.total_machines = rank.total;
    result.rank_is_estimate = false;
    result.percentile_text = format!(
        "Top {:.1}% (#{} of {} OmaStat submissions)",
        rank.percentile, rank.rank, rank.total
    );
}

/// Replaces the offline estimate with the real OmaStat rank when the user has opted in.
/// `allow_network` = false only consults the cache.
pub fn apply_world_rank(result: &mut OmaRankResult, allow_network: bool) {
    if !crate::survey::load_survey_state().has_opted_in {
        return;
    }
    let path = rank_cache_path();
    let now = now_epoch();

    let cached = path
        .as_ref()
        .and_then(|p| crate::securefs::read_owned_file(p, 4096))
        .and_then(|c| serde_json::from_str::<RankCache>(&c).ok());
    if let Some(cache) = &cached {
        match cache_state(cache, result.total_score, now) {
            Some(true) => return apply(result, cache),
            Some(false) => return,
            None => {}
        }
    }

    if !allow_network {
        return;
    }

    let entry = match fetch_rank(result.total_score) {
        Some(mut rank) => {
            rank.fetched_epoch = now;
            apply(result, &rank);
            rank
        }
        None => RankCache {
            score: result.total_score,
            rank: 0,
            total: 0,
            percentile: 0.0,
            fetched_epoch: now,
            ok: false,
        },
    };
    if let (Some(p), Ok(json)) = (path, serde_json::to_string(&entry)) {
        crate::securefs::write_owned_file_atomic(&p, json.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rank_response() {
        let ok = parse_rank_response(
            br#"{"score":72,"rank":45,"total":366,"percentile":12.3}"#,
            72,
        )
        .expect("valid response");
        assert_eq!((ok.rank, ok.total), (45, 366));
        assert!((ok.percentile - 12.3).abs() < 1e-9);

        let derived = parse_rank_response(br#"{"score":72,"rank":1,"total":4}"#, 72).unwrap();
        assert!((derived.percentile - 25.0).abs() < 1e-9);

        assert!(parse_rank_response(br#"{"score":71,"rank":1,"total":4}"#, 72).is_none());
        assert!(parse_rank_response(br#"{"score":72,"rank":5,"total":4}"#, 72).is_none());
        assert!(parse_rank_response(br#"{"score":72,"rank":0,"total":4}"#, 72).is_none());
        assert!(
            parse_rank_response(br#"{"score":72,"rank":1,"total":4,"percentile":250}"#, 72)
                .is_none()
        );
        assert!(parse_rank_response(b"<html>", 72).is_none());
    }

    #[test]
    fn test_rank_cache_state() {
        let mut c = RankCache {
            score: 72,
            rank: 45,
            total: 366,
            percentile: 12.3,
            fetched_epoch: 1_000_000,
            ok: true,
        };
        assert_eq!(cache_state(&c, 72, 1_000_000 + 3600), Some(true));
        assert_eq!(cache_state(&c, 72, 1_000_000 + 24 * 3600), None, "expired");
        assert_eq!(cache_state(&c, 73, 1_000_001), None, "keyed by score");
        assert_eq!(cache_state(&c, 72, 999_999), None, "future timestamp");

        c.ok = false;
        assert_eq!(cache_state(&c, 72, 1_000_000 + 60), Some(false), "backoff");
        assert_eq!(
            cache_state(&c, 72, 1_000_000 + 15 * 60),
            None,
            "retry allowed"
        );
    }
}
