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
    /// Tier letter on the unified OmaRank/OmaStat scale (S+, S, A, B, C, D, E, F).
    pub grade: String,
    pub tier_name: String,
    pub tier_icon: String,
    pub tier_nerd_icon: String,
    pub tier_quote: String,
    pub tier_color: String,
    pub sub_scores: SubScores,
    pub percentile_text: String,
    /// True while global_rank/total_machines are the offline estimate rather than a real
    /// OmaStat rank (only fetched after the user opted in to the survey).
    pub rank_is_estimate: bool,
    pub battlestation_id: String,
    pub archetype_signature: String,
    pub hardware: HardwareInfo,
}

pub fn calculate_score(hw: &HardwareInfo) -> OmaRankResult {
    let cpu_score = score_cpu(hw.cpu_cores, hw.cpu_threads, hw.cpu_mhz);
    let ram_score = score_ram(
        hw.ram_total_gb,
        &hw.ram_type,
        hw.ram_speed_mts,
        hw.ram_modules,
    );
    let mobo_score = score_mobo(&hw.chipset, &hw.mobo_name);
    let gpu_score = score_gpu(&hw.gpu_name, &hw.gpu_driver, hw.gpu_integrated);
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

    let total_score = (total.round() as u32).clamp(1, 100);

    let total_machines = 12480;
    let hw_seed = calculate_hw_seed(hw);
    let global_rank = calculate_global_rank(total_score, hw_seed, total_machines);

    let (grade, tier_name, tier_icon, tier_nerd_icon, tier_quote, tier_color, _) =
        get_rank_tier(total_score);

    // Offline estimate from a fixed score distribution; crate::rank replaces it with the
    // real OmaStat rank when the user has opted in.
    let pct = (global_rank as f64 / total_machines as f64) * 100.0;
    let percentile_text = format!("Estimated Top {:.1}% (offline estimate)", pct);

    let battlestation_id = generate_local_battlestation_id();
    let archetype_signature = generate_archetype_signature(hw);

    OmaRankResult {
        total_score,
        global_rank,
        total_machines,
        grade,
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
        rank_is_estimate: true,
        battlestation_id,
        archetype_signature,
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

    (s.round() as u32).clamp(5, 100)
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

    // Generation bonus (DDR5 / DDR4 / DDR3). An undetected type ("Unknown") earns nothing,
    // and the speed/channel bonuses below are naturally zero for 0 MT/s and 0 modules.
    let t = ram_type.to_uppercase();
    if t.contains("DDR5") {
        s += 10.0;
    } else if t.contains("DDR4") {
        s += 6.0;
    } else if t.contains("DDR3") {
        s += 0.0;
    } else if t.contains("DDR2") {
        s -= 6.0;
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

    (s.round() as u32).clamp(5, 100)
}

/// One row of the discrete GPU table: any lowercase substring in `match` selects `score`.
#[derive(Debug, Clone, Deserialize)]
struct GpuScoreRow {
    #[serde(rename = "match")]
    patterns: Vec<String>,
    score: u32,
}

const BUILTIN_GPU_SCORES: &str = include_str!("gpu_scores.json");

fn parse_gpu_table(json: &str) -> Option<Vec<GpuScoreRow>> {
    let mut rows: Vec<GpuScoreRow> = serde_json::from_str(json).ok()?;
    if rows.is_empty() {
        return None;
    }
    for row in &mut rows {
        row.score = row.score.min(100);
        row.patterns = row
            .patterns
            .iter()
            .map(|p| p.trim().to_lowercase())
            .filter(|p| !p.is_empty())
            .collect();
    }
    Some(rows)
}

/// User override: `$XDG_CONFIG_HOME/omarank/gpu_scores.json` (or `~/.config/omarank/...`).
/// It must be a regular, non-symlink file owned by us, under 64 KiB, and parse cleanly;
/// otherwise the table embedded at build time is used.
fn load_user_gpu_table() -> Option<Vec<GpuScoreRow>> {
    let path = crate::securefs::xdg_path("XDG_CONFIG_HOME", ".config", "gpu_scores.json")?;
    let content = crate::securefs::read_owned_file(&path, 64 * 1024)?;
    parse_gpu_table(&content)
}

fn gpu_table() -> &'static [GpuScoreRow] {
    static TABLE: std::sync::OnceLock<Vec<GpuScoreRow>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        // Unit tests must not depend on whatever the developer has in ~/.config
        let user = if cfg!(test) {
            None
        } else {
            load_user_gpu_table()
        };
        user.or_else(|| parse_gpu_table(BUILTIN_GPU_SCORES))
            .unwrap_or_default()
    })
}

