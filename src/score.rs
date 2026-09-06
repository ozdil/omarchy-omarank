use crate::hardware::HardwareInfo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubScores {
    pub cpu: u32,
    pub gpu: u32,
    pub ram: u32,
    pub display: u32,
    pub storage: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmaRankResult {
    pub total_score: u32,
    pub tier_name: String,
    pub tier_icon: String,
    pub tier_quote: String,
    pub tier_color: String,
    pub sub_scores: SubScores,
    pub percentile_text: String,
    pub hardware: HardwareInfo,
}

pub fn calculate_score(hw: &HardwareInfo) -> OmaRankResult {
    let cpu_score = score_cpu(hw.cpu_cores, hw.cpu_threads, hw.cpu_mhz);
    let ram_score = score_ram(hw.ram_total_gb);
    let gpu_score = score_gpu(&hw.gpu_name, &hw.gpu_driver);
    let display_score = score_display(&hw.monitors);
    let storage_score = score_storage(&hw.storage_type);

    // Weighted Total Score (0-100)
    // CPU: 28%, GPU: 35%, RAM: 15%, Display: 17%, Storage: 5%
    let total = (cpu_score as f64 * 0.28)
        + (gpu_score as f64 * 0.35)
        + (ram_score as f64 * 0.15)
        + (display_score as f64 * 0.17)
        + (storage_score as f64 * 0.05);

    let total_score = (total.round() as u32).min(100).max(1);

    let (tier_name, tier_icon, tier_quote, tier_color, percentile_text) = get_rank_tier(total_score);

    OmaRankResult {
        total_score,
        tier_name,
        tier_icon,
        tier_quote,
        tier_color,
        sub_scores: SubScores {
            cpu: cpu_score,
            gpu: gpu_score,
            ram: ram_score,
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

fn score_ram(gb: f64) -> u32 {
    if gb >= 120.0 {
        100
    } else if gb >= 60.0 {
        96
    } else if gb >= 30.0 {
        85
    } else if gb >= 15.0 {
        62
    } else if gb >= 11.0 {
        45
    } else if gb >= 7.0 {
        28
    } else if gb >= 3.5 {
        15
    } else {
        5
    }
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

fn score_storage(storage: &str) -> u32 {
    if storage.contains("NVMe") {
        92
    } else if storage.contains("SSD") {
        70
    } else {
        30
    }
}

fn get_rank_tier(score: u32) -> (String, String, String, String, String) {
    match score {
        96..=100 => (
            "Cosmic Reality Simulator".to_string(),
            "🌌".to_string(),
            "Is this a quantum supercomputer? The pinnacle of silicon evolution.".to_string(),
            "#a855f7".to_string(),
            "Top 1% ultra-enthusiast tier in the Omarchy community.".to_string(),
        ),
        89..=95 => (
            "NASA Supercomputer".to_string(),
            "🛸".to_string(),
            "Hyprland bowed in respect before the kernel even finished booting.".to_string(),
            "#6366f1".to_string(),
            "Top 5% elite powerhouse tier in the Omarchy community.".to_string(),
        ),
        76..=88 => (
            "Cyberpunk Beast".to_string(),
            "🚀".to_string(),
            "High-refresh DSC + heavy silicon. Wayland animations glide like liquid butter.".to_string(),
            "#38bdf8".to_string(),
            "Top 15% enthusiast battlestation tier.".to_string(),
        ),
        61..=75 => (
            "Gaming Chair Missing".to_string(),
            "🏎️".to_string(),
            "High refresh rate, solid GPU. Now you can only blame your own reflexes.".to_string(),
            "#22c55e".to_string(),
            "Upper 35% competitive gaming & dev segment.".to_string(),
        ),
        46..=60 => (
            "Honest Daily Driver".to_string(),
            "🚗".to_string(),
            "Reliable workhorse. Won't break records, won't break a sweat. Perfectly balanced.".to_string(),
            "#eab308".to_string(),
            "Balanced middle 50% community baseline.".to_string(),
        ),
        31..=45 => (
            "Budget Warrior".to_string(),
            "🚲".to_string(),
            "Proud veteran silicon. Smooth in 720p, doubles as a space heater on 1080p60.".to_string(),
            "#f97316".to_string(),
            "Lightweight terminal & efficiency tier.".to_string(),
        ),
        16..=30 => (
            "Study Mode Only".to_string(),
            "📻".to_string(),
            "Fans are quiet as long as you only open LibreOffice and htop. Don't push your luck.".to_string(),
            "#ef4444".to_string(),
            "Ultra-minimalist low-power tier.".to_string(),
        ),
        _ => (
            "Potato Toaster".to_string(),
            "🥔".to_string(),
            "You installed Arch on a microwave. The cooling fans are screaming for mercy.".to_string(),
            "#94a3b8".to_string(),
            "Potato tier: Time to check your local PC parts flea market!".to_string(),
        ),
    }
}
