# OmaRank - Hardware Benchmarking and Tier Ranking for Omarchy Linux

[![Omarchy Verified Plugin](https://img.shields.io/badge/Omarchy-Verified_Plugin-22c55e?style=for-the-badge&logo=omarchy)](https://github.com/ozdil)

[![Buy Me A Coffee](https://img.shields.io/badge/Buy_Me_A_Coffee-Support_Development-FFDD00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black)](https://buymeacoffee.com/ozdil)

![OmaRank Preview](preview.png)

Hardware benchmarking and tier-ranking plugin for the Omarchy desktop environment.

Author: Ozan Ozdil (ozdil)  
License: MIT  
Plugin ID: ozdil.omarank

---

## Highlights

- Native Rust Engine (`omarank-engine`): Fast, zero-overhead hardware detection directly from Linux sysfs (`/proc/cpuinfo`, `/proc/meminfo`, `/sys/class/drm/`, `/sys/class/dmi/id/`, `lspci`, `inxi`, `hyprctl`).
- 0 to 100 Weighted OmaScore (6-Component Breakdown):
  - CPU (24%): Physical cores, hyperthreads, clock speed.
  - GPU (32%): Discrete GPU tier table, integrated GPU class (Radeon 890M/780M/680M, Intel Lunar/Meteor/Arrow Lake, Iris Xe) and driver fallback (`xe`, `amdgpu`, `nvidia`).
  - RAM (16%): Capacity, plus generation (DDR3/DDR4/DDR5), MT/s and populated slots when `inxi -m` can read them (it needs root; otherwise they are reported as `Unknown` and earn no bonus).
  - Motherboard (8%): Chipset tiers and enthusiast series recognition.
  - Display (15%): High-refresh gaming and ultrawide panel detection.
  - Storage (5%): NVMe model recognition and PCIe generation classification.
- OmaStat Global Leaderboard and Tier Matrix:
  - Real OmaStat world rank once you have opted in to the survey; otherwise a clearly labelled offline estimate.
  - Complete S+ to F grade scale, shared with OmaStat.
- Native Quickshell Bar Widget (`Panel.qml`):
  - Monochrome Nerd Font rank icon in the top bar matching native Omarchy bar widgets.
  - 4-column telemetry grid for Processor, Graphics, Memory, Motherboard, Storage, Display, World Rank, and Total Score.
  - 6-pill segmented sub-score breakdown row.
  - Clean action buttons and system verdict card.
- OmaStat Community Survey:
  - Optional, privacy-preserving hardware survey.
  - 100% anonymous: zero personal data, zero MAC addresses, zero IP logging, zero usernames, zero hardware serial numbers.

---

## The OmaRank Grade Scale

The same scale is used by OmaStat. `--json` reports it as `grade`.

| Grade | OmaScore | Rank Tier | Assessment |
| :---: | :---: | :--- | :--- |
| **S+** | **96-100** | **Cosmic Reality Simulator** | Peak silicon performance and workstation computing power. |
| **S** | **89-95** | **NASA Supercomputer** | Extreme enthusiast hardware with exceptional multi-threaded throughput. |
| **A** | **76-88** | **Cyberpunk Beast** | High-refresh rate displays combined with modern high-tier silicon. |
| **B** | **61-75** | **Gaming Chair Missing** | High refresh rates and capable dedicated graphics for modern workloads. |
| **C** | **46-60** | **Honest Daily Driver** | Reliable, well-balanced workhorse configuration. |
| **D** | **31-45** | **Budget Warrior** | Capable hardware suitable for standard computing and light workloads. |
| **E** | **16-30** | **Study Mode Only** | Functional configuration for text editing, terminals, and light browsing. |
| **F** | **0-15** | **Potato Toaster** | Ultra-constrained legacy hardware. |

---

## Requirements

- cargo and rustc (Rust toolchain, for building from source)
- inxi and lspci (for detailed hardware identification)

---

## Installation and Setup

### Why Building from Source is Required
Under the Omarchy Linux Security Standards (AGENTS.md Rule 5.3), precompiled binaries are strictly forbidden from Git repositories to guarantee user system integrity. Therefore, the native engine must be compiled from source on your local machine after adding the plugin.

### Step 1: Add the Plugin to Omarchy
```bash
omarchy plugin add https://github.com/ozdil/omarchy-omarank.git
```

### Step 2: Build the Native Engine
Navigate to the plugin directory and compile the engine:
```bash
cd ~/.config/omarchy/plugins/ozdil.omarank
cargo build --release --locked
install -m 755 target/release/omarank-engine ./omarank-engine
```

### Step 3: Add to Omarchy Shell Configuration
Add `ozdil.omarank` to `bar.layout.right` in `~/.config/omarchy/shell.json`:
```json
{
  "id": "ozdil.omarank"
}
```

### Step 4: Restart Shell
```bash
omarchy-restart-shell
```

---

## CLI Usage

The standalone engine can be run directly from any terminal:

```bash
# Display formatted hardware score card
omarank-engine

# View full tier ladder
omarank-engine --ladder

# Output machine-readable JSON for integration
omarank-engine --json

# Re-detect hardware now (ignores the per-boot cache), then print JSON like --json
omarank-engine --rescan

# Waybar-style status JSON (text + tooltip with grade and rank)
omarank-engine --status

# Print the exact survey payload that would be sent, without sending it
omarank-engine --survey-preview

# Submit anonymous hardware profile to OmaStat survey
omarank-engine --submit-survey
```

Hardware detection is cached in `$XDG_CACHE_HOME/omarank/hardware.json` for up to one hour within the same boot (monitors are always re-read). Use `--rescan` (or the panel's re-check button) after changing hardware.

Discrete GPU scores come from a built-in table. To override it, place a JSON list such as `[{"match": ["rtx 5090"], "score": 100}]` in `~/.config/omarank/gpu_scores.json` (first match wins; the file must be owned by you, not a symlink and under 64 KiB).

---

## OmaStat Survey, World Rank and Privacy

- Nothing is sent unless you submit the survey (`--submit-survey` or the panel button). `--survey-preview` and the panel's "Preview data" toggle show the exact JSON first.
- The world rank is an offline estimate (marked "Estimated ... (offline estimate)" and `rank_is_estimate: true`) until you have opted in. After opting in, the engine looks up your real rank among OmaStat submissions at most once a day per score (`$XDG_CACHE_HOME/omarank/rank.json`).
- `install_token`: a monthly rotating hash, the first 32 hex characters of `sha256("omarank-survey-v1:" + /etc/machine-id + ":" + YYYY-MM)`. It cannot be reversed to your machine-id, changes every month, and is used only to count one submission per machine per month.
- `OMASTAT_URL` overrides the OmaStat server (base URL; `https://...`, or `http://127.0.0.1` / `http://localhost` for local testing). `HTTPS_PROXY` / `HTTP_PROXY` / `NO_PROXY` are honoured.

---

## Security and Architecture Standards

OmaRank complies strictly with the Omarchy Linux Security Standards (AGENTS.md):
- Plain Text UI: Every text element in `Panel.qml` explicitly sets `textFormat: Text.PlainText` to eliminate injection vulnerabilities.
- Safe IPC Execution: All CLI calls use explicit argument arrays, eliminating shell injection vectors.
- Bounded Buffers: Process output streams are strictly bounded to 64 KiB limits.
- Process Lifecycle Safety: Employs bounded timeout timers and terminates all child processes upon component destruction.
- Secure File Permissions: State files are saved to `$XDG_STATE_HOME/omarank/` with secure POSIX mode 0600 file permissions.

---

## What's New in 1.4.0

Full list of fixes and changes: [CHANGELOG.md](CHANGELOG.md).

- RAM details that cannot be read without root are reported as `Unknown` instead of a made-up DDR4-3200 kit, and are not scored.
- Integrated GPUs (AMD APUs, Intel iGPUs) are detected (`gpu_integrated`, `gpu_vram_mb`) and scored by class instead of the generic driver score.
- The discrete GPU score table is data (`src/gpu_scores.json`) and can be overridden per user.
- Hardware detection is cached per boot; `--rescan` forces a fresh scan.
- The survey body is sent over stdin (not visible in `ps`), proxies are honoured and `OMASTAT_URL` can point the engine at another server.
- Survey payloads carry an anonymous monthly `install_token`.
- The world rank is real (opt-in) or clearly labelled as an estimate; the unified S+..F `grade` is shown everywhere.
- `--survey-preview` and the panel's "Preview data" toggle.
- AUR-ready PKGBUILD, CI (fmt, clippy, tests, script syntax, version check) and CodeQL for Rust.

---

## Support & Sponsorship

If you find OmaRank useful and want to support independent Linux development:

<a href="https://buymeacoffee.com/ozdil" target="_blank"><img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me A Coffee" style="height: 50px !important;width: 180px !important;" ></a>

---

## License

MIT License. See [LICENSE](LICENSE) for details.
