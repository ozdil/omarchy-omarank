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
        print_ladder(&result);
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
        "nerd_icon": res.tier_nerd_icon,
    });

    println!("{}", serde_json::to_string(&status_json).unwrap_or_default());
}

fn print_terminal_banner(res: &score::OmaRankResult) {
    println!("\x1b[1;35m╔══════════════════════════════════════════════════════════════════════════════╗\x1b[0m");
    println!("\x1b[1;35m║\x1b[0m  🏆 \x1b[1;37mOMARANK\x1b[0m • \x1b[1;36mOmarchy Hardware Benchmark & Community Survey Hub\x1b[0m         \x1b[1;35m║\x1b[0m");
    println!("\x1b[1;35m╚══════════════════════════════════════════════════════════════════════════════╝\x1b[0m\n");

    println!("  \x1b[1;36m🆔 BATTLESTATION ID:\x1b[0m    \x1b[1;35m{}\x1b[0m (Local privacy badge)", res.battlestation_id);
    println!("  \x1b[1;36m🧬 BUILD ARCHETYPE:\x1b[0m     \x1b[1;36m{}\x1b[0m", res.archetype_signature);
    println!("  \x1b[1;33m⭐ TOTAL OMASCORE:\x1b[0m      \x1b[1;32m{}/100\x1b[0m", res.total_score);
    println!("  \x1b[1;34m🏅 RANK TIER:\x1b[0m           {} \x1b[1;37m{}\x1b[0m", res.tier_nerd_icon, res.tier_name);
    println!("  \x1b[1;36m🌐 OMASTAT WORLD RANK:\x1b[0m  \x1b[1;32m#{}\x1b[0m of {} battlestations ({})", res.global_rank, res.total_machines, res.percentile_text);
    println!("  \x1b[1;90m💬 CRITIC QUOTE:\x1b[0m        \"{}\"\n", res.tier_quote);

    println!("  \x1b[1;37m📊 Hardware Component Score Breakdown:\x1b[0m");
    println!("  • Processor (CPU):     {:>3}/100 {}", res.sub_scores.cpu, render_bar(res.sub_scores.cpu));
    println!("  • Graphics  (GPU):     {:>3}/100 {}", res.sub_scores.gpu, render_bar(res.sub_scores.gpu));
    println!("  • Memory    (RAM):     {:>3}/100 {}", res.sub_scores.ram, render_bar(res.sub_scores.ram));
    println!("  • Motherboard (Mobo):  {:>3}/100 {}", res.sub_scores.mobo, render_bar(res.sub_scores.mobo));
    println!("  • Display   (Monitor): {:>3}/100 {}", res.sub_scores.display, render_bar(res.sub_scores.display));
    println!("  • Storage   (Disk):    {:>3}/100 {}\n", res.sub_scores.storage, render_bar(res.sub_scores.storage));

    println!("  \x1b[1;37m💻 Detected Hardware Specifications:\x1b[0m");
    println!("  • Processor:   {} ({} Cores, {} Threads)", res.hardware.cpu_name, res.hardware.cpu_cores, res.hardware.cpu_threads);
    println!("  • Graphics:    {} [Driver: {}]", res.hardware.gpu_name, res.hardware.gpu_driver);
    println!("  • Motherboard: {} {} (Chipset: {}, BIOS: {})", res.hardware.mobo_vendor, res.hardware.mobo_name, res.hardware.chipset, res.hardware.mobo_bios);
    println!("  • Memory:      {:.1} GB {} @ {} MT/s ({} populated slots)", res.hardware.ram_total_gb, res.hardware.ram_type, res.hardware.ram_speed_mts, res.hardware.ram_modules);
    if let Some(m) = res.hardware.monitors.first() {
        println!("  • Display:     {} ({}x{} @ {:.0}Hz)", m.name, m.width, m.height, m.refresh_rate);
    }
    println!("  • Storage:     {} ({})", res.hardware.storage_model, res.hardware.storage_type);
    println!("  • OS:          {}\n", res.hardware.os_name);

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

fn print_ladder(res: &score::OmaRankResult) {
    println!("\x1b[1;32m╔════════════════════════════════════════════════════════════════════════════════════════════╗\x1b[0m");
    println!("\x1b[1;32m║\x1b[0m  🌐 \x1b[1;37mOMASTAT GLOBAL HARDWARE LADDER & TIER LIST\x1b[0m ({} Active Battlestations)             \x1b[1;32m║\x1b[0m", res.total_machines);
    println!("\x1b[1;32m╚════════════════════════════════════════════════════════════════════════════════════════════╝\x1b[0m\n");

    let tiers = [
        ("S+", "96-100", "#1 - #124",     "", "Cosmic Reality Simulator", "Top 1%"),
        ("S",  "89-95",  "#125 - #624",   "󰓅", "NASA Supercomputer",        "Top 5%"),
        ("A",  "76-88",  "#625 - #1872",  "", "Cyberpunk Beast",           "Top 15%"),
        ("B",  "61-75",  "#1873 - #4368", "", "Gaming Chair Missing",     "Upper 35%"),
        ("C",  "46-60",  "#4369 - #8112", "", "Honest Daily Driver",       "Mid 50%"),
        ("D",  "31-45",  "#8113 - #10608","", "Budget Warrior",            "Lower 25%"),
        ("E",  "16-30",  "#10609-#12105", "", "Study Mode Only",           "Ultra-Eco"),
        ("F",  "0-15",   "#12106-#12480", "", "Potato Toaster",            "Potato"),
    ];

    for (t_badge, range, rank_span, icon, name, share) in tiers {
        let is_my_tier = (res.total_score >= 76 && t_badge == "A")
            || (res.total_score >= 89 && res.total_score <= 95 && t_badge == "S")
            || (res.total_score >= 96 && t_badge == "S+")
            || (res.total_score >= 61 && res.total_score <= 75 && t_badge == "B")
            || (res.total_score >= 46 && res.total_score <= 60 && t_badge == "C")
            || (res.total_score >= 31 && res.total_score <= 45 && t_badge == "D")
            || (res.total_score >= 16 && res.total_score <= 30 && t_badge == "E")
            || (res.total_score < 16 && t_badge == "F");

        if is_my_tier {
            println!("  \x1b[1;32m┌────────────────────────────────────────────────────────────────────────────────────────┐\x1b[0m");
            println!("  \x1b[1;32m│\x1b[0m \x1b[1;33m[{:>2}]\x1b[0m {:<7} {:<15} {} \x1b[1;37m{:<27}\x1b[0m \x1b[1;32m{:<9}\x1b[0m \x1b[1;32m│\x1b[0m", t_badge, range, rank_span, icon, name, share);
            println!("  \x1b[1;32m│\x1b[0m   \x1b[1;32m👉 WORLD RANK #{} [YOUR PC]\x1b[0m • Score: \x1b[1;37m{}/100\x1b[0m ({})                    \x1b[1;32m│\x1b[0m", res.global_rank, res.total_score, res.tier_name);
            println!("  \x1b[1;32m└────────────────────────────────────────────────────────────────────────────────────────┘\x1b[0m");
        } else {
            println!("    \x1b[1;30m[{:>2}]\x1b[0m {:<7} {:<15} {} \x1b[0;37m{:<27}\x1b[0m \x1b[90m{:<9}\x1b[0m", t_badge, range, rank_span, icon, name, share);
        }
    }
    println!();
}
