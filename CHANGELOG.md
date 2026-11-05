# Changelog

All notable changes to OmaRank are listed here.

## 1.5.0 (unreleased)

Targets Qt 6.12 LTS and the current Omarchy shell plugin contract.

### Fixed

- **Panel colours broke on Qt 6.12.** Qt 6.12 adds a `Color` type to QtQuick that shadows the Omarchy `qs.Commons` palette singleton, so every bare `Color.foreground` / `Color.accent` in `Panel.qml` resolved to the wrong object (qmllint 6.12: 49 `Member "foreground" not found`). The panel now imports `qs.Commons as Commons` and reads `Commons.Color`, as Omarchy requires of plugins.
- **The panel started a second Quickshell.** "Open dashboard" ran `omarank-dashboard`, which launched `quickshell -p qml/shell.qml` next to `omarchy-shell`; Omarchy plugins must never do that. The panel now opens the terminal dashboard through `omarchy-launch-or-focus-tui` (focusing an existing window), and inside Omarchy `omarank-dashboard` does the same unless `--window` is given.
- **The PKGBUILD installed the widget where Omarchy never looks** (`/usr/share/omarchy/plugins`). The package now installs the engine to `/usr/bin`, a `/usr/bin/omarank-dashboard` link and the desktop entry, and tells you to add the plugin with `omarchy plugin add`; the panel falls back to `/usr/bin/omarank-engine` when its folder has no build.
- **Wayland session variables were dropped** by the scripts' `env -i` re-exec (`XDG_SESSION_TYPE`, `XDG_DATA_DIRS`, `DBUS_SESSION_BUS_ADDRESS`, `OMARCHY_PATH`), so Qt could pick xcb under Xwayland and terminal/desktop-entry lookup failed. They are passed through, with `QT_QPA_PLATFORM` defaulting to `wayland;xcb` and client-side decorations off.
- **`omarank-engine --json | head` panicked** with "failed printing to stdout: Broken pipe" when the reader closed the pipe early. Output now goes through a writer that exits quietly (status 0) on a broken pipe.
- **Fake 1920x1080 display off Hyprland.** Without `hyprctl` the engine invented a monitor; it now reads connected outputs from DRM sysfs first, and skips disabled Hyprland outputs.
- **Standalone theme colours were NaN** when `colors.toml` gave a foreground/accent without the optional keys (`.r/.g/.b` read on a string).

### Changed

- Omarchy design alignment: no hard-coded colours (verified badge, coffee button, about overlay, cards now use the palette, `Style.normalFillFor`, `Border.controlSpec` and `Commons.Color.popups`); corners follow Hyprland's rounding (`Style.cornerRadius`, square by default); the standalone window uses the `monospace` font alias and Hyprland's rounding; animations go through a reduced-motion aware duration and stay under 300 ms.
- The panel card is lazy (`KeyboardPanel.content`), so it is built on first open instead of at shell start on every monitor, and it scrolls when the screen is too small.
- Links open through `omarchy-launch-browser` (fallback `xdg-open`), detached.
- Standalone window: Esc/Q close, R refresh, Shift+R re-scan, C copy archetype.
- About overlay text is English like the rest of the UI; the self-declared "VERIFIED" badge is replaced by the version.
- CI fails on a bare `Color.` palette reference in QML.

## 1.4.0

Works with OmaStat 1.1.0. Older OmaStat servers still accept the payload; they ignore `install_token` and have no rank API, so the rank stays an estimate.

### Fixed

- **Survey reported success on failure.** `--submit-survey` treated any HTTP 200 as success. It now reads the JSON `success` field and only saves the "submitted" state when the server accepted the data.
- **Fake RAM details.** When module details need root, RAM used to show a made-up DDR4-3200 kit and got points for it. It now shows `Unknown` (0 MT/s, 0 modules) and scores nothing for the unknown parts.
- **Integrated GPUs scored like discrete cards.** AMD APUs and Intel iGPUs fell through to the generic amdgpu (65) or xe (78) scores. They are now detected and scored by class.
- **NVIDIA driver fallback never matched**, because nvidia-smi reports the driver as `nvidia <version>`.
- **Different grade scale than OmaStat.** The engine and the server now share the tier letters S+ S A B C D E F with the same thresholds; `grade` is in `--json`, `--status`, the banner and the panel.
- **The world rank was invented** from a hard-coded 12,480 machines. It now comes from OmaStat's rank API (only after you opt in to the survey) and is labelled "estimate" otherwise (`rank_is_estimate`). The ladder no longer prints the fake per-tier rank spans.
- **Multi-byte names were cut mid-character.** `clean_string` truncation is now UTF-8 safe.
- **CPU clock read the current frequency** instead of the maximum (cpufreq `cpuinfo_max_freq`).
- **HDDs and removable drives were scored as SSDs.**
- **Ladder tier A** had the wrong upper bound.
- **Dashboard toggle did nothing.** `omarank-dashboard` used `DIR` before defining it and called the wrong IPC target; it now uses `quickshell ipc -p shell.qml call ozdil.omarank.window toggle` and falls back to `xdg-terminal-exec`.
- **Dashboard window was killed after 5 seconds** by the panel's launch timeout; it now starts detached.
- **Panel kept polling while closed.** The refresh timer only runs while the panel is open, and it refreshes when opened.
- **Panel shortcuts called functions that do not exist** (C, S, M, B). C copies the archetype, D opens the dashboard, L opens the web leaderboard. Survey submission has no single-key shortcut on purpose.
- **Status and dashboard scripts lost the environment** in their `env -i` re-exec, so the bar could not see the opt-in state, `OMASTAT_URL`, proxies or the Hyprland socket.
- **Root-owned installs** (PKGBUILD) were rejected by the scripts' ownership checks; packaged installs are recognised.
- **PKGBUILD did not install `qml/`.**

### Added

- `--survey-preview` and a "Preview data" toggle in the panel show exactly what would be sent.
- Anonymous `install_token` in the survey: a hash of the machine id and the current month (`sha256("omarank-survey-v1:<machine-id>:<YYYY-MM>")`, first 32 hex chars). It changes every month and cannot be reversed; OmaStat uses it to count each machine once per month.
- `system_vendor` and `product_name` (DMI) in the survey for better OEM and laptop names.
- `gpu_vram_mb` and `gpu_integrated` in `--json`.
- GPU entries for RX 9070 / 9060, RTX 5060 and Ryzen AI archetypes.
- The discrete GPU score table is data (`src/gpu_scores.json`) and can be overridden with `$XDG_CONFIG_HOME/omarank/gpu_scores.json` (same ownership and symlink checks as the survey state).
- Hardware detection is cached per boot (`hardware.json`, 1 hour, invalidated on upgrade); `--rescan` forces a fresh scan. The panel's re-check button and the R key use it.
- `OMASTAT_URL` points the engine at another OmaStat server (https only, or http on localhost).

### Changed

- The survey body is sent to curl over stdin (no longer visible in `ps`), only proxy variables are passed through, and the workers.dev endpoint is tried first.
- AUR-style PKGBUILD: release tarball source, `cargo fetch --locked` in `prepare()`, `--frozen` build, `check()` runs the tests. Needs a `v1.4.0` tag before it builds.
- CI: `cargo fmt --check`, clippy with `-D warnings`, tests, script syntax and `scripts/check-version.sh` (all version fields must match); CodeQL scans Rust and Actions.
- The engine sources are formatted with `cargo fmt`.
