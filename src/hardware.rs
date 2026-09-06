use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub refresh_rate: f64,
    pub scale: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub cpu_name: String,
    pub cpu_cores: usize,
    pub cpu_threads: usize,
    pub cpu_mhz: f64,
    pub ram_total_mb: u64,
    pub ram_total_gb: f64,
    pub gpu_name: String,
    pub gpu_driver: String,
    pub monitors: Vec<MonitorInfo>,
    pub storage_type: String,
    pub os_name: String,
}

pub fn detect_hardware() -> HardwareInfo {
    let (cpu_name, cpu_cores, cpu_threads, cpu_mhz) = detect_cpu();
    let (ram_total_mb, ram_total_gb) = detect_ram();
    let (gpu_name, gpu_driver) = detect_gpu();
    let monitors = detect_monitors();
    let storage_type = detect_storage();
    let os_name = detect_os();

    HardwareInfo {
        cpu_name,
        cpu_cores,
        cpu_threads,
        cpu_mhz,
        ram_total_mb,
        ram_total_gb,
        gpu_name,
        gpu_driver,
        monitors,
        storage_type,
        os_name,
    }
}

fn detect_cpu() -> (String, usize, usize, f64) {
    let mut name = String::from("Generic CPU");
    let mut threads = 0;
    let mut cores = 0;
    let mut max_mhz = 0.0;

    if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        for line in content.lines() {
            let mut parts = line.splitn(2, ':');
            if let (Some(key), Some(val)) = (parts.next(), parts.next()) {
                let key = key.trim();
                let val = val.trim();
                if key == "model name" && name == "Generic CPU" {
                    name = clean_string(val, 50);
                } else if key == "processor" {
                    threads += 1;
                } else if key == "cpu cores" && cores == 0 {
                    cores = val.parse::<usize>().unwrap_or(0);
                } else if key == "cpu MHz" {
                    if let Ok(mhz) = val.parse::<f64>() {
                        if mhz > max_mhz {
                            max_mhz = mhz;
                        }
                    }
                }
            }
        }
    }

    if cores == 0 {
        cores = if threads > 0 { threads } else { 1 };
    }
    if threads == 0 {
        threads = cores;
    }

    (name, cores, threads, max_mhz)
}

fn detect_ram() -> (u64, f64) {
    let mut total_kb: u64 = 0;
    if let Ok(content) = fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    total_kb = parts[1].parse::<u64>().unwrap_or(0);
                    break;
                }
            }
        }
    }

    let total_mb = total_kb / 1024;
    let total_gb = (total_mb as f64) / 1024.0;
    (total_mb, (total_gb * 10.0).round() / 10.0)
}

fn detect_gpu() -> (String, String) {
    let mut gpu_name = String::from("Generic GPU");
    let mut driver = String::from("drm");

    // Try lspci first for human-readable GPU name
    if let Ok(output) = Command::new("/usr/bin/lspci")
        .args(["-mm", "-d", "::0300"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split('"').collect();
                if parts.len() >= 4 {
                    let vendor = parts[3].trim();
                    let device = if parts.len() >= 6 { parts[5].trim() } else { "" };
                    gpu_name = format!("{} {}", vendor, device).trim().to_string();
                    break;
                }
            }
        }
    }

    // Try uevent for kernel driver name (e.g. xe, i915, amdgpu, nvidia)
    for card_num in 0..=4 {
        let uevent_path = format!("/sys/class/drm/card{}/device/uevent", card_num);
        if let Ok(content) = fs::read_to_string(&uevent_path) {
            for line in content.lines() {
                if line.starts_with("DRIVER=") {
                    driver = line.trim_start_matches("DRIVER=").trim().to_string();
                    break;
                }
            }
            if driver != "drm" {
                break;
            }
        }
    }

    // Clean up name
    gpu_name = clean_string(&gpu_name, 55);
    (gpu_name, driver)
}

fn detect_monitors() -> Vec<MonitorInfo> {
    let mut list = Vec::new();

    // Query Hyprland monitors via IPC
    if let Ok(output) = Command::new("/usr/bin/hyprctl")
        .args(["-j", "monitors"])
        .output()
    {
        if output.status.success() {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                if let Some(arr) = val.as_array() {
                    for m in arr {
                        let name = m.get("name").and_then(|v| v.as_str()).unwrap_or("DP-1");
                        let width = m.get("width").and_then(|v| v.as_u64()).unwrap_or(1920) as u32;
                        let height = m.get("height").and_then(|v| v.as_u64()).unwrap_or(1080) as u32;
                        let refresh_rate = m.get("refreshRate").and_then(|v| v.as_f64()).unwrap_or(60.0);
                        let scale = m.get("scale").and_then(|v| v.as_f64()).unwrap_or(1.0);

                        list.push(MonitorInfo {
                            name: clean_string(name, 20),
                            width,
                            height,
                            refresh_rate: (refresh_rate * 10.0).round() / 10.0,
                            scale,
                        });
                    }
                }
            }
        }
    }

    // Fallback if hyprctl returned empty
    if list.is_empty() {
        list.push(MonitorInfo {
            name: "Display-1".to_string(),
            width: 1920,
            height: 1080,
            refresh_rate: 60.0,
            scale: 1.0,
        });
    }

    list
}

fn detect_storage() -> String {
    if let Ok(entries) = fs::read_dir("/sys/block") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("nvme") {
                return "NVMe PCIe Gen4/5 SSD".to_string();
            }
        }
    }
    "SATA SSD / HDD".to_string()
}

fn detect_os() -> String {
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("PRETTY_NAME=") {
                let val = line.trim_start_matches("PRETTY_NAME=").trim_matches('"');
                return clean_string(val, 30);
            }
        }
    }
    "Omarchy Linux".to_string()
}

pub fn clean_string(s: &str, max_len: usize) -> String {
    let sanitized: String = s
        .chars()
        .filter(|c| !c.is_control() && *c != '<' && *c != '>' && *c != '&' && *c != '"' && *c != '\'')
        .collect();
    let trimmed = sanitized.trim();
    if trimmed.len() > max_len {
        trimmed[..max_len].to_string()
    } else {
        trimmed.to_string()
    }
}
