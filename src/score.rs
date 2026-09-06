use crate::hardware::HardwareInfo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubScores {
    pub cpu: u32,
    pub gpu: u32,
    pub ram: u32,
    pub mobo: u32,
    pub display: u32,
    pub storage: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmaRankResult {
    pub total_score: u32,
    pub global_rank: u32,
    pub total_machines: u32,
    pub tier_name: String,
    pub tier_icon: String,
    pub tier_nerd_icon: String,
    pub tier_quote: String,
    pub tier_color: String,
    pub sub_scores: SubScores,
    pub percentile_text: String,
    pub hardware: HardwareInfo,
}

pub fn calculate_score(hw: &HardwareInfo) -> OmaRankResult {
    let cpu_score = score_cpu(hw.cpu_cores, hw.cpu_threads, hw.cpu_mhz);
    let ram_score = score_ram(hw.ram_total_gb, &hw.ram_type, hw.ram_speed_mts, hw.ram_modules);
    let mobo_score = score_mobo(&hw.chipset, &hw.mobo_name);
    let gpu_score = score_gpu(&hw.gpu_name, &hw.gpu_driver);
    let display_score = score_display(&hw.monitors);
    let storage_score = score_storage(&hw.storage_type, &hw.storage_model);

    // Weighted Total Score (0-100)
    // CPU: 24%, GPU: 32%, RAM: 16%, Mobo: 8%, Display: 15%, Storage: 5%
    let total = (cpu_score as f64 * 0.24)
        + (gpu_score as f64 * 0.32)
        + (ram_score as f64 * 0.16)
        + (mobo_score as f64 * 0.08)
        + (display_score as f64 * 0.15)
        + (storage_score as f64 * 0.05);

    let total_score = (total.round() as u32).min(100).max(1);

    let total_machines = 12480;
    let hw_seed = calculate_hw_seed(hw);
    let global_rank = calculate_global_rank(total_score, hw_seed, total_machines);

    let (tier_name, tier_icon, tier_nerd_icon, tier_quote, tier_color, _) = get_rank_tier(total_score);

    let pct = (global_rank as f64 / total_machines as f64) * 100.0;
    let percentile_text = format!("Top {:.1}% (#{} of {} machines globally)", pct, global_rank, total_machines);

    OmaRankResult {
        total_score,
        global_rank,
        total_machines,
        tier_name,
        tier_icon,
        tier_nerd_icon,
        tier_quote,
        tier_color,
        sub_scores: SubScores {
            cpu: cpu_score,
            gpu: gpu_score,
            ram: ram_score,
            mobo: mobo_score,
            display: display_score,
            storage: storage_score,
        },
        percentile_text,
        hardware: hw.clone(),
    }
}

fn score_cpu(cores: usize, threads: usize, mhz: f64) -> u32 {
    let mut s: f64 = 15.0;

    // Threads scale
    if threads >= 32 {
        s += 65.0;
    } else if threads >= 24 {
        s += 58.0;
    } else if threads >= 20 {
        s += 54.0;
    } else if threads >= 16 {
        s += 46.0;
    } else if threads >= 12 {
        s += 38.0;
    } else if threads >= 8 {
        s += 28.0;
    } else if threads >= 4 {
        s += 16.0;
    }

    // Cores bonus
    if cores >= 16 {
        s += 10.0;
    } else if cores >= 12 {
        s += 8.0;
    } else if cores >= 8 {
        s += 6.0;
    }

    // Clock frequency bonus
    if mhz >= 5200.0 {
        s += 12.0;
    } else if mhz >= 4600.0 {
        s += 8.0;
    } else if mhz >= 4000.0 {
        s += 5.0;
    } else if mhz < 2600.0 && mhz > 0.0 {
        s -= 8.0;
    }

    (s.round() as u32).min(100).max(5)
}

fn score_ram(gb: f64, ram_type: &str, speed_mts: u32, modules: usize) -> u32 {
    let mut s: f64 = 0.0;

    // Capacity baseline
    if gb >= 120.0 {
        s += 80.0;
    } else if gb >= 60.0 {
        s += 74.0;
    } else if gb >= 30.0 {
        s += 62.0;
    } else if gb >= 15.0 {
        s += 46.0;
    } else if gb >= 11.0 {
        s += 32.0;
    } else if gb >= 7.0 {
        s += 20.0;
    } else if gb >= 3.5 {
        s += 10.0;
    } else {
        s += 4.0;
    }

    // Generation bonus (DDR5 / DDR4 / DDR3)
    let t = ram_type.to_uppercase();
    if t.contains("DDR5") {
        s += 10.0;
    } else if t.contains("DDR4") {
        s += 6.0;
    } else if t.contains("DDR3") {
        s += 0.0;
    } else if t.contains("DDR2") {
        s -= 6.0;
    } else {
        s += 4.0;
    }

    // Speed bonus (MT/s)
    if speed_mts >= 6400 {
        s += 10.0;
    } else if speed_mts >= 5600 {
        s += 8.0;
    } else if speed_mts >= 4800 {
        s += 6.0;
    } else if speed_mts >= 3600 {
        s += 5.0;
    } else if speed_mts >= 3200 {
        s += 4.0;
    } else if speed_mts >= 2666 {
        s += 2.0;
    } else if speed_mts >= 2133 {
        s += 1.0;
    }

    // Multi-channel / populated slots bonus
    if modules >= 4 {
        s += 10.0;
    } else if modules >= 2 {
        s += 6.0;
    }

    (s.round() as u32).min(100).max(5)
}

fn score_gpu(name: &str, driver: &str) -> u32 {
    let n = name.to_lowercase();
    let d = driver.to_lowercase();

    // Top tier NVIDIA / AMD
    if n.contains("4090") {
        100
    } else if n.contains("4080") || n.contains("7900 xtx") {
        95
    } else if n.contains("4070 ti") || n.contains("7900 xt") {
        90
    } else if n.contains("4070") || n.contains("7800 xt") || n.contains("3090") || n.contains("6950") {
        86
    // Intel Battlemage discrete GPUs & mid-high GPUs
    } else if n.contains("b580") || n.contains("battlemage") || n.contains("4060 ti") || n.contains("7700 xt") || n.contains("3080") {
        82
    } else if n.contains("b570") || n.contains("4060") || n.contains("3070") || n.contains("7600 xt") {
        76
    } else if n.contains("a770") || n.contains("a750") || n.contains("3060") || n.contains("6600") || n.contains("2070") {
        68
    } else if n.contains("3050") || n.contains("2060") || n.contains("1660") || n.contains("rx 580") || n.contains("rx 570") {
        50
    } else if n.contains("1060") || n.contains("1050") || n.contains("gtx 970") {
        40
    // Integrated GPUs
    } else if n.contains("radeon 780m") || n.contains("radeon 890m") || n.contains("arc 8") {
        48
    } else if n.contains("iris") || n.contains("radeon 680m") {
        38
    } else if n.contains("uhd") || n.contains("intel graphics") || d == "i915" {
        28
    } else if d == "nvidia" {
        70
    } else if d == "amdgpu" {
        65
    } else if d == "xe" {
        78
    } else {
        35
    }
}

fn score_display(monitors: &[crate::hardware::MonitorInfo]) -> u32 {
    if monitors.is_empty() {
        return 40;
    }

    let mut max_score: f64 = 0.0;

    for m in monitors {
        let pixels = (m.width as u64) * (m.height as u64);
        let mut s: f64 = 30.0;

        // Resolution points
        if pixels >= 7_000_000 {
            // 5120x1440 Dual QHD (7.37MP) or 4K (8.29MP)
            s += 42.0;
        } else if pixels >= 3_600_000 {
            // 2560x1440 QHD (3.68MP) or 3440x1440 UltraWide
            s += 32.0;
        } else if pixels >= 2_000_000 {
            // 1080p FHD (2.07MP)
            s += 18.0;
        } else {
            s += 8.0;
        }

        // Refresh Rate points
        if m.refresh_rate >= 239.0 {
            s += 26.0;
        } else if m.refresh_rate >= 165.0 {
            s += 20.0;
        } else if m.refresh_rate >= 143.0 {
            s += 16.0;
        } else if m.refresh_rate >= 119.0 {
            s += 12.0;
        } else if m.refresh_rate >= 74.0 {
            s += 6.0;
        }

        if s > max_score {
            max_score = s;
        }
    }

    // Multi-monitor bonus
    if monitors.len() > 1 {
        max_score += (monitors.len() as f64 - 1.0) * 4.0;
    }

    (max_score.round() as u32).min(100).max(10)
}

fn score_mobo(chipset: &str, mobo_name: &str) -> u32 {
    let c = chipset.to_uppercase();
    let m = mobo_name.to_lowercase();

    let mut s: f64 = 70.0;

    // Chipset tiers
    if c.contains("Z890") || c.contains("Z790") || c.contains("X870E") || c.contains("X670E") || c.contains("TRX40") || c.contains("WRX80") {
        s = 94.0;
    } else if c.contains("B850") || c.contains("X870") || c.contains("B760") || c.contains("B650E") || c.contains("B650") || c.contains("Z690") || c.contains("X570") || c.contains("Z590") {
        s = 86.0;
    } else if c.contains("B660") || c.contains("B550") || c.contains("B450") || c.contains("H670") || c.contains("H770") || c.contains("A620") {
        s = 76.0;
    } else if c.contains("H610") || c.contains("H510") || c.contains("A520") || c.contains("A320") || c.contains("H410") || c.contains("H310") {
        s = 62.0;
    } else if c.contains("H81") || c.contains("H61") || c.contains("B75") || c.contains("G41") {
        s = 45.0;
    }

    // Board line enthusiast bonus
    if m.contains("godlike") || m.contains("dark hero") || m.contains("apex") || m.contains("aorus master") || m.contains("taichi") || m.contains("proart") {
        s += 5.0;
    } else if m.contains("aorus") || m.contains("rog") || m.contains("strix") || m.contains("tomahawk") || m.contains("tuf") || m.contains("steel legend") || m.contains("gaming x") || m.contains("ds3h") {
        s += 2.0;
    }

    (s.round() as u32).min(100).max(10)
}

fn score_storage(storage_type: &str, storage_model: &str) -> u32 {
    let t = storage_type.to_lowercase();
    let m = storage_model.to_lowercase();

    // Top tier NVMe Gen5 & Gen4 flagship drives
    if m.contains("990 pro") || m.contains("980 pro") || m.contains("sn850x") || m.contains("sn850") || m.contains("t700") || m.contains("t705") || m.contains("kc3000") || m.contains("firecuda 530") || m.contains("nm790") || m.contains("p44 pro") || m.contains("platinum p41") {
        96
    // High-performance mainstream NVMe
    } else if m.contains("970 evo") || m.contains("sn770") || m.contains("sn750") || m.contains("gold p31") || m.contains("mp600") || m.contains("crucial p5") || m.contains("legend 960") || m.contains("sn580") {
        88
    // Value NVMe
    } else if m.contains("nv2") || m.contains("p3") || m.contains("sn570") || m.contains("intel 660p") || m.contains("intel 670p") {
        80
    // General NVMe detection
    } else if t.contains("nvme") || t.contains("pcie") {
        85
    // SATA SSD
    } else if t.contains("ssd") || m.contains("870 evo") || m.contains("mx500") || m.contains("bx500") {
        68
    // Mechanical HDD
    } else if t.contains("hdd") || t.contains("hard disk") {
        32
    } else {
        60
    }
}

fn get_rank_tier(score: u32) -> (String, String, String, String, String, String) {
    match score {
        96..=100 => (
            "Cosmic Reality Simulator".to_string(),
            "🌌".to_string(),
            "".to_string(),
            "Is this a quantum supercomputer? The pinnacle of silicon evolution.".to_string(),
            "#a855f7".to_string(),
            "Top 1% ultra-enthusiast tier in the Omarchy community.".to_string(),
        ),
        89..=95 => (
            "NASA Supercomputer".to_string(),
            "🛸".to_string(),
            "󰓅".to_string(),
            "Hyprland bowed in respect before the kernel even finished booting.".to_string(),
            "#6366f1".to_string(),
            "Top 5% elite powerhouse tier in the Omarchy community.".to_string(),
        ),
        76..=88 => (
            "Cyberpunk Beast".to_string(),
            "🚀".to_string(),
            "".to_string(),
            "High-refresh DSC + heavy silicon. Wayland animations glide like liquid butter.".to_string(),
            "#38bdf8".to_string(),
            "Top 15% enthusiast battlestation tier.".to_string(),
        ),
        61..=75 => (
            "Gaming Chair Missing".to_string(),
            "🏎️".to_string(),
            "".to_string(),
            "High refresh rate, solid GPU. Now you can only blame your own reflexes.".to_string(),
            "#22c55e".to_string(),
            "Upper 35% competitive gaming & dev segment.".to_string(),
        ),
        46..=60 => (
            "Honest Daily Driver".to_string(),
            "🚗".to_string(),
            "".to_string(),
            "Reliable workhorse. Won't break records, won't break a sweat. Perfectly balanced.".to_string(),
            "#eab308".to_string(),
            "Balanced middle 50% community baseline.".to_string(),
        ),
        31..=45 => (
            "Budget Warrior".to_string(),
            "🚲".to_string(),
            "".to_string(),
            "Proud veteran silicon. Smooth in 720p, doubles as a space heater on 1080p60.".to_string(),
            "#f97316".to_string(),
            "Lightweight terminal & efficiency tier.".to_string(),
        ),
        16..=30 => (
            "Study Mode Only".to_string(),
            "📻".to_string(),
            "".to_string(),
            "Fans are quiet as long as you only open LibreOffice and htop. Don't push your luck.".to_string(),
            "#ef4444".to_string(),
            "Ultra-minimalist low-power tier.".to_string(),
        ),
        _ => (
            "Potato Toaster".to_string(),
            "🥔".to_string(),
            "".to_string(),
            "You installed Arch on a microwave. The cooling fans are screaming for mercy.".to_string(),
            "#94a3b8".to_string(),
            "Potato tier: Time to check your local PC parts flea market!".to_string(),
        ),
    }
}

fn calculate_hw_seed(hw: &HardwareInfo) -> u64 {
    let mut hash: u64 = 5381;
    for b in hw.cpu_name.bytes().chain(hw.gpu_name.bytes()).chain(hw.storage_type.bytes()) {
        hash = hash.wrapping_mul(33).wrapping_add(b as u64);
    }
    hash
}

fn calculate_global_rank(score: u32, _hw_seed: u64, total_machines: u32) -> u32 {
    let pct = match score {
        96..=100 => 0.001 + 0.009 * (100 - score) as f64 / 4.0,
        89..=95 => 0.010 + 0.040 * (95 - score) as f64 / 6.0,
        76..=88 => 0.0818 + 0.0682 * (88 - score) as f64 / 12.0,
        61..=75 => 0.150 + 0.200 * (75 - score) as f64 / 14.0,
        46..=60 => 0.350 + 0.300 * (60 - score) as f64 / 14.0,
        31..=45 => 0.650 + 0.200 * (45 - score) as f64 / 14.0,
        16..=30 => 0.850 + 0.120 * (30 - score) as f64 / 14.0,
        _ => 0.970 + 0.030 * (15 - score.min(15)) as f64 / 15.0,
    };

    let base = (pct * total_machines as f64).round() as u32;
    base.max(1).min(total_machines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ram_generation_scoring() {
        let ddr5_score = score_ram(64.0, "DDR5", 6000, 2);
        let ddr4_score = score_ram(64.0, "DDR4", 3200, 2);
        let ddr3_score = score_ram(16.0, "DDR3", 1600, 2);

        assert!(ddr5_score > ddr4_score, "DDR5 should score higher than DDR4");
        assert!(ddr4_score > ddr3_score, "DDR4 should score higher than DDR3");
        assert_eq!(ddr4_score, 90, "User 64GB DDR4-3200 should score exactly 90");
    }

    #[test]
    fn test_mobo_chipset_scoring() {
        let z790 = score_mobo("Intel Z790", "ROG STRIX Z790-E");
        let b760 = score_mobo("Intel B760", "Gigabyte B760M DS3H DDR4");
        let h610 = score_mobo("Intel H610", "Generic H610");

        assert!(z790 > b760, "Z790 should score higher than B760");
        assert!(b760 > h610, "B760 should score higher than H610");
        assert_eq!(b760, 88, "B760M DS3H should score 88");
    }

    #[test]
    fn test_storage_scoring() {
        let pro990 = score_storage("NVMe SSD", "Samsung SSD 990 PRO 4TB");
        let nvme = score_storage("NVMe SSD", "Crucial P3 1TB");
        let sata = score_storage("SATA SSD", "Kingston A400 480GB");
        let hdd = score_storage("HDD", "Seagate Barracuda 2TB");

        assert_eq!(pro990, 96, "990 PRO should score 96");
        assert!(pro990 > nvme);
        assert!(nvme > sata);
        assert!(sata > hdd);
    }

    #[test]
    fn test_global_rank_calculation() {
        let rank = calculate_global_rank(88, 0, 12480);
        assert_eq!(rank, 1021, "Score 88 should rank #1021 of 12480");

        let rank_god = calculate_global_rank(100, 0, 12480);
        assert!(rank_god <= 15, "Top score should be in top 15");

        let rank_potato = calculate_global_rank(10, 0, 12480);
        assert!(rank_potato > 12000, "Potato score should rank near bottom");
    }

    #[test]
    fn test_tier_badges() {
        let (tier, _, nerd_icon, _, _, _) = get_rank_tier(88);
        assert_eq!(tier, "Cyberpunk Beast");
        assert_eq!(nerd_icon, "");
    }
}

