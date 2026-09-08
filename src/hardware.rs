use serde::{Deserialize, Serialize};
use std::fs;
use std::time::{Duration, Instant};

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
    pub ram_type: String,
    pub ram_speed_mts: u32,
    pub ram_modules: usize,
    pub ram_slots: usize,
    pub mobo_vendor: String,
    pub mobo_name: String,
    pub mobo_bios: String,
    pub chipset: String,
    pub gpu_name: String,
    pub gpu_driver: String,
    pub monitors: Vec<MonitorInfo>,
    pub storage_type: String,
    pub storage_model: String,
    pub os_name: String,
}

pub fn detect_hardware() -> HardwareInfo {
    let deadline = Instant::now() + Duration::from_millis(3500);
    let (cpu_name, cpu_cores, cpu_threads, cpu_mhz) = detect_cpu();
    let (ram_total_mb, ram_total_gb) = detect_ram();
    let (ram_type, ram_speed_mts, ram_modules, ram_slots) = detect_ram_details(deadline);
    let (mobo_vendor, mobo_name, mobo_bios, chipset) = detect_motherboard(deadline);
    let (gpu_name, gpu_driver) = detect_gpu(deadline);
    let monitors = detect_monitors(deadline);
    let (storage_type, storage_model) = detect_storage();
    let os_name = detect_os();

    HardwareInfo {
        cpu_name,
        cpu_cores,
        cpu_threads,
        cpu_mhz,
        ram_total_mb,
        ram_total_gb,
        ram_type,
        ram_speed_mts,
        ram_modules,
        ram_slots,
        mobo_vendor,
        mobo_name,
        mobo_bios,
        chipset,
        gpu_name,
        gpu_driver,
        monitors,
        storage_type,
        storage_model,
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

fn detect_gpu(deadline: Instant) -> (String, String) {
    // 1. If nvidia-smi is available, query exact discrete GPU model and driver
    if let Some(stdout_bytes) = crate::subproc::run_cmd_bounded(
        "/usr/bin/nvidia-smi",
        &["--query-gpu=gpu_name,driver_version", "--format=csv,noheader"],
        &[],
        deadline,
        4096,
    ) {
        let stdout = String::from_utf8_lossy(&stdout_bytes);
        if let Some(first_line) = stdout.lines().next() {
            let parts: Vec<&str> = first_line.split(',').collect();
            if !parts.is_empty() {
                let name = parts[0].trim();
                let drv = if parts.len() > 1 {
                    format!("nvidia {}", parts[1].trim())
                } else {
                    "nvidia".to_string()
                };
                if !name.is_empty() {
                    return (clean_string(name, 55), clean_string(&drv, 30));
                }
            }
        }
    }

    // 2. Scan all display controllers via lspci
    struct GpuCandidate {
        name: String,
        driver: String,
        is_discrete: bool,
    }
    let mut candidates: Vec<GpuCandidate> = Vec::new();

    if let Some(stdout_bytes) = crate::subproc::run_cmd_bounded(
        "/usr/bin/lspci",
        &["-mm"],
        &[],
        deadline,
        65536,
    ) {
        let stdout = String::from_utf8_lossy(&stdout_bytes);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('"').collect();
            if parts.len() >= 4 {
                let class = parts[1].trim();
                    if class.contains("VGA") || class.contains("3D") || class.contains("Display") {
                        let slot_raw = parts[0].trim();
                        let slot = if slot_raw.contains(':') { slot_raw } else { "" };
                        let vendor = parts[3].trim();
                        let device = if parts.len() >= 6 { parts[5].trim() } else { "" };
                        let full_name = format!("{} {}", vendor, device).trim().to_string();

                        // Detect driver from sysfs for this PCI slot
                        let mut dev_driver = String::from("drm");
                        let sysfs_uevent = format!("/sys/bus/pci/devices/0000:{}/uevent", slot);
                        let alt_uevent = format!("/sys/bus/pci/devices/{}/uevent", slot);
                        let uevent_content = fs::read_to_string(&sysfs_uevent).or_else(|_| fs::read_to_string(&alt_uevent));
                        if let Ok(content) = uevent_content {
                            for uline in content.lines() {
                                if uline.starts_with("DRIVER=") {
                                    dev_driver = uline.trim_start_matches("DRIVER=").trim().to_string();
                                    break;
                                }
                            }
                        }

                        let is_discrete = vendor.contains("NVIDIA")
                            || (vendor.contains("Advanced Micro") && (device.contains("RX ") || device.contains("Radeon Pro")))
                            || (vendor.contains("Intel") && (device.contains("Arc A") || device.contains("Arc B") || device.contains("Battlemage")));

                        candidates.push(GpuCandidate {
                            name: full_name,
                            driver: dev_driver,
                            is_discrete,
                        });
                    }
                }
            }
        }

    if let Some(discrete) = candidates.iter().find(|c| c.is_discrete) {
        return (clean_string(&discrete.name, 55), clean_string(&discrete.driver, 30));
    }

    if let Some(first) = candidates.first() {
        return (clean_string(&first.name, 55), clean_string(&first.driver, 30));
    }

    // Fallback: DRM sysfs query
    let mut driver = String::from("drm");
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

    (String::from("Generic GPU"), driver)
}

fn detect_monitors(deadline: Instant) -> Vec<MonitorInfo> {
    let mut list = Vec::new();

    let mut extra_envs = Vec::new();
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_default();
    if !runtime_dir.is_empty() {
        extra_envs.push(("XDG_RUNTIME_DIR", runtime_dir.as_str()));
    }
    let hypr_sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap_or_default();
    if !hypr_sig.is_empty() {
        extra_envs.push(("HYPRLAND_INSTANCE_SIGNATURE", hypr_sig.as_str()));
    }

    // Query Hyprland monitors via IPC
    if let Some(stdout_bytes) = crate::subproc::run_cmd_bounded(
        "/usr/bin/hyprctl",
        &["-j", "monitors"],
        &extra_envs,
        deadline,
        65536,
    ) {
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&stdout_bytes) {
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

fn detect_motherboard(deadline: Instant) -> (String, String, String, String) {
    let mut vendor = fs::read_to_string("/sys/class/dmi/id/board_vendor")
        .or_else(|_| fs::read_to_string("/sys/class/dmi/id/sys_vendor"))
        .unwrap_or_else(|_| "Generic".into());
    vendor = vendor.replace(" Technology Co., Ltd.", "").replace(" Inc.", "").trim().to_string();

    let mut name = fs::read_to_string("/sys/class/dmi/id/board_name")
        .or_else(|_| fs::read_to_string("/sys/class/dmi/id/product_name"))
        .unwrap_or_else(|_| "Motherboard".into());
    name = name.trim().to_string();

    let bios = fs::read_to_string("/sys/class/dmi/id/bios_version")
        .unwrap_or_else(|_| "Unknown".into());

    let mut chipset = String::from("Mainstream Chipset");
    if let Some(stdout_bytes) = crate::subproc::run_cmd_bounded(
        "/usr/bin/lspci",
        &[],
        &[],
        deadline,
        65536,
    ) {
        let stdout = String::from_utf8_lossy(&stdout_bytes);
        for line in stdout.lines() {
            if line.contains("ISA bridge:") || line.contains("Host bridge:") || line.contains("SMBus:") {
                if let Some(idx) = line.find("Intel Corporation ") {
                    let rest = &line[idx + 18..];
                    if let Some(c_idx) = rest.find(" Chipset") {
                        chipset = format!("Intel {}", &rest[..c_idx]);
                        break;
                    }
                } else if line.contains("AMD") {
                    for pat in ["X870E", "X870", "X670E", "X670", "B850", "B650E", "B650", "A620", "X570", "B550", "B450"] {
                        if line.contains(pat) {
                            chipset = format!("AMD {}", pat);
                            break;
                        }
                    }
                    if chipset.starts_with("AMD") {
                        break;
                    }
                }
            }
        }
    }

    if chipset == "Mainstream Chipset" {
        for pat in ["Z890", "Z790", "B760", "H770", "H610", "X870E", "X870", "X670E", "X670", "B850", "B650", "A620", "Z690", "B660"] {
            if name.contains(pat) {
                if pat.starts_with('Z') || pat.starts_with('B') || pat.starts_with('H') {
                    chipset = format!("Intel {}", pat);
                } else {
                    chipset = format!("AMD {}", pat);
                }
                break;
            }
        }
    }

    (
        clean_string(&vendor, 25),
        clean_string(&name, 35),
        clean_string(bios.trim(), 15),
        clean_string(&chipset, 25),
    )
}

fn detect_ram_details(deadline: Instant) -> (String, u32, usize, usize) {
    let mut ram_type = String::from("DDR4");
    let mut speed_mts: u32 = 3200;
    let mut modules: usize = 0;
    let mut slots: usize = 4;

    if let Some(stdout_bytes) = crate::subproc::run_cmd_bounded(
        "/usr/bin/inxi",
        &["--tty", "-m", "--output", "json", "--output-file", "print"],
        &[],
        deadline,
        65536,
    ) {
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&stdout_bytes) {
                if let Some(arr) = val.as_array() {
                    for section in arr {
                        if let Some(obj) = section.as_object() {
                            for (k, v) in obj {
                                if k.contains("Memory") {
                                    if let Some(mem_items) = v.as_array() {
                                        for item in mem_items {
                                            if let Some(iobj) = item.as_object() {
                                                for (ik, iv) in iobj {
                                                    if ik.contains("type") {
                                                        if let Some(s) = iv.as_str() {
                                                            if s.starts_with("DDR") || s.starts_with("LPDDR") {
                                                                ram_type = s.to_string();
                                                            }
                                                        }
                                                    }
                                                    if ik.contains("speed") {
                                                        if let Some(s) = iv.as_str() {
                                                            if let Some(mts) = s.split_whitespace().next() {
                                                                if let Ok(spd) = mts.parse::<u32>() {
                                                                    speed_mts = spd;
                                                                }
                                                            }
                                                        }
                                                    }
                                                    if ik.contains("Device") {
                                                        modules += 1;
                                                    }
                                                    if ik.contains("slots") {
                                                        if let Some(s) = iv.as_u64() {
                                                            slots = s as usize;
                                                        } else if let Some(s) = iv.as_str() {
                                                            if let Ok(sl) = s.parse::<usize>() {
                                                                slots = sl;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

    if modules == 0 {
        modules = 2;
    }
    (ram_type, speed_mts, modules, slots)
}

fn detect_storage() -> (String, String) {
    let mut storage_type = String::from("SATA SSD / HDD");
    let mut storage_model = String::from("Solid State Drive");

    if let Ok(entries) = fs::read_dir("/sys/block") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("nvme") && name.ends_with("n1") {
                storage_type = "NVMe PCIe Gen4/5 SSD".to_string();
                let model_path = format!("/sys/block/{}/device/model", name);
                if let Ok(m) = fs::read_to_string(&model_path) {
                    storage_model = clean_string(m.trim(), 40);
                    break;
                }
            } else if (name.starts_with("sd") || name.starts_with("vd")) && storage_model == "Solid State Drive" {
                let model_path = format!("/sys/block/{}/device/model", name);
                if let Ok(m) = fs::read_to_string(&model_path) {
                    storage_model = clean_string(m.trim(), 40);
                    storage_type = "SATA SSD".to_string();
                }
            }
        }
    }

    (storage_type, storage_model)
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