/// Integrated GPU classes by codename / marketing name as lspci reports them.
/// Returns None when nothing iGPU-specific is recognised.
fn integrated_gpu_class(n: &str) -> Option<u32> {
    let has = |pats: &[&str]| pats.iter().any(|p| n.contains(p));
    if has(&["strix halo", "8060s", "8050s"]) {
        // Ryzen AI Max (40 CU RDNA 3.5) is far ahead of the other APUs
        Some(60)
    } else if has(&[
        "strix point",
        "strix",
        "krackan",
        "890m",
        "880m",
        "860m",
        "840m",
    ]) {
        // RDNA 3.5 (Radeon 890M class)
        Some(52)
    } else if has(&["phoenix", "hawk point", "780m", "760m", "740m"]) {
        // RDNA 3 (Radeon 780M class)
        Some(48)
    } else if has(&["lunar lake", "arc 130v", "arc 140v"]) {
        Some(46)
    } else if has(&["arrow lake", "meteor lake", "arc 8"]) {
        Some(42)
    } else if has(&["rembrandt", "680m", "660m", "iris"]) {
        // RDNA 2 (Radeon 680M) and Intel Iris Xe
        Some(38)
    } else if has(&[
        "renoir",
        "cezanne",
        "lucienne",
        "barcelo",
        "picasso",
        "raven",
        "mendocino",
        "van gogh",
    ]) {
        // Vega / RDNA 2 lite APUs
        Some(30)
    } else if has(&["uhd", "intel graphics"]) {
        Some(28)
    } else {
        None
    }
}

fn score_integrated_gpu(n: &str, d: &str) -> u32 {
    integrated_gpu_class(n).unwrap_or(if d == "amdgpu" || n.contains("amd") {
        30
    } else {
        28
    })
}

