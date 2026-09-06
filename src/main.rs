mod hardware;
mod score;
mod survey;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    let hw = hardware::detect_hardware();
    let result = score::calculate_score(&hw);

    if args.iter().any(|a| a == "--json") {
        print_json(&result);
        return;
    }

    if args.iter().any(|a| a == "--status") {
        print_bar_status(&result);
        return;
    }

    if args.iter().any(|a| a == "--submit-survey") {
        let (ok, msg) = survey::submit_to_omastat(&result);
        let resp = serde_json::json!({
            "success": ok,
            "message": msg,
            "score": result.total_score,
            "tier": result.tier_name
        });
        println!("{}", serde_json::to_string(&resp).unwrap_or_default());
        return;
    }

    if args.iter().any(|a| a == "--ladder") {
        print_ladder();
        return;
    }

    // Default: Terminal Presentation view
    print_terminal_banner(&result);
}

fn print_json(res: &score::OmaRankResult) {
    if let Ok(json_str) = serde_json::to_string(res) {
        // Enforce 64 KiB safety bound
        if json_str.len() <= 65536 {
            println!("{}", json_str);
        } else {
            eprintln!("OmaRank JSON boyutu sınır aşıldı.");
        }
    }
}

fn print_bar_status(res: &score::OmaRankResult) {
    let text = format!("{} {} {}", res.tier_icon, res.total_score, res.tier_name);
    let tooltip = format!(
        "🏆 OmaRank: {} / 100 ({})\n• {}\n\n• CPU: {} ({} threads)\n• GPU: {}\n• RAM: {:.1} GB\n• Display: {} @ {:.0}Hz\n\n[Left Click] View Hardware Breakdown & Survey",
        res.total_score,
        res.tier_name,
        res.tier_quote,
        res.hardware.cpu_name,
        res.hardware.cpu_threads,
        res.hardware.gpu_name,
        res.hardware.ram_total_gb,
        res.hardware.monitors.first().map(|m| format!("{}x{}", m.width, m.height)).unwrap_or_else(|| "1080p".into()),
        res.hardware.monitors.first().map(|m| m.refresh_rate).unwrap_or(60.0)
    );

    let status_json = serde_json::json!({
        "text": text,
        "tooltip": tooltip,
        "score": res.total_score,
        "tier": res.tier_name,
        "color": res.tier_color,
        "icon": res.tier_icon,
    });

    println!("{}", serde_json::to_string(&status_json).unwrap_or_default());
}

fn print_terminal_banner(res: &score::OmaRankResult) {
    println!("\x1b[1;35m╔══════════════════════════════════════════════════════════════════════════════╗\x1b[0m");
    println!("\x1b[1;35m║\x1b[0m  🏆 \x1b[1;37mOMARANK\x1b[0m • \x1b[1;36mOmarchy Hardware Benchmark & Community Survey Hub\x1b[0m         \x1b[1;35m║\x1b[0m");
    println!("\x1b[1;35m╚══════════════════════════════════════════════════════════════════════════════╝\x1b[0m\n");

    println!("  \x1b[1;33m⭐ TOTAL OMASCORE:\x1b[0m  \x1b[1;32m{}/100\x1b[0m", res.total_score);
    println!("  \x1b[1;34m🏅 RANK TIER:\x1b[0m       {} \x1b[1;37m{}\x1b[0m", res.tier_icon, res.tier_name);
    println!("  \x1b[1;90m💬 CRITIC QUOTE:\x1b[0m    \"{}\"\n", res.tier_quote);

    println!("  \x1b[1;37m📊 Hardware Component Score Breakdown:\x1b[0m");
    println!("  • Processor (CPU):     {:>3}/100 {}", res.sub_scores.cpu, render_bar(res.sub_scores.cpu));
    println!("  • Graphics  (GPU):     {:>3}/100 {}", res.sub_scores.gpu, render_bar(res.sub_scores.gpu));
    println!("  • Memory    (RAM):     {:>3}/100 {}", res.sub_scores.ram, render_bar(res.sub_scores.ram));
    println!("  • Display   (Monitor): {:>3}/100 {}", res.sub_scores.display, render_bar(res.sub_scores.display));
    println!("  • Storage   (Disk):    {:>3}/100 {}\n", res.sub_scores.storage, render_bar(res.sub_scores.storage));

    println!("  \x1b[1;37m💻 Detected Hardware Specifications:\x1b[0m");
    println!("  • Processor: {} ({} Cores, {} Threads)", res.hardware.cpu_name, res.hardware.cpu_cores, res.hardware.cpu_threads);
    println!("  • Graphics:  {} [Driver: {}]", res.hardware.gpu_name, res.hardware.gpu_driver);
    println!("  • Memory:    {:.1} GB Total System RAM", res.hardware.ram_total_gb);
    if let Some(m) = res.hardware.monitors.first() {
        println!("  • Display:   {} ({}x{} @ {:.0}Hz)", m.name, m.width, m.height, m.refresh_rate);
    }
    println!("  • Storage:   {}", res.hardware.storage_type);
    println!("  • OS:        {}\n", res.hardware.os_name);

    println!("  \x1b[1;36m🌐 OmaStat Community Ranking:\x1b[0m");
    println!("  • {}", res.percentile_text);
    println!("  • Submit to anonymous community survey: \x1b[1;32momarank-engine --submit-survey\x1b[0m\n");
}

fn render_bar(score: u32) -> String {
    let width = 24;
    let filled = ((score as usize) * width) / 100;
    let empty = width.saturating_sub(filled);
    format!("\x1b[32m[{}{}]\x1b[0m", "█".repeat(filled), "░".repeat(empty))
}

fn print_ladder() {
    println!("\x1b[1;35mOmarchy OmaRank Tier Ladder:\x1b[0m\n");
    let tiers = [
        ("96-100", "🌌", "Cosmic Reality Simulator", "Is this a quantum supercomputer? The pinnacle of silicon evolution."),
        ("89-95",  "🛸", "NASA Supercomputer",        "Hyprland bowed in respect before the kernel even finished booting."),
        ("76-88",  "🚀", "Cyberpunk Beast",           "High-refresh DSC + heavy silicon. Wayland animations glide like liquid butter."),
        ("61-75",  "🏎️", "Gaming Chair Missing",     "High refresh rate, solid GPU. Now you can only blame your own reflexes."),
        ("46-60",  "🚗", "Honest Daily Driver",       "Reliable workhorse. Won't break records, won't break a sweat. Perfectly balanced."),
        ("31-45",  "🚲", "Budget Warrior",            "Proud veteran silicon. Smooth in 720p, doubles as a space heater on 1080p60."),
        ("16-30",  "📻", "Study Mode Only",           "Fans are quiet as long as you only open LibreOffice and htop. Don't push your luck."),
        ("0-15",   "🥔", "Potato Toaster",            "You installed Arch on a microwave. The cooling fans are screaming for mercy."),
    ];

    for (range, icon, name, desc) in tiers {
        println!("  \x1b[1;33m{:>7}\x1b[0m  {} \x1b[1;37m{:<28}\x1b[0m \x1b[90m{}\x1b[0m", range, icon, name, desc);
    }
    println!();
}
