use crate::score::OmaRankResult;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmaStatPayload {
    pub schema_version: u32,
    pub timestamp_epoch: u64,
    pub omarank_score: u32,
    pub tier_name: String,
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub cpu_threads: usize,
    pub gpu_model: String,
    pub gpu_driver: String,
    pub ram_gb: f64,
    pub ram_type: String,
    pub ram_speed_mts: u32,
    pub mobo_vendor: String,
    pub mobo_name: String,
    pub system_vendor: String,
    pub product_name: String,
    pub chipset: String,
    pub primary_display: String,
    pub display_count: usize,
    pub storage_type: String,
    pub storage_model: String,
    pub os_name: String,
    pub archetype_signature: String,
    pub install_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SurveyState {
    pub has_opted_in: bool,
    pub last_submitted_epoch: u64,
    pub last_score: u32,
}

pub fn get_survey_state_path() -> PathBuf {
    if let Ok(state_home) = std::env::var("XDG_STATE_HOME") {
        if !state_home.is_empty() {
            return PathBuf::from(state_home).join("omarank/survey_state.json");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home).join(".local/state/omarank/survey_state.json");
        }
    }
    PathBuf::from("/var/empty/omarank_state.json")
}

pub fn load_survey_state() -> SurveyState {
    load_survey_state_from_path(&get_survey_state_path())
}

pub fn load_survey_state_from_path(p: &std::path::Path) -> SurveyState {
    crate::securefs::read_owned_file(p, 65536)
        .and_then(|content| serde_json::from_str::<SurveyState>(&content).ok())
        .unwrap_or_default()
}

pub fn save_survey_state(state: &SurveyState) {
    save_survey_state_to_path(state, &get_survey_state_path());
}

pub fn save_survey_state_to_path(state: &SurveyState, p: &std::path::Path) {
    if let Ok(json) = serde_json::to_string_pretty(state) {
        crate::securefs::write_owned_file_atomic(p, json.as_bytes());
    }
}

/// Converts days since 1970-01-01 to a (year, month) civil date (Howard Hinnant's
/// days_from_civil inverse), so no date crate is needed for the token's UTC month.
fn year_month_from_epoch(epoch_secs: u64) -> (i64, u32) {
    let z = (epoch_secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11], March-based
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    (year, month)
}

/// Anonymous, monthly rotating install token: the first 32 hex chars of
/// sha256("omarank-survey-v1:" + machine-id + ":" + "YYYY-MM" in UTC).
/// It lets OmaStat count one submission per machine per month; the hash cannot be reversed
/// to the machine-id and changes every month, so submissions cannot be linked over time.
pub fn compute_install_token(machine_id: &str, epoch_secs: u64) -> String {
    use sha2::{Digest, Sha256};

    let machine_id = machine_id.trim();
    if machine_id.is_empty() {
        return String::new();
    }
    let (year, month) = year_month_from_epoch(epoch_secs);
    let input = format!("omarank-survey-v1:{}:{:04}-{:02}", machine_id, year, month);
    let digest = Sha256::digest(input.as_bytes());
    digest
        .iter()
        .take(16)
        .map(|b| format!("{:02x}", b))
        .collect()
}

fn read_install_token(epoch_secs: u64) -> String {
    // Unreadable machine-id: send an empty token rather than a guessable substitute
    std::fs::read_to_string("/etc/machine-id")
        .map(|id| compute_install_token(&id, epoch_secs))
        .unwrap_or_default()
}

