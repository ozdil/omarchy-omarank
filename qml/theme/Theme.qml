pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

QtObject {
    id: root

    readonly property string homeDir: Quickshell.env("HOME")
    readonly property string omarchyStateDir: homeDir + "/.local/state/omarchy/current"

    property string themeName: "monotone"
    property bool isDarkTheme: true

    // Dynamic Color Palette
    property color bgDark: "#0a0a0a"
    property color bgBase: "#111111"
    property color bgSurface: "#1a1a1a"
    property color bgCard: "#222222"
    property color bgCardHover: "#2c2c2c"
    property color border: "#333333"
    property color borderLight: "#444444"

    property color textMain: "#e5e5e5"
    property color textMuted: "#888888"
    property color textDim: "#555555"

    property color accent: "#b8b8b8"
    property color accentHover: "#cccccc"
    property color accentSuccess: "#a6da95"
    property color accentWarning: "#eed49f"
    property color accentDanger: "#ed8796"

    // Corner radius mirrors Hyprland's decoration:rounding, exactly like the Omarchy shell's
    // Style.cornerRadius (0 = Omarchy's default square corners). Read once at startup.
    property int cornerRadius: 0
    readonly property int radiusSm: cornerRadius
    readonly property int radiusMd: cornerRadius
    readonly property int radiusLg: cornerRadius

    // `omarchy toggle animations` turns Hyprland animations off; honour it as reduced motion.
    property bool reduceMotion: false
    function duration(ms) { return reduceMotion ? 0 : ms }

    // Omarchy's shell font is the fontconfig `monospace` alias, set by `omarchy font set`
    // (a Nerd Font by default, so the icon glyphs below resolve through it).
    readonly property string fontFamily: "monospace"
    readonly property string monoFont: "monospace"
    readonly property string iconFont: "monospace"

    // Themeable Monochrome Icons (Unicode Font Glyph Standard - Zero Emoji Policy).
    // Material Design glyphs live above U+FFFF, so they are built with fromCodePoint;
    // every glyph here exists in JetBrainsMono Nerd Font v3.
    readonly property string iconRocket: "\uf135"
    readonly property string iconCpu: String.fromCodePoint(0xF0EE0)      // md-cpu_64_bit
    readonly property string iconGpu: String.fromCodePoint(0xF08AE)      // md-expansion_card
    readonly property string iconMemory: "\uefc5"                        // fa-memory (RAM stick)
    readonly property string iconStorage: String.fromCodePoint(0xF02CA)  // md-harddisk
    readonly property string iconDisplay: String.fromCodePoint(0xF0379)  // md-monitor
    readonly property string iconMobo: String.fromCodePoint(0xF0697)     // md-developer_board
    readonly property string iconTrophy: "\uf091"
    readonly property string iconRefresh: "\uf021"
    readonly property string iconCopy: "\uf0c5"
    readonly property string iconCheck: "\uf00c"
    readonly property string iconGlobe: String.fromCodePoint(0xF01E7)    // md-earth
    readonly property string iconInfo: String.fromCodePoint(0xF02FC)     // md-information

    property string lastLoadedRaw: ""

    function loadColors(raw) {
        if (!raw || raw.trim().length === 0 || raw === lastLoadedRaw) return
        lastLoadedRaw = raw

        var dict = {}
        var lines = String(raw).split("\n")
        for (var i = 0; i < lines.length; i++) {
            var line = lines[i].trim()
            if (!line || line.charAt(0) === '#') continue
            var match = line.match(/^([A-Za-z0-9_-]+)\s*=\s*["']?([^"'\r\n]+?)["']?\s*(?:#.*)?$/)
            if (match) {
                dict[match[1].toLowerCase()] = match[2].trim()
            }
        }

        var mode = dict["mode"] || "dark"
        root.isDarkTheme = (mode !== "light")

        var base = dict["background"] || dict["bg"] || (root.isDarkTheme ? "#111111" : "#f5f5f5")
        var fg = dict["foreground"] || dict["fg"] || (root.isDarkTheme ? "#e5e5e5" : "#1a1a1a")
        var acc = dict["accent"] || dict["color4"] || dict["color6"] || "#b8b8b8"
        var sel = dict["selection"] || dict["selection_background"] || ""
        var mut = dict["muted"] || dict["color8"] || ""
        // colors.toml values are strings; component access (.r/.g/.b) needs real colors
        var fgc = Qt.color(fg)
        var accc = Qt.color(acc)

        root.bgBase = base

        if (root.isDarkTheme) {
            root.bgDark = dict["dark_background"] || Qt.darker(base, 1.25)
            root.bgSurface = dict["lighter_background"] || (sel ? sel : Qt.lighter(base, 1.3))
            root.bgCard = (sel && sel !== base) ? sel : Qt.lighter(base, 1.5)
            root.bgCardHover = Qt.lighter(root.bgCard, 1.15)
            root.border = mut ? mut : Qt.rgba(fgc.r, fgc.g, fgc.b, 0.2)
            root.borderLight = Qt.rgba(accc.r, accc.g, accc.b, 0.35)
            root.textMain = dict["bright_foreground"] || fg
            root.textMuted = dict["light_foreground"] || mut || Qt.rgba(fgc.r, fgc.g, fgc.b, 0.65)
            root.textDim = dict["dark_foreground"] || dict["color8"] || Qt.rgba(fgc.r, fgc.g, fgc.b, 0.4)
        } else {
            root.bgDark = Qt.darker(base, 1.08)
            root.bgSurface = Qt.lighter(base, 1.02)
            root.bgCard = Qt.darker(base, 1.04)
            root.bgCardHover = Qt.darker(root.bgCard, 1.06)
            root.border = mut ? mut : Qt.rgba(fgc.r, fgc.g, fgc.b, 0.2)
            root.borderLight = Qt.rgba(accc.r, accc.g, accc.b, 0.35)
            root.textMain = dict["bright_foreground"] || fg
            root.textMuted = dict["light_foreground"] || mut || Qt.rgba(fgc.r, fgc.g, fgc.b, 0.65)
            root.textDim = Qt.rgba(fgc.r, fgc.g, fgc.b, 0.4)
        }

        root.accent = acc
        root.accentHover = Qt.lighter(acc, 1.15)
        root.accentSuccess = dict["bright_green"] || dict["green"] || "#a6da95"
        root.accentWarning = dict["yellow"] || dict["bright_yellow"] || "#eed49f"
        root.accentDanger = dict["red"] || dict["color1"] || dict["bright_red"] || "#ed8796"
    }

    function applyHyprOption(raw, apply) {
        try {
            var json = JSON.parse(raw || "{}")
            var n = Number(json.int)
            if (isFinite(n) && n >= 0) apply(n)
        } catch (e) {
            // hyprctl missing / not running under Hyprland: keep the defaults
        }
    }

    property Process roundingProc: Process {
        command: ["hyprctl", "-j", "getoption", "decoration:rounding"]
        running: Quickshell.env("HYPRLAND_INSTANCE_SIGNATURE") !== ""
        stdout: StdioCollector {
            onStreamFinished: root.applyHyprOption(text, function(n) { root.cornerRadius = Math.round(n) })
        }
    }

    property Process animationsProc: Process {
        command: ["hyprctl", "-j", "getoption", "animations:enabled"]
        running: Quickshell.env("HYPRLAND_INSTANCE_SIGNATURE") !== ""
        stdout: StdioCollector {
            onStreamFinished: root.applyHyprOption(text, function(n) { root.reduceMotion = (n === 0) })
        }
    }

    property FileView colorsFile: FileView {
        id: colorsFile
        path: root.omarchyStateDir + "/theme/colors.toml"
        watchChanges: true
        printErrors: false
        onLoaded: root.loadColors(text())
        onFileChanged: reload()
    }

    property FileView themeNameFile: FileView {
        id: themeNameFile
        path: root.omarchyStateDir + "/theme.name"
        watchChanges: true
        printErrors: false
        onLoaded: {
            var n = text().trim()
            if (n.length > 0 && n !== root.themeName) {
                root.themeName = n
                root.lastLoadedRaw = ""
                colorsFile.reload()
            }
        }
        onFileChanged: {
            reload()
            root.lastLoadedRaw = ""
            colorsFile.reload()
        }
    }
}
