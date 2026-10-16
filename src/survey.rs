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
    }
}

/// POSTs the payload with curl and reports whether OmaStat actually accepted it.
/// curl exits 0 for HTTP 4xx/5xx too, so the response body's `success` flag is the source of truth.
fn post_survey(endpoint: &str, json_data: &str, max_time_secs: u64) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(max_time_secs + 1);
    let max_time = max_time_secs.to_string();
    let output = crate::subproc::run_cmd_bounded(
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
            "User-Agent: OmaRank-Engine/1.0",
            "-d",
            json_data,
            endpoint,
        ],
        &[],
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

    let official_endpoint = "https://omastat.omarchy.org/api/survey/v1";
    let community_endpoint = "https://omastat.ozan-zdil.workers.dev/api/survey/v1";

    // 1. Try official endpoint first, 2. fall back to the live community deployment
    let outcome = post_survey(official_endpoint, &json_data, 2)
        .or_else(|_| post_survey(community_endpoint, &json_data, 3));

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
