# OmaRank - Hardware Benchmarking and Tier Ranking for Omarchy Linux

Hardware benchmarking and tier-ranking plugin for the Omarchy desktop environment.

Author: Ozan Ozdil (ozdil)  
License: MIT  
Plugin ID: ozdil.omarank

---

## Highlights

- Native Rust Engine (`omarank-engine`): Fast, zero-overhead hardware detection directly from Linux sysfs (`/proc/cpuinfo`, `/proc/meminfo`, `/sys/class/drm/`, `/sys/class/dmi/id/`, `lspci`, `inxi`, `hyprctl`).
- 0 to 100 Weighted OmaScore (6-Component Breakdown):
  - CPU (24%): Physical cores, hyperthreads, clock speed.
  - GPU (32%): Dedicated GPU tier, driver classification (`xe`, `amdgpu`, `nvidia`).
  - RAM (16%): Generation (DDR3/DDR4/DDR5), transfer speed in MT/s, multi-channel slots, capacity.
  - Motherboard (8%): Chipset tiers and enthusiast series recognition.
  - Display (15%): High-refresh gaming and ultrawide panel detection.
  - Storage (5%): NVMe model recognition and PCIe generation classification.
- OmaStat Global Leaderboard and Tier Matrix:
  - Deterministic World Rank position estimation.
  - Complete S+ to F Tier matrix highlighting system capability.
- Native Quickshell Bar Widget (`Panel.qml`):
  - Monochrome Nerd Font rank icon in the top bar matching native Omarchy bar widgets.
  - 4-column telemetry grid for Processor, Graphics, Memory, Motherboard, Storage, Display, World Rank, and Total Score.
  - 6-pill segmented sub-score breakdown row.
  - Clean action buttons and system verdict card.
- OmaStat Community Survey:
  - Optional, privacy-preserving hardware survey.
  - 100% anonymous: zero personal data, zero MAC addresses, zero IP logging, zero usernames, zero hardware serial numbers.

---

## The OmaRank Tier Ladder

| OmaScore | Rank Tier | Assessment |
| :---: | :--- | :--- |
| **96-100** | **Cosmic Reality Simulator** | Peak silicon performance and workstation computing power. |
| **89-95** | **NASA Supercomputer** | Extreme enthusiast hardware with exceptional multi-threaded throughput. |
| **76-88** | **Cyberpunk Beast** | High-refresh rate displays combined with modern high-tier silicon. |
| **61-75** | **Gaming Rig** | High refresh rates and capable dedicated graphics for modern workloads. |
| **46-60** | **Honest Daily Driver** | Reliable, well-balanced workhorse configuration. |
| **31-45** | **Budget Warrior** | Capable hardware suitable for standard computing and light workloads. |
| **16-30** | **Study Mode Only** | Functional configuration for text editing, terminals, and light browsing. |
| **0-15** | **Minimal Hardware** | Ultra-constrained legacy hardware. |

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

# Submit anonymous hardware profile to OmaStat survey
omarank-engine --submit-survey
```

---

## Security and Architecture Standards

OmaRank complies strictly with the Omarchy Linux Security Standards (AGENTS.md):
- Plain Text UI: Every text element in `Panel.qml` explicitly sets `textFormat: Text.PlainText` to eliminate injection vulnerabilities.
- Safe IPC Execution: All CLI calls use explicit argument arrays, eliminating shell injection vectors.
- Bounded Buffers: Process output streams are strictly bounded to 64 KiB limits.
- Process Lifecycle Safety: Employs bounded timeout timers and terminates all child processes upon component destruction.
- Secure File Permissions: State files are saved to `$XDG_STATE_HOME/omarank/` with secure POSIX mode 0600 file permissions.

---

## License

MIT License. See [LICENSE](LICENSE) for details.