fn score_gpu(name: &str, driver: &str, integrated: bool) -> u32 {
    let n = name.to_lowercase();
    let d = driver.to_lowercase();

    // An iGPU must never reach the generic per-driver scores below (amdgpu 65 / xe 78),
    // which are meant for unrecognised discrete cards.
    if integrated {
        return score_integrated_gpu(&n, &d);
    }

    // Discrete cards: first matching row of the (overridable) score table wins
    if let Some(score) = gpu_table()
        .iter()
        .find(|row| row.patterns.iter().any(|p| n.contains(p.as_str())))
        .map(|row| row.score)
    {
        score
    // Integrated GPU names that slipped through detection without the integrated flag
    } else if let Some(class) = integrated_gpu_class(&n) {
        class
    } else if d == "i915" {
        28
    } else if d.starts_with("nvidia") {
        // nvidia-smi reports the driver as "nvidia <version>"
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

    (max_score.round() as u32).clamp(10, 100)
}

fn score_mobo(chipset: &str, mobo_name: &str) -> u32 {
    let c = chipset.to_uppercase();
    let m = mobo_name.to_lowercase();

    let mut s: f64 = 70.0;

    // Chipset tiers
    if c.contains("Z890")
        || c.contains("Z790")
        || c.contains("X870E")
        || c.contains("X670E")
        || c.contains("TRX40")
        || c.contains("WRX80")
    {
        s = 94.0;
    } else if c.contains("B850")
        || c.contains("X870")
        || c.contains("B760")
        || c.contains("B650E")
        || c.contains("B650")
        || c.contains("Z690")
        || c.contains("X570")
        || c.contains("Z590")
        || c.contains("ARROW LAKE")
        || c.contains("LUNAR LAKE")
        || c.contains("METEOR LAKE")
        || c.contains("STRIX POINT")
    {
        s = 86.0;
    } else if c.contains("B660")
        || c.contains("B550")
        || c.contains("B450")
        || c.contains("H670")
        || c.contains("H770")
        || c.contains("A620")
        || c.contains("RAPTOR LAKE")
        || c.contains("ALDER LAKE")
        || c.contains("HAWK POINT")
        || c.contains("PHOENIX")
    {
        s = 78.0;
    } else if c.contains("H610")
        || c.contains("H510")
        || c.contains("A520")
        || c.contains("A320")
        || c.contains("H410")
        || c.contains("H310")
    {
        s = 62.0;
    } else if c.contains("H81") || c.contains("H61") || c.contains("B75") || c.contains("G41") {
        s = 45.0;
    }

    // Board line enthusiast bonus
    if m.contains("godlike")
        || m.contains("dark hero")
        || m.contains("apex")
        || m.contains("aorus master")
        || m.contains("taichi")
        || m.contains("proart")
    {
        s += 5.0;
    } else if m.contains("aorus")
        || m.contains("rog")
        || m.contains("strix")
        || m.contains("tomahawk")
        || m.contains("tuf")
        || m.contains("steel legend")
        || m.contains("gaming x")
        || m.contains("ds3h")
    {
        s += 2.0;
    }

    (s.round() as u32).clamp(10, 100)
}

fn score_storage(storage_type: &str, storage_model: &str) -> u32 {
    let t = storage_type.to_lowercase();
    let m = storage_model.to_lowercase();

    // Top tier NVMe Gen5 & Gen4 flagship drives
    if m.contains("990 pro")
        || m.contains("980 pro")
        || m.contains("sn850x")
        || m.contains("sn850")
        || m.contains("t700")
        || m.contains("t705")
        || m.contains("kc3000")
        || m.contains("firecuda 530")
        || m.contains("nm790")
        || m.contains("p44 pro")
        || m.contains("platinum p41")
    {
        96
    // High-performance mainstream NVMe
    } else if m.contains("970 evo")
        || m.contains("sn770")
        || m.contains("sn750")
        || m.contains("gold p31")
        || m.contains("mp600")
        || m.contains("crucial p5")
        || m.contains("legend 960")
        || m.contains("sn580")
    {
        88
    // Value NVMe
    } else if m.contains("nv2")
        || m.contains("p3")
        || m.contains("sn570")
        || m.contains("intel 660p")
        || m.contains("intel 670p")
    {
        80
    // General NVMe detection
    } else if t.contains("nvme") || t.contains("pcie") {
        85
    // SATA SSD
    } else if t.contains("ssd")
        || m.contains("870 evo")
        || m.contains("mx500")
        || m.contains("bx500")
    {
        68
    // Mechanical HDD
    } else if t.contains("hdd") || t.contains("hard disk") {
        32
    } else {
        60
    }
}

/// Unified grade scale shared with OmaStat: (grade, min score, max score, tier name).
pub const GRADE_SCALE: [(&str, u32, u32, &str); 8] = [
    ("S+", 96, 100, "Cosmic Reality Simulator"),
    ("S", 89, 95, "NASA Supercomputer"),
    ("A", 76, 88, "Cyberpunk Beast"),
    ("B", 61, 75, "Gaming Chair Missing"),
    ("C", 46, 60, "Honest Daily Driver"),
    ("D", 31, 45, "Budget Warrior"),
    ("E", 16, 30, "Study Mode Only"),
    ("F", 0, 15, "Potato Toaster"),
];

/// (grade, tier name, icon, nerd icon, quote, color, blurb). The match arms mirror GRADE_SCALE.
type RankTier = (String, String, String, String, String, String, String);

pub fn get_rank_tier(score: u32) -> RankTier {
    match score {
        96..=100 => (
            "S+".to_string(),
            "Cosmic Reality Simulator".to_string(),
            "".to_string(),
            "".to_string(),
            "Is this a quantum supercomputer? The pinnacle of silicon evolution.".to_string(),
            "#a855f7".to_string(),
            "Top 1% ultra-enthusiast tier in the Omarchy community.".to_string(),
        ),
        89..=95 => (
            "S".to_string(),
            "NASA Supercomputer".to_string(),
            "󰓅".to_string(),
            "󰓅".to_string(),
            "Hyprland bowed in respect before the kernel even finished booting.".to_string(),
            "#6366f1".to_string(),
            "Top 5% elite powerhouse tier in the Omarchy community.".to_string(),
        ),
        76..=88 => (
            "A".to_string(),
            "Cyberpunk Beast".to_string(),
            "".to_string(),
            "".to_string(),
            "High-refresh DSC + heavy silicon. Wayland animations glide like liquid butter."
                .to_string(),
            "#38bdf8".to_string(),
            "Top 15% enthusiast battlestation tier.".to_string(),
        ),
        61..=75 => (
            "B".to_string(),
            "Gaming Chair Missing".to_string(),
            "".to_string(),
            "".to_string(),
            "High refresh rate, solid GPU. Now you can only blame your own reflexes.".to_string(),
            "#22c55e".to_string(),
            "Upper 35% competitive gaming & dev segment.".to_string(),
        ),
        46..=60 => (
            "C".to_string(),
            "Honest Daily Driver".to_string(),
            "".to_string(),
            "".to_string(),
            "Reliable workhorse. Won't break records, won't break a sweat. Perfectly balanced."
                .to_string(),
            "#eab308".to_string(),
            "Balanced middle 50% community baseline.".to_string(),
        ),
        31..=45 => (
            "D".to_string(),
            "Budget Warrior".to_string(),
            "".to_string(),
            "".to_string(),
            "Proud veteran silicon. Smooth in 720p, doubles as a space heater on 1080p60."
                .to_string(),
            "#f97316".to_string(),
            "Lightweight terminal & efficiency tier.".to_string(),
        ),
        16..=30 => (
            "E".to_string(),
            "Study Mode Only".to_string(),
            "".to_string(),
            "".to_string(),
            "Fans are quiet as long as you only open LibreOffice and htop. Don't push your luck."
                .to_string(),
            "#ef4444".to_string(),
            "Ultra-minimalist low-power tier.".to_string(),
        ),
        _ => (
            "F".to_string(),
            "Potato Toaster".to_string(),
            "".to_string(),
            "".to_string(),
            "You installed Arch on a microwave. The cooling fans are screaming for mercy."
                .to_string(),
            "#94a3b8".to_string(),
            "Potato tier: Time to check your local PC parts flea market!".to_string(),
        ),
    }
}

fn calculate_hw_seed(hw: &HardwareInfo) -> u64 {
    let mut hash: u64 = 5381;
    for b in hw
        .cpu_name
        .bytes()
        .chain(hw.gpu_name.bytes())
        .chain(hw.storage_type.bytes())
    {
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

fn generate_local_battlestation_id() -> String {
    let seed = if let Ok(id_str) = std::fs::read_to_string("/etc/machine-id") {
        let trimmed = id_str.trim();
        let mut h: u64 = 0xcbf29ce484222325;
        for b in "omarchy-battlestation-salt:".bytes().chain(trimmed.bytes()) {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    } else {
        let mut h: u64 = 0xcbf29ce484222325;
        if let Ok(host) = std::env::var("HOSTNAME") {
            for b in host.bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
        }
        h
    };

    let p1 = (seed >> 32) as u32 & 0xFFFF;
    let p2 = (seed & 0xFFFF) as u32;
    format!("OMA-{:04X}-{:04X}", p1, p2)
}

fn generate_archetype_signature(hw: &HardwareInfo) -> String {
    let cpu_part = if hw.cpu_name.contains("Ultra 9") {
        "U9"
    } else if hw.cpu_name.contains("Ultra 7") {
        "U7"
    } else if hw.cpu_name.contains("Ultra 5") {
        "U5"
    } else if hw.cpu_name.contains("Ryzen 9") || hw.cpu_name.contains("Ryzen AI 9") {
        "R9"
    } else if hw.cpu_name.contains("Ryzen 7") || hw.cpu_name.contains("Ryzen AI 7") {
        "R7"
    } else if hw.cpu_name.contains("Ryzen 5") || hw.cpu_name.contains("Ryzen AI 5") {
        "R5"
    } else if hw.cpu_name.contains("i9") {
        "I9"
    } else if hw.cpu_name.contains("i7") {
        "I7"
    } else if hw.cpu_name.contains("i5") {
        "I5"
    } else {
        "CPU"
    };

    let gpu_part = if hw.gpu_name.contains("RTX 5090") {
        "5090"
    } else if hw.gpu_name.contains("RTX 5080") {
        "5080"
    } else if hw.gpu_name.contains("RTX 5070") {
        "5070"
    } else if hw.gpu_name.contains("RTX 4090") {
        "4090"
    } else if hw.gpu_name.contains("RTX 4080") {
        "4080"
    } else if hw.gpu_name.contains("RTX 4070") {
        "4070"
    } else if hw.gpu_name.contains("RTX 5060") {
        "5060"
    } else if hw.gpu_name.contains("RTX 4060") {
        "4060"
    } else if hw.gpu_name.contains("B580") || hw.gpu_name.contains("Battlemage") {
        "B580"
    } else if hw.gpu_name.contains("Intel Arc") || hw.gpu_name.contains("Meteor Lake") {
        "ARC"
    } else if hw.gpu_name.contains("Radeon") {
        "RADEON"
    } else {
        "GPU"
    };

    let ram_gb = (hw.ram_total_gb.round() as u32).to_string();
    let hz = hw
        .monitors
        .first()
        .map(|m| format!("{:.0}H", m.refresh_rate))
        .unwrap_or_else(|| "60H".into());

    format!("OMA-{}-{}-{}G-{}", cpu_part, gpu_part, ram_gb, hz)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ram_generation_scoring() {
        let ddr5_score = score_ram(64.0, "DDR5", 6000, 2);
        let ddr4_score = score_ram(64.0, "DDR4", 3200, 2);
        let ddr3_score = score_ram(16.0, "DDR3", 1600, 2);

        assert!(
            ddr5_score > ddr4_score,
            "DDR5 should score higher than DDR4"
        );
        assert!(
            ddr4_score > ddr3_score,
            "DDR4 should score higher than DDR3"
        );
        assert_eq!(
            ddr4_score, 90,
            "User 64GB DDR4-3200 should score exactly 90"
        );
    }

    #[test]
    fn test_ram_unknown_details_get_no_bonus() {
        // Without root, inxi reports nothing: only the capacity baseline may count
        assert_eq!(score_ram(16.0, "Unknown", 0, 0), 46);
        assert_eq!(score_ram(32.0, "Unknown", 0, 0), 62);
        assert!(score_ram(16.0, "DDR4", 3200, 2) > score_ram(16.0, "Unknown", 0, 0));
        assert!(score_ram(16.0, "DDR3", 1600, 1) == score_ram(16.0, "Unknown", 0, 0));
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
    fn test_next_gen_gpu_scoring() {
        // RDNA4 / RTX 50 mid-range used to fall through to the generic driver score
        assert_eq!(score_gpu("AMD Radeon RX 9070 XT", "amdgpu", false), 90);
        assert_eq!(score_gpu("AMD Radeon RX 9070", "amdgpu", false), 86);
        assert_eq!(score_gpu("NVIDIA GeForce RTX 5060 Ti", "nvidia", false), 82);
        assert_eq!(score_gpu("NVIDIA GeForce RTX 5060", "nvidia", false), 76);
        assert_eq!(score_gpu("NVIDIA GeForce RTX 5070 Ti", "nvidia", false), 95);
    }

    #[test]
    fn test_integrated_gpu_scoring() {
        let amd = "Advanced Micro Devices, Inc. [AMD/ATI] ";
        assert_eq!(score_gpu(&format!("{amd}Phoenix1"), "amdgpu", true), 48);
        assert_eq!(score_gpu(&format!("{amd}Hawk Point1"), "amdgpu", true), 48);
        assert_eq!(
            score_gpu(&format!("{amd}Strix [Radeon 880M / 890M]"), "amdgpu", true),
            52
        );
        assert_eq!(
            score_gpu(
                &format!("{amd}Krackan [Radeon 840M / 860M]"),
                "amdgpu",
                true
            ),
            52
        );
        assert_eq!(
            score_gpu(&format!("{amd}Rembrandt [Radeon 680M]"), "amdgpu", true),
            38
        );
        assert_eq!(
            score_gpu(
                &format!("{amd}Cezanne [Radeon Vega Series]"),
                "amdgpu",
                true
            ),
            30
        );
        assert_eq!(score_gpu(&format!("{amd}Renoir"), "amdgpu", true), 30);
        // Unknown APU must not get the generic discrete amdgpu score (65)
        assert_eq!(score_gpu(&format!("{amd}Device 15bf"), "amdgpu", true), 30);

        let intel = "Intel Corporation ";
        assert_eq!(
            score_gpu(
                &format!("{intel}Lunar Lake [Intel Arc Graphics 130V / 140V]"),
                "xe",
                true
            ),
            46
        );
        assert_eq!(
            score_gpu(
                &format!("{intel}Meteor Lake-P [Intel Arc Graphics]"),
                "i915",
                true
            ),
            42
        );
        assert_eq!(
            score_gpu(&format!("{intel}Arrow Lake-P [Intel Graphics]"), "xe", true),
            42
        );
        assert_eq!(
            score_gpu(
                &format!("{intel}TigerLake-LP GT2 [Iris Xe Graphics]"),
                "i915",
                true
            ),
            38
        );
        assert_eq!(
            score_gpu(
                &format!("{intel}Alder Lake-P GT1 [UHD Graphics]"),
                "i915",
                true
            ),
            28
        );
        // An xe-driven iGPU must not get the generic discrete xe score (78)
        assert_eq!(score_gpu(&format!("{intel}Device 7d67"), "xe", true), 28);

        // Discrete cards with unknown names still use the per-driver fallback
        assert_eq!(
            score_gpu(
                "Advanced Micro Devices, Inc. [AMD/ATI] Device 73ff",
                "amdgpu",
                false
            ),
            65
        );
        assert_eq!(
            score_gpu("NVIDIA Quadro Mystery", "nvidia 550.54", false),
            70
        );
    }

    #[test]
    fn test_gpu_table_parsing() {
        let builtin = parse_gpu_table(BUILTIN_GPU_SCORES).expect("embedded table must parse");
        assert_eq!(builtin[0].patterns, vec!["5090"]);
        assert_eq!(builtin[0].score, 100);

        let custom = parse_gpu_table(r#"[{"match": [" RTX 9999 "], "score": 250}]"#).unwrap();
        assert_eq!(custom[0].patterns, vec!["rtx 9999"]);
        assert_eq!(custom[0].score, 100, "scores are capped at 100");

        assert!(parse_gpu_table("[]").is_none());
        assert!(parse_gpu_table("{\"match\": 1}").is_none());
    }

    #[test]
    fn test_grade_boundaries() {
        let cases = [
            (100, "S+"),
            (96, "S+"),
            (95, "S"),
            (89, "S"),
            (88, "A"),
            (76, "A"),
            (75, "B"),
            (61, "B"),
            (60, "C"),
            (46, "C"),
            (45, "D"),
            (31, "D"),
            (30, "E"),
            (16, "E"),
            (15, "F"),
            (1, "F"),
            (0, "F"),
        ];
        for (score, expected) in cases {
            assert_eq!(get_rank_tier(score).0, expected, "score {}", score);
        }
        // GRADE_SCALE and get_rank_tier must agree on every score and tier name
        for score in 0..=100 {
            let (grade, name, ..) = get_rank_tier(score);
            let row = GRADE_SCALE
                .iter()
                .find(|(_, min, max, _)| (*min..=*max).contains(&score))
                .expect("every score has a grade");
            assert_eq!(
                (row.0, row.3),
                (grade.as_str(), name.as_str()),
                "score {}",
                score
            );
        }
    }

    #[test]
    fn test_tier_badges() {
        let (grade, tier, _, nerd_icon, _, _, _) = get_rank_tier(88);
        assert_eq!(grade, "A");
        assert_eq!(tier, "Cyberpunk Beast");
        assert_eq!(nerd_icon, "");
    }
}