pub fn create_survey_payload(result: &OmaRankResult) -> OmaStatPayload {
    let epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let primary_display = if let Some(m) = result.hardware.monitors.first() {
        format!("{}x{} @ {:.0}Hz", m.width, m.height, m.refresh_rate)
    } else {
        "1920x1080 @ 60Hz".to_string()
    };

    OmaStatPayload {
        schema_version: 2,
        timestamp_epoch: epoch,
        omarank_score: result.total_score,
        tier_name: result.tier_name.clone(),
        cpu_model: result.hardware.cpu_name.clone(),
        cpu_cores: result.hardware.cpu_cores,
        cpu_threads: result.hardware.cpu_threads,
        gpu_model: result.hardware.gpu_name.clone(),
        gpu_driver: result.hardware.gpu_driver.clone(),
        ram_gb: result.hardware.ram_total_gb,
        ram_type: result.hardware.ram_type.clone(),
        ram_speed_mts: result.hardware.ram_speed_mts,
        mobo_vendor: result.hardware.mobo_vendor.clone(),
        mobo_name: result.hardware.mobo_name.clone(),
        system_vendor: result.hardware.system_vendor.clone(),
        product_name: result.hardware.product_name.clone(),
        chipset: result.hardware.chipset.clone(),
        primary_display,
        display_count: result.hardware.monitors.len(),
        storage_type: result.hardware.storage_type.clone(),
        storage_model: result.hardware.storage_model.clone(),
        os_name: result.hardware.os_name.clone(),
        archetype_signature: result.archetype_signature.clone(),
        install_token: read_install_token(epoch),
    }
}

/// Live community deployment first; the official domain is not routed yet.
const DEFAULT_OMASTAT_BASES: [&str; 2] = [
    "https://omastat.ozan-zdil.workers.dev",
    "https://omastat.omarchy.org",
];

/// Validates an `OMASTAT_URL` override: HTTPS anywhere, plain HTTP only to the loopback host.
fn parse_omastat_override(raw: &str) -> Option<String> {
    let url = raw.trim().trim_end_matches('/');
    if url.is_empty() || url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }
    let local_http = ["http://127.0.0.1", "http://localhost"].iter().any(|host| {
        url.strip_prefix(host)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(':') || rest.starts_with('/'))
    });
    let https = url
        .strip_prefix("https://")
        .is_some_and(|rest| !rest.is_empty());
    if https || local_http {
        Some(url.to_string())
    } else {
        None
    }
}

/// OmaStat base URLs in the order they should be tried. `OMASTAT_URL` (when set and valid)
/// is the only endpoint; an invalid override yields no endpoint rather than a silent fallback.
pub fn omastat_bases() -> Vec<String> {
    match std::env::var("OMASTAT_URL") {
        Ok(raw) if !raw.trim().is_empty() => parse_omastat_override(&raw).into_iter().collect(),
        _ => DEFAULT_OMASTAT_BASES
            .iter()
            .map(|s| s.to_string())
            .collect(),
    }
}

/// Proxy settings are the only part of the engine's environment handed to curl.
pub fn proxy_envs() -> Vec<(&'static str, String)> {
    [
        "HTTPS_PROXY",
        "HTTP_PROXY",
        "NO_PROXY",
        "https_proxy",
        "http_proxy",
        "no_proxy",
    ]
    .iter()
    .filter_map(|k| {
        std::env::var(k)
            .ok()
            .filter(|v| !v.is_empty())
            .map(|v| (*k, v))
    })
    .collect()
}

/// POSTs the payload with curl and reports whether OmaStat actually accepted it.
/// The body goes through stdin (`--data-binary @-`) so it never shows up in `ps`.
/// curl exits 0 for HTTP 4xx/5xx too, so the response body's `success` flag is the source of truth.
fn post_survey(endpoint: &str, json_data: &str, max_time_secs: u64) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(max_time_secs + 1);
    let max_time = max_time_secs.to_string();
    let user_agent = format!("User-Agent: OmaRank-Engine/{}", env!("CARGO_PKG_VERSION"));
    let proxies = proxy_envs();
    let envs: Vec<(&str, &str)> = proxies.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let output = crate::subproc::run_cmd_bounded_with_stdin(
        "/usr/bin/curl",
        &[
            "-s",
            "-S",
            "--max-time",
            &max_time,
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
            "-H",
            &user_agent,
            "--data-binary",
            "@-",
            endpoint,
        ],
        &envs,
        Some(json_data.as_bytes()),
        deadline,
        4096,
    )
    .ok_or_else(|| "network error".to_string())?;

    let resp: serde_json::Value =
        serde_json::from_slice(&output).map_err(|_| "unexpected server response".to_string())?;
    if resp.get("success").and_then(|v| v.as_bool()) == Some(true) {
        Ok(())
    } else {
        let reason = resp
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("rejected by server");
        Err(crate::hardware::clean_string(reason, 80))
    }
}

