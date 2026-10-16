import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import Quickshell
import Quickshell.Io
import "theme"

Item {
    id: root

    property int totalScore: 0
    property int globalRank: 0
    property int totalMachines: 0
    property bool rankIsEstimate: true
    property string grade: ""
    property string tierName: "Unranked"
    property string tierQuote: ""
    property color tierColor: Theme.accent
    property string percentileText: ""
    property string battlestationId: ""
    property string archetypeSignature: ""
    property var subScores: ({})
    property var hardware: ({})
    property bool loading: true
    property string toastText: ""

    readonly property string enginePath: {
        var base = Qt.resolvedUrl(".").toString().replace(/^file:\/\//, "");
        var parent = base.replace(/\/qml\/?$/, "");
        return parent + "/omarank-engine";
    }

    // Group digits with commas regardless of system locale (the UI is English)
    function fmtInt(n) {
        return String(Math.round(Number(n) || 0)).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
    }

    function refresh() {
        if (!rankProc.running) {
            root.loading = true;
            rankProc.running = true;
        }
    }

    function rescan() {
        root.loading = true;
        rescanProc.command = [root.enginePath, "--rescan"];
        rescanProc.running = true;
    }

    function copy(label, value) {
        if (!value) return;
        copyProc.command = ["wl-copy", String(value)];
        copyProc.running = true;
        root.toastText = label + " copied";
        toastTimer.restart();
    }

    // Joins detail parts so a wrapped line only breaks between parts, never inside one
    function joinParts(parts) {
        return parts.filter(function(p) { return p && String(p).length > 0; })
            .map(function(p) { return String(p).replace(/ /g, "\u00a0"); })
            .join("  ·  ");
    }

    // ---- Hardware text helpers (shared wording with the bar panel) ----
    readonly property var hw: root.hardware || ({})

    readonly property string cpuValue: String(hw.cpu_name || "Unknown CPU")
        .replace(/\(R\)|\(TM\)/g, "").replace(/\s+/g, " ").trim()
    readonly property string cpuDetail: {
        var parts = [];
        if (hw.cpu_cores) parts.push(hw.cpu_cores + " cores / " + (hw.cpu_threads || hw.cpu_cores) + " threads");
        if (hw.cpu_mhz) parts.push((Number(hw.cpu_mhz) / 1000).toFixed(1) + " GHz boost");
        return root.joinParts(parts);
    }

    readonly property string gpuValue: String(hw.gpu_name || "Integrated Graphics")
        .replace(/NVIDIA Corporation |Advanced Micro Devices, Inc\. |Intel Corporation /g, "").trim()
    readonly property string gpuDetail: {
        var parts = [];
        if (hw.gpu_vram_mb) parts.push(Math.round(Number(hw.gpu_vram_mb) / 1024) + " GB VRAM");
        if (hw.gpu_integrated) parts.push("integrated");
        if (hw.gpu_driver) parts.push(hw.gpu_driver);
        return root.joinParts(parts);
    }

    readonly property string ramValue: (Number(hw.ram_total_gb) || 0).toFixed(1) + " GB " + (hw.ram_type || "RAM")
    readonly property string ramDetail: root.joinParts([
        hw.ram_speed_mts ? (hw.ram_speed_mts + " MT/s") : "",
        hw.ram_slots ? ((hw.ram_modules || 1) + " of " + hw.ram_slots + " slots") : ""
    ])

    readonly property string storageValue: String(hw.storage_model || hw.storage_type || "Unknown drive")
    readonly property string storageDetail: hw.storage_model ? String(hw.storage_type || "") : ""

    readonly property var monitors: hw.monitors || []
    readonly property string displayValue: {
        if (monitors.length === 0) return "No display detected";
        var m = monitors[0];
        return m.width + " × " + m.height + " @ " + Math.round(Number(m.refresh_rate) || 0) + " Hz";
    }
    readonly property string displayDetail: {
        if (monitors.length === 0) return "";
        var m = monitors[0];
        var parts = [m.name];
        if (m.scale && Number(m.scale) !== 1) parts.push("scale " + Number(m.scale).toFixed(2).replace(/\.?0+$/, ""));
        if (monitors.length > 1) parts.push("+" + (monitors.length - 1) + " more");
        return root.joinParts(parts);
    }

    readonly property string moboValue: {
        var vendor = String(hw.mobo_vendor || "").trim();
        var name = String(hw.mobo_name || "").trim();
        if (!name) return vendor || "Unknown platform";
        if (vendor && name.toLowerCase().indexOf(vendor.toLowerCase()) < 0) return vendor + " " + name;
        return name;
    }
    readonly property string moboDetail: {
        var parts = [];
        if (hw.chipset) parts.push(hw.chipset);
        if (hw.mobo_bios) parts.push("BIOS " + hw.mobo_bios);
        return root.joinParts(parts);
    }

    // Mirrors GRADE_SCALE in src/score.rs (lowest first)
    readonly property var gradeScale: [
        { grade: "F", min: 0, name: "Potato Toaster" },
        { grade: "E", min: 16, name: "Study Mode Only" },
        { grade: "D", min: 31, name: "Budget Warrior" },
        { grade: "C", min: 46, name: "Honest Daily Driver" },
        { grade: "B", min: 61, name: "Gaming Chair Missing" },
        { grade: "A", min: 76, name: "Cyberpunk Beast" },
        { grade: "S", min: 89, name: "NASA Supercomputer" },
        { grade: "S+", min: 96, name: "Cosmic Reality Simulator" }
    ]
    readonly property var nextGrade: {
        for (var i = 0; i < gradeScale.length; i++) {
            if (gradeScale[i].min > root.totalScore) return gradeScale[i];
        }
        return null;
    }

    readonly property string rankHeadline: {
        if (root.globalRank <= 0) return root.loading ? "Ranking…" : "Unranked";
        return (root.rankIsEstimate ? "~#" : "#") + fmtInt(root.globalRank)
            + " of " + fmtInt(root.totalMachines);
    }
    readonly property string percentileShort: String(root.percentileText)
        .replace(/\s*\((offline )?estimate\)\s*$/i, "").replace(/^Estimated\s+/i, "")

    Process {
        id: rankProc
        command: [root.enginePath, "--json"]
        stdout: StdioCollector {
            waitForEnd: true
            onStreamFinished: {
                try {
                    var data = JSON.parse(text || "{}");
                    root.totalScore = Number(data.total_score) || 0;
                    root.globalRank = Number(data.global_rank) || 0;
                    root.totalMachines = Number(data.total_machines) || 0;
                    root.rankIsEstimate = data.rank_is_estimate !== false;
                    root.grade = data.grade || "";
                    root.tierName = data.tier_name || "Unranked";
                    root.tierQuote = data.tier_quote || "";
                    root.tierColor = /^#[0-9a-fA-F]{6}$/.test(data.tier_color || "") ? data.tier_color : Theme.accent;
                    root.percentileText = data.percentile_text || "";
                    root.battlestationId = data.battlestation_id || "";
                    root.archetypeSignature = data.archetype_signature || "";
                    root.subScores = data.sub_scores || ({});
                    root.hardware = data.hardware || ({});
                } catch(e) {
                    console.warn("Failed to parse omarank json:", e);
                }
                root.loading = false;
            }
        }
    }

    Process {
        id: rescanProc
        onExited: { rankProc.running = false; root.refresh(); }
    }

    Process { id: copyProc }

    Timer { id: toastTimer; interval: 1600; onTriggered: root.toastText = "" }

    Component.onCompleted: refresh()

    // ---------------- Reusable pieces ----------------

    component SectionLabel: Text {
        font.family: Theme.fontFamily
        font.pixelSize: 10
        font.bold: true
        font.letterSpacing: 1.4
        color: Theme.textMuted
    }

    component Panel: Rectangle {
        radius: Theme.radiusMd
        color: Theme.bgSurface
        border.color: Theme.border
        border.width: 1
    }

    // One hardware component: fixed icon tile, label/value/detail stack, subscore on the right.
    // Everything sits in fixed columns so rows line up regardless of text length.
    component HardwareCard: Rectangle {
        id: card
        property string icon: ""
        property string label: ""
        property string value: ""
        property string detail: ""
        property int score: 0

        radius: Theme.radiusSm
        color: cardArea.containsMouse ? Theme.bgCardHover : Theme.bgCard
        border.color: cardArea.containsMouse ? Theme.borderLight : Theme.border
        border.width: 1
        Behavior on color { ColorAnimation { duration: 120 } }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 14
            anchors.rightMargin: 14
            spacing: 14

            Rectangle {
                Layout.preferredWidth: 40
                Layout.preferredHeight: 40
                Layout.alignment: Qt.AlignVCenter
                radius: Theme.radiusSm
                color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.12)

                Text {
                    anchors.centerIn: parent
                    text: card.icon
                    font.family: Theme.iconFont
                    font.pixelSize: 22
                    color: Theme.accent
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                spacing: 3

                Text {
                    Layout.fillWidth: true
                    text: card.label.toUpperCase()
                    font.family: Theme.fontFamily
                    font.pixelSize: 9
                    font.bold: true
                    font.letterSpacing: 1.2
                    color: Theme.textMuted
                    elide: Text.ElideRight
                }
                Text {
                    Layout.fillWidth: true
                    text: card.value
                    font.family: Theme.monoFont
                    font.pixelSize: 13
                    font.bold: true
                    color: Theme.textMain
                    wrapMode: Text.WordWrap
                    maximumLineCount: 2
                    elide: Text.ElideRight
                }
                Text {
                    Layout.fillWidth: true
                    visible: card.detail.length > 0
                    text: card.detail
                    font.family: Theme.monoFont
                    font.pixelSize: 10
                    color: Theme.textMuted
                    wrapMode: Text.WordWrap
                    maximumLineCount: 2
                    elide: Text.ElideRight
                }
            }

            ColumnLayout {
                Layout.preferredWidth: 38
                Layout.alignment: Qt.AlignVCenter
                spacing: 0
                Text {
                    Layout.alignment: Qt.AlignRight
                    text: card.score > 0 ? String(card.score) : "--"
                    font.family: Theme.monoFont
                    font.pixelSize: 18
                    font.bold: true
                    color: Theme.textMain
                }
                Text {
                    Layout.alignment: Qt.AlignRight
                    text: "/100"
                    font.family: Theme.monoFont
                    font.pixelSize: 9
                    color: Theme.textDim
                }
            }
        }

        MouseArea {
            id: cardArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: root.copy(card.label, card.value + (card.detail ? (" (" + card.detail + ")") : ""))
        }
    }

    // ---------------- Layout ----------------

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 14

        // Header
        Panel {
            Layout.fillWidth: true
            Layout.preferredHeight: 64

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 18
                anchors.rightMargin: 14
                spacing: 14

                Text {
                    text: Theme.iconRocket
                    font.family: Theme.iconFont
                    font.pixelSize: 22
                    color: Theme.accent
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2
                    Text {
                        text: "OMARANK"
                        font.family: Theme.fontFamily
                        font.pixelSize: 15
                        font.bold: true
                        font.letterSpacing: 2
                        color: Theme.textMain
                    }
                    Text {
                        Layout.fillWidth: true
                        text: [String(root.hw.system_vendor || ""), String(root.hw.product_name || ""), root.battlestationId]
                            .filter(function(s) { return s.length > 0; }).join("  ·  ") || "Silicon benchmark"
                        font.family: Theme.fontFamily
                        font.pixelSize: 11
                        color: Theme.textMuted
                        elide: Text.ElideRight
                    }
                }

                // Rank pill: real OmaStat rank, or a clearly labelled offline estimate
                Rectangle {
                    Layout.preferredHeight: 36
                    implicitWidth: rankRow.implicitWidth + 24
                    radius: Theme.radiusSm
                    color: Theme.bgCard
                    border.color: Theme.border
                    border.width: 1

                    RowLayout {
                        id: rankRow
                        anchors.centerIn: parent
                        spacing: 10

                        Text {
                            text: root.rankIsEstimate ? Theme.iconTrophy : Theme.iconGlobe
                            font.family: Theme.iconFont
                            font.pixelSize: 14
                            color: root.rankIsEstimate ? Theme.textMuted : Theme.accentWarning
                        }
                        ColumnLayout {
                            spacing: 0
                            Text {
                                text: root.rankHeadline
                                font.family: Theme.monoFont
                                font.pixelSize: 12
                                font.bold: true
                                color: Theme.textMain
                            }
                            Text {
                                text: (root.rankIsEstimate ? "OFFLINE ESTIMATE" : "OMASTAT WORLD RANK")
                                    + (root.percentileShort ? ("  ·  " + root.percentileShort.toUpperCase()) : "")
                                font.family: Theme.fontFamily
                                font.pixelSize: 8
                                font.bold: true
                                font.letterSpacing: 1
                                color: Theme.textMuted
                            }
                        }
                    }
                }

                // Rescan
                Rectangle {
                    Layout.preferredHeight: 36
                    implicitWidth: rescanRow.implicitWidth + 24
                    radius: Theme.radiusSm
                    color: rescanArea.containsMouse ? Theme.bgCardHover : Theme.bgCard
                    border.color: rescanArea.containsMouse ? Theme.borderLight : Theme.border
                    border.width: 1

                    RowLayout {
                        id: rescanRow
                        anchors.centerIn: parent
                        spacing: 8

                        Text {
                            text: Theme.iconRefresh
                            font.family: Theme.iconFont
                            font.pixelSize: 13
                            color: Theme.textMain
                            RotationAnimator on rotation {
                                running: root.loading
                                from: 0; to: 360; duration: 900
                                loops: Animation.Infinite
                                onRunningChanged: if (!running) parent.rotation = 0
                            }
                        }
                        Text {
                            text: root.loading ? "Scanning" : "Rescan"
                            font.family: Theme.fontFamily
                            font.pixelSize: 11
                            font.bold: true
                            color: Theme.textMain
                        }
                    }

                    MouseArea {
                        id: rescanArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        enabled: !root.loading
                        onClicked: root.rescan()
                    }
                }
            }
        }

        // Body
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 14

            // Left: score and subscores
            Panel {
                Layout.preferredWidth: root.width < 1000 ? 290 : 340
                Layout.fillHeight: true

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 18
                    spacing: 14

                    // Score ring
                    Item {
                        Layout.alignment: Qt.AlignHCenter
                        Layout.preferredWidth: root.height < 800 ? 148 : 168
                        Layout.preferredHeight: Layout.preferredWidth

                        Canvas {
                            id: ring
                            anchors.fill: parent
                            property real progress: Math.max(0, Math.min(100, root.totalScore)) / 100
                            property color trackColor: Theme.bgDark
                            property color fillColor: root.tierColor
                            Behavior on progress { NumberAnimation { duration: 600; easing.type: Easing.OutCubic } }
                            onProgressChanged: requestPaint()
                            onTrackColorChanged: requestPaint()
                            onFillColorChanged: requestPaint()
                            onPaint: {
                                var ctx = getContext("2d");
                                ctx.reset();
                                var cx = width / 2, cy = height / 2, r = Math.min(cx, cy) - 8;
                                var start = Math.PI * 0.75, sweep = Math.PI * 1.5;
                                ctx.lineWidth = 10;
                                ctx.lineCap = "round";
                                ctx.strokeStyle = trackColor;
                                ctx.beginPath();
                                ctx.arc(cx, cy, r, start, start + sweep);
                                ctx.stroke();
                                if (progress > 0) {
                                    ctx.strokeStyle = fillColor;
                                    ctx.beginPath();
                                    ctx.arc(cx, cy, r, start, start + sweep * progress);
                                    ctx.stroke();
                                }
                            }
                        }

                        ColumnLayout {
                            anchors.centerIn: parent
                            spacing: 0
                            Text {
                                Layout.alignment: Qt.AlignHCenter
                                text: root.totalScore > 0 ? String(root.totalScore) : "--"
                                font.family: Theme.monoFont
                                font.pixelSize: 46
                                font.bold: true
                                color: Theme.textMain
                            }
                            Text {
                                Layout.alignment: Qt.AlignHCenter
                                text: "OF 100"
                                font.family: Theme.fontFamily
                                font.pixelSize: 9
                                font.bold: true
                                font.letterSpacing: 1.6
                                color: Theme.textMuted
                            }
                        }

                        // Grade badge on the open bottom of the ring
                        Rectangle {
                            anchors.horizontalCenter: parent.horizontalCenter
                            anchors.bottom: parent.bottom
                            width: 46
                            height: 24
                            radius: 12
                            visible: root.grade.length > 0
                            color: Qt.rgba(root.tierColor.r, root.tierColor.g, root.tierColor.b, 0.18)
                            border.color: root.tierColor
                            border.width: 1
                            Text {
                                anchors.centerIn: parent
                                text: root.grade
                                font.family: Theme.monoFont
                                font.pixelSize: 12
                                font.bold: true
                                color: root.tierColor
                            }
                        }
                    }

                    Text {
                        Layout.fillWidth: true
                        text: root.tierName
                        font.family: Theme.fontFamily
                        font.pixelSize: 15
                        font.bold: true
                        color: root.tierColor
                        horizontalAlignment: Text.AlignHCenter
                        elide: Text.ElideRight
                    }

                    Text {
                        Layout.fillWidth: true
                        visible: text.length > 0
                        text: root.tierQuote
                        font.family: Theme.fontFamily
                        font.pixelSize: 11
                        font.italic: true
                        color: Theme.textMuted
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.WordWrap
                        maximumLineCount: 3
                        elide: Text.ElideRight
                    }

                    Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }

                    SectionLabel { text: "SUBSCORES" }

                    Repeater {
                        model: [
                            { name: "Processor", score: root.subScores.cpu || 0, icon: Theme.iconCpu },
                            { name: "Graphics", score: root.subScores.gpu || 0, icon: Theme.iconGpu },
                            { name: "Memory", score: root.subScores.ram || 0, icon: Theme.iconMemory },
                            { name: "Storage", score: root.subScores.storage || 0, icon: Theme.iconStorage },
                            { name: "Display", score: root.subScores.display || 0, icon: Theme.iconDisplay },
                            { name: "Motherboard", score: root.subScores.mobo || 0, icon: Theme.iconMobo }
                        ]

                        delegate: RowLayout {
                            Layout.fillWidth: true
                            spacing: 10

                            Text {
                                Layout.preferredWidth: 18
                                horizontalAlignment: Text.AlignHCenter
                                text: modelData.icon
                                font.family: Theme.iconFont
                                font.pixelSize: 14
                                color: Theme.accent
                            }
                            Text {
                                Layout.preferredWidth: 92
                                text: modelData.name
                                font.family: Theme.fontFamily
                                font.pixelSize: 11
                                color: Theme.textMain
                                elide: Text.ElideRight
                            }
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 6
                                radius: 3
                                color: Theme.bgDark

                                Rectangle {
                                    width: parent.width * (Math.max(0, Math.min(100, modelData.score)) / 100.0)
                                    height: parent.height
                                    radius: 3
                                    color: Theme.accent
                                    Behavior on width { NumberAnimation { duration: 500; easing.type: Easing.OutCubic } }
                                }
                            }
                            Text {
                                Layout.preferredWidth: 26
                                horizontalAlignment: Text.AlignRight
                                text: String(modelData.score)
                                font.family: Theme.monoFont
                                font.pixelSize: 11
                                font.bold: true
                                color: Theme.textMain
                            }
                        }
                    }

                    Item { Layout.fillHeight: true }

                    Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }

                    // Grade ladder: where this machine sits and what the next grade needs
                    RowLayout {
                        Layout.fillWidth: true
                        SectionLabel { text: "GRADE LADDER" }
                        Item { Layout.fillWidth: true }
                        Text {
                            text: root.totalScore <= 0 ? ""
                                : root.nextGrade
                                    ? ((root.nextGrade.min - root.totalScore) + " pts to " + root.nextGrade.grade + " · " + root.nextGrade.name)
                                    : "Top grade reached"
                            font.family: Theme.monoFont
                            font.pixelSize: 10
                            font.bold: true
                            color: Theme.textMuted
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 4
                        Repeater {
                            model: root.gradeScale
                            delegate: Rectangle {
                                readonly property bool current: modelData.grade === root.grade
                                readonly property bool reached: root.totalScore >= modelData.min
                                Layout.fillWidth: true
                                Layout.preferredHeight: 26
                                radius: Theme.radiusSm
                                color: current ? Qt.rgba(root.tierColor.r, root.tierColor.g, root.tierColor.b, 0.2)
                                     : reached ? Theme.bgCard : "transparent"
                                border.color: current ? root.tierColor : Theme.border
                                border.width: 1
                                Text {
                                    anchors.centerIn: parent
                                    text: modelData.grade
                                    font.family: Theme.monoFont
                                    font.pixelSize: 11
                                    font.bold: parent.current
                                    color: parent.current ? root.tierColor : (parent.reached ? Theme.textMain : Theme.textDim)
                                }
                            }
                        }
                    }

                }
            }

            // Right: hardware grid and community strip
            Panel {
                Layout.fillWidth: true
                Layout.fillHeight: true

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 18
                    spacing: 12

                    RowLayout {
                        Layout.fillWidth: true
                        SectionLabel { text: "HARDWARE" }
                        Item { Layout.fillWidth: true }
                        Text {
                            text: root.toastText || "Click a card to copy"
                            font.family: Theme.fontFamily
                            font.pixelSize: 10
                            color: root.toastText ? Theme.accentSuccess : Theme.textDim
                        }
                    }

                    GridLayout {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        // Two columns only when each card still has room for its text
                        columns: width >= 620 ? 2 : 1
                        rowSpacing: 10
                        columnSpacing: 10
                        uniformCellWidths: true
                        uniformCellHeights: true

                        HardwareCard {
                            Layout.fillWidth: true; Layout.fillHeight: true; Layout.minimumHeight: 64; Layout.maximumHeight: 150
                            icon: Theme.iconCpu; label: "Processor"
                            value: root.cpuValue; detail: root.cpuDetail; score: root.subScores.cpu || 0
                        }
                        HardwareCard {
                            Layout.fillWidth: true; Layout.fillHeight: true; Layout.minimumHeight: 64; Layout.maximumHeight: 150
                            icon: Theme.iconGpu; label: "Graphics"
                            value: root.gpuValue; detail: root.gpuDetail; score: root.subScores.gpu || 0
                        }
                        HardwareCard {
                            Layout.fillWidth: true; Layout.fillHeight: true; Layout.minimumHeight: 64; Layout.maximumHeight: 150
                            icon: Theme.iconMemory; label: "Memory"
                            value: root.ramValue; detail: root.ramDetail; score: root.subScores.ram || 0
                        }
                        HardwareCard {
                            Layout.fillWidth: true; Layout.fillHeight: true; Layout.minimumHeight: 64; Layout.maximumHeight: 150
                            icon: Theme.iconStorage; label: "Storage"
                            value: root.storageValue; detail: root.storageDetail; score: root.subScores.storage || 0
                        }
                        HardwareCard {
                            Layout.fillWidth: true; Layout.fillHeight: true; Layout.minimumHeight: 64; Layout.maximumHeight: 150
                            icon: Theme.iconDisplay; label: "Display"
                            value: root.displayValue; detail: root.displayDetail; score: root.subScores.display || 0
                        }
                        HardwareCard {
                            Layout.fillWidth: true; Layout.fillHeight: true; Layout.minimumHeight: 64; Layout.maximumHeight: 150
                            icon: Theme.iconMobo; label: "Motherboard"
                            value: root.moboValue; detail: root.moboDetail; score: root.subScores.mobo || 0
                        }
                    }

                    // Community strip: identity used by OmaStat, plus how to get a real rank
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: communityRow.implicitHeight + 24
                        radius: Theme.radiusSm
                        color: Theme.bgDark
                        border.color: Theme.border
                        border.width: 1

                        ColumnLayout {
                            id: communityRow
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.leftMargin: 14
                            anchors.rightMargin: 14
                            spacing: 10

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 24

                                ColumnLayout {
                                    spacing: 2
                                    SectionLabel { text: "ARCHETYPE"; font.pixelSize: 8 }
                                    Text {
                                        text: root.archetypeSignature || "--"
                                        font.family: Theme.monoFont
                                        font.pixelSize: 12
                                        font.bold: true
                                        color: Theme.textMain
                                        MouseArea {
                                            anchors.fill: parent
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: root.copy("Archetype", root.archetypeSignature)
                                        }
                                    }
                                }

                                ColumnLayout {
                                    spacing: 2
                                    SectionLabel { text: "OS"; font.pixelSize: 8 }
                                    Text {
                                        text: root.hw.os_name || "Omarchy"
                                        font.family: Theme.monoFont
                                        font.pixelSize: 12
                                        font.bold: true
                                        color: Theme.textMain
                                    }
                                }

                                Item { Layout.fillWidth: true }
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 8
                                Text {
                                    text: Theme.iconInfo
                                    font.family: Theme.iconFont
                                    font.pixelSize: 13
                                    color: Theme.textMuted
                                }
                                Text {
                                    Layout.fillWidth: true
                                    text: root.rankIsEstimate
                                        ? "Offline estimate. The real world rank needs survey opt-in and a reachable OmaStat server."
                                        : "World rank from OmaStat, refreshed daily."
                                    font.family: Theme.fontFamily
                                    font.pixelSize: 10
                                    color: Theme.textMuted
                                    wrapMode: Text.WordWrap
                                    maximumLineCount: 2
                                    elide: Text.ElideRight
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
