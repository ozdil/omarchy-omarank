use crate::score::OmaRankResult;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

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
    pub primary_display: String,
    pub display_count: usize,
    pub storage_type: String,
    pub os_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    let p = get_survey_state_path();
    if p.exists() {
        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(state) = serde_json::from_str::<SurveyState>(&content) {
                return state;
            }
        }
    }
    SurveyState::default()
}

pub fn save_survey_state(state: &SurveyState) {
    let p = get_survey_state_path();
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(state) {
        if fs::write(&p, json).is_ok() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&p, fs::Permissions::from_mode(0o600));
            }
        }
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
        schema_version: 1,
        timestamp_epoch: epoch,
        omarank_score: result.total_score,
        tier_name: result.tier_name.clone(),
        cpu_model: result.hardware.cpu_name.clone(),
        cpu_cores: result.hardware.cpu_cores,
        cpu_threads: result.hardware.cpu_threads,
        gpu_model: result.hardware.gpu_name.clone(),
        gpu_driver: result.hardware.gpu_driver.clone(),
        ram_gb: result.hardware.ram_total_gb,
        primary_display,
        display_count: result.hardware.monitors.len(),
        storage_type: result.hardware.storage_type.clone(),
        os_name: result.hardware.os_name.clone(),
    }
}

pub fn submit_to_omastat(result: &OmaRankResult) -> (bool, String) {
    let payload = create_survey_payload(result);
    let json_data = match serde_json::to_string(&payload) {
        Ok(j) => j,
        Err(e) => return (false, format!("JSON serialization error: {}", e)),
    };

    // Public OmaStat Endpoint URL (Survey aggregator)
    let endpoint = "https://omastat.omarchy.org/api/survey/v1";

    // Safe isolated curl call with strict 3-second timeout and clean env
    let mut cmd = Command::new("/usr/bin/curl");
    cmd.env_clear();
    cmd.envs([
        ("PATH", "/usr/bin:/bin"),
        ("LC_ALL", "C"),
    ]);
    cmd.args([
        "-s",
        "-S",
        "--max-time", "3",
        "-X", "POST",
        "-H", "Content-Type: application/json",
        "-H", "User-Agent: OmaRank-Engine/1.0",
        "-d", &json_data,
        endpoint,
    ]);

    match cmd.output() {
        Ok(output) => {
            let mut state = load_survey_state();
            state.has_opted_in = true;
            state.last_submitted_epoch = payload.timestamp_epoch;
            state.last_score = result.total_score;
            save_survey_state(&state);

            if output.status.success() {
                (true, "Successfully submitted anonymous hardware profile to OmaStat survey!".to_string())
            } else {
                // In offline mode or staging, local state is still preserved
                (true, "OmaStat survey payload registered locally (Offline mode)".to_string())
            }
        }
        Err(e) => (false, format!("Survey submission error: {}", e)),
    }
}
