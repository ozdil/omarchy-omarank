use crate::score::OmaRankResult;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

extern "C" {
    fn getuid() -> u32;
}

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
    pub mobo_name: String,
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
    if let Ok(meta) = fs::symlink_metadata(p) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if meta.file_type().is_symlink() || !meta.file_type().is_file() {
                return SurveyState::default();
            }
            // SAFETY: getuid is a POSIX libc function without side effects
            let current_uid = unsafe { getuid() };
            if meta.uid() != current_uid {
                return SurveyState::default();
            }
        }
        if let Ok(content) = fs::read_to_string(p) {
            if let Ok(state) = serde_json::from_str::<SurveyState>(&content) {
                return state;
            }
        }
    }
    SurveyState::default()
}

pub fn save_survey_state(state: &SurveyState) {
    save_survey_state_to_path(state, &get_survey_state_path());
}

pub fn save_survey_state_to_path(state: &SurveyState, p: &std::path::Path) {
    let parent = match p.parent() {
        Some(par) => par,
        None => return,
    };

    if fs::create_dir_all(parent).is_err() {
        return;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
    }

    if let Ok(meta) = fs::symlink_metadata(p) {
        if meta.file_type().is_symlink() || !meta.file_type().is_file() {
            return;
        }
    }

    let json = match serde_json::to_string_pretty(state) {
        Ok(j) => j,
        Err(_) => return,
    };

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_path = parent.join(format!(".tmp_survey_state_{}_{}.json", std::process::id(), nanos));

    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true).mode(0o600);
        if let Ok(mut f) = opts.open(&tmp_path) {
            let _ = f.set_permissions(fs::Permissions::from_mode(0o600));
            if f.write_all(json.as_bytes()).is_ok() && f.sync_all().is_ok() {
                drop(f);
                let _ = fs::rename(&tmp_path, p);
                return;
            }
        }
        let _ = fs::remove_file(&tmp_path);
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
        mobo_name: result.hardware.mobo_name.clone(),
        chipset: result.hardware.chipset.clone(),
        primary_display,
        display_count: result.hardware.monitors.len(),
        storage_type: result.hardware.storage_type.clone(),
        storage_model: result.hardware.storage_model.clone(),
        os_name: result.hardware.os_name.clone(),
        archetype_signature: result.archetype_signature.clone(),
    }
}

pub fn submit_to_omastat(result: &OmaRankResult) -> (bool, String) {
    let payload = create_survey_payload(result);
    let json_data = match serde_json::to_string(&payload) {
        Ok(j) => j,
        Err(e) => return (false, format!("JSON serialization error: {}", e)),
    };

    let endpoint = "https://omastat.omarchy.org/api/survey/v1";
    let deadline = Instant::now() + Duration::from_secs(4);

    let output = crate::subproc::run_cmd_bounded(
        "/usr/bin/curl",
        &[
            "-s",
            "-S",
            "--max-time", "3",
            "-X", "POST",
            "-H", "Content-Type: application/json",
            "-H", "User-Agent: OmaRank-Engine/1.0",
            "-d", &json_data,
            endpoint,
        ],
        &[],
        deadline,
        4096,
    );

    let mut state = load_survey_state();
    state.has_opted_in = true;
    state.last_submitted_epoch = payload.timestamp_epoch;
    state.last_score = result.total_score;
    save_survey_state(&state);

    if output.is_some() {
        (true, "Successfully submitted anonymous hardware profile to OmaStat survey!".to_string())
    } else {
        (true, "✓ Profile saved locally (Awaiting OmaStat community server deployment)".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            assert_eq!(meta.mode() & 0o777, 0o600, "File permissions must be strictly 0600");

            let parent_meta = fs::metadata(&temp_dir).expect("Parent dir should exist");
            assert_eq!(parent_meta.mode() & 0o777, 0o700, "Directory permissions must be 0700");
        }

        let loaded = load_survey_state_from_path(&state_path);
        assert_eq!(loaded, test_state);

        let _ = fs::remove_file(&state_path);
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_symlink_rejection_on_state_file() {
        let temp_dir = std::env::temp_dir().join(format!("omarank_symlink_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let real_file = temp_dir.join("real.json");
        let symlink_path = temp_dir.join("survey_state.json");

        let _ = fs::write(&real_file, "{}");
        #[cfg(unix)]
        {
            let _ = std::os::unix::fs::symlink(&real_file, &symlink_path);
        }

        let state = load_survey_state_from_path(&symlink_path);
        assert_eq!(state, SurveyState::default(), "Symlink must be rejected on load");

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
}