pub fn submit_to_omastat(result: &OmaRankResult) -> (bool, String) {
    let payload = create_survey_payload(result);
    let json_data = match serde_json::to_string(&payload) {
        Ok(j) => j,
        Err(e) => return (false, format!("JSON serialization error: {}", e)),
    };

    let bases = omastat_bases();
    if bases.is_empty() {
        return (
            false,
            "Survey not submitted (OMASTAT_URL must be https:// or http://localhost).".to_string(),
        );
    }

    // Try each endpoint in order; the first one gets a little more time
    let mut outcome = Err("no endpoint".to_string());
    for (i, base) in bases.iter().enumerate() {
        let endpoint = format!("{}/api/survey/v1", base);
        outcome = post_survey(&endpoint, &json_data, if i == 0 { 4 } else { 2 });
        if outcome.is_ok() {
            break;
        }
    }

    match outcome {
        Ok(()) => {
            let mut state = load_survey_state();
            state.has_opted_in = true;
            state.last_submitted_epoch = payload.timestamp_epoch;
            state.last_score = result.total_score;
            save_survey_state(&state);
            (
                true,
                "Successfully submitted anonymous hardware profile to OmaStat survey!".to_string(),
            )
        }
        Err(reason) => (
            false,
            format!("Survey not submitted ({}). Please try again later.", reason),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_year_month_from_epoch() {
        assert_eq!(year_month_from_epoch(0), (1970, 1));
        assert_eq!(year_month_from_epoch(951_782_400), (2000, 2)); // 2000-02-29 00:00 UTC
        assert_eq!(year_month_from_epoch(1_709_251_199), (2024, 2)); // 2024-02-29 23:59:59
        assert_eq!(year_month_from_epoch(1_709_251_200), (2024, 3)); // 2024-03-01 00:00:00
        assert_eq!(year_month_from_epoch(1_767_225_599), (2025, 12)); // 2025-12-31 23:59:59
        assert_eq!(year_month_from_epoch(1_767_225_600), (2026, 1)); // 2026-01-01 00:00:00
    }

    #[test]
    fn test_install_token() {
        let id = "0123456789abcdef0123456789abcdef\n";
        // 2026-10-16 UTC: sha256("omarank-survey-v1:0123456789abcdef0123456789abcdef:2026-10")
        let token = compute_install_token(id, 1_792_180_000);
        assert_eq!(token, "b2040ad108eccdca7ad562875a06e92a");
        assert_eq!(token.len(), 32);
        // Same month, same token; next month, a different one
        assert_eq!(compute_install_token(id, 1_792_180_000 + 86_400), token);
        assert_ne!(
            compute_install_token(id, 1_792_180_000 + 31 * 86_400),
            token
        );
        assert_eq!(compute_install_token("  \n", 1_792_180_000), "");
    }

    #[test]
    fn test_omastat_override_validation() {
        assert_eq!(
            parse_omastat_override("https://stats.example.org/"),
            Some("https://stats.example.org".to_string())
        );
        assert_eq!(
            parse_omastat_override("http://127.0.0.1:8787"),
            Some("http://127.0.0.1:8787".to_string())
        );
        assert_eq!(
            parse_omastat_override("http://localhost"),
            Some("http://localhost".to_string())
        );
        assert_eq!(parse_omastat_override("http://example.org"), None);
        assert_eq!(parse_omastat_override("http://localhost.evil.org"), None);
        assert_eq!(parse_omastat_override("http://127.0.0.1.evil.org"), None);
        assert_eq!(parse_omastat_override("https://"), None);
        assert_eq!(parse_omastat_override("https://a b"), None);
        assert_eq!(parse_omastat_override("file:///etc/passwd"), None);
    }

    #[test]
    fn test_secure_survey_state_creation_and_permissions() {
        let temp_dir = std::env::temp_dir().join(format!("omarank_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let state_path = temp_dir.join("survey_state.json");

        let test_state = SurveyState {
            has_opted_in: true,
            last_submitted_epoch: 123456789,
            last_score: 88,
        };

        save_survey_state_to_path(&test_state, &state_path);

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let meta = fs::metadata(&state_path).expect("State file should exist");
            assert_eq!(
                meta.mode() & 0o777,
                0o600,
                "File permissions must be strictly 0600"
            );

            let parent_meta = fs::metadata(&temp_dir).expect("Parent dir should exist");
            assert_eq!(
                parent_meta.mode() & 0o777,
                0o700,
                "Directory permissions must be 0700"
            );
        }

        let loaded = load_survey_state_from_path(&state_path);
        assert_eq!(loaded, test_state);

        let _ = fs::remove_file(&state_path);
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_symlink_rejection_on_state_file() {
        let temp_dir =
            std::env::temp_dir().join(format!("omarank_symlink_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let real_file = temp_dir.join("real.json");
        let symlink_path = temp_dir.join("survey_state.json");

        let _ = fs::write(&real_file, "{}");
        #[cfg(unix)]
        {
            let _ = std::os::unix::fs::symlink(&real_file, &symlink_path);
        }

        let state = load_survey_state_from_path(&symlink_path);
        assert_eq!(
            state,
            SurveyState::default(),
            "Symlink must be rejected on load"
        );

        let test_state = SurveyState {
            has_opted_in: true,
            last_submitted_epoch: 999,
            last_score: 99,
        };
        save_survey_state_to_path(&test_state, &symlink_path);

        let real_content = fs::read_to_string(&real_file).unwrap_or_default();
        assert_eq!(real_content, "{}", "Symlink target must not be overwritten");

        let _ = fs::remove_file(&symlink_path);
        let _ = fs::remove_file(&real_file);
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_symlink_rejection_on_parent_dir() {
        let root_temp =
            std::env::temp_dir().join(format!("omarank_parent_sym_root_{}", std::process::id()));
        let _ = fs::create_dir_all(&root_temp);
        let real_parent = root_temp.join("real_dir");
        let symlink_parent = root_temp.join("symlink_dir");
        let _ = fs::create_dir_all(&real_parent);

        #[cfg(unix)]
        {
            let _ = std::os::unix::fs::symlink(&real_parent, &symlink_parent);
        }

        let target_in_symlink_parent = symlink_parent.join("survey_state.json");
        let test_state = SurveyState {
            has_opted_in: true,
            last_submitted_epoch: 111,
            last_score: 77,
        };

        save_survey_state_to_path(&test_state, &target_in_symlink_parent);

        // Target file must not have been created through the symlinked parent
        let real_target = real_parent.join("survey_state.json");
        assert!(
            !real_target.exists(),
            "Write through symlinked parent directory must be rejected"
        );

        let _ = fs::remove_file(&symlink_parent);
        let _ = fs::remove_dir_all(&real_parent);
        let _ = fs::remove_dir_all(&root_temp);
    }

    #[test]
    fn test_preplanted_tmp_symlink_rejection() {
        let temp_dir =
            std::env::temp_dir().join(format!("omarank_preplant_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let victim_file = temp_dir.join("victim.txt");
        let _ = fs::write(&victim_file, "PRESERVED_CONTENT");

        let state_path = temp_dir.join("survey_state.json");

        // Plant a symlink matching a staging pattern in the directory
        let planted_symlink = temp_dir.join(format!(
            ".tmp_survey_state_{}_planted.json",
            std::process::id()
        ));
        #[cfg(unix)]
        {
            let _ = std::os::unix::fs::symlink(&victim_file, &planted_symlink);
        }

        let test_state = SurveyState {
            has_opted_in: true,
            last_submitted_epoch: 222,
            last_score: 88,
        };

        save_survey_state_to_path(&test_state, &state_path);

        // Check victim file was not touched
        let victim_content = fs::read_to_string(&victim_file).unwrap_or_default();
        assert_eq!(
            victim_content, "PRESERVED_CONTENT",
            "Victim file pointed by symlink must remain untouched"
        );

        // The legitimate state file should still be saved safely
        let loaded = load_survey_state_from_path(&state_path);
        assert_eq!(loaded, test_state);

        let _ = fs::remove_file(&state_path);
        let _ = fs::remove_file(&planted_symlink);
        let _ = fs::remove_file(&victim_file);
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
