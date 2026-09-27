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
    property string tierName: "Unranked"
    property string tierQuote: ""
    property string percentileText: ""
    property string battlestationId: ""
    property var subScores: ({})
    property var hardware: ({})

    readonly property string enginePath: {
        var base = Qt.resolvedUrl(".").toString().replace(/^file:\/\//, "");
        var parent = base.replace(/\/qml\/?$/, "");
        return parent + "/omarank-engine";
    }

    function refresh() {
        if (!rankProc.running) {
            rankProc.running = true;
        }
    }

    function rescan() {
        rescanProc.command = [root.enginePath, "--rescan"];
        rescanProc.running = true;
    }

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
                    root.tierName = data.tier_name || "Unranked";
                    root.tierQuote = data.tier_quote || "";
                    root.percentileText = data.percentile_text || "";
                    root.battlestationId = data.battlestation_id || "";
                    root.subScores = data.sub_scores || ({});
                    root.hardware = data.hardware || ({});
                } catch(e) {
                    console.warn("Failed to parse omarank json:", e);
                }
            }
        }
    }

    Process {
        id: rescanProc
        onExited: root.refresh()
    }

    Component.onCompleted: refresh()

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 14

        // Header Bar
        Rectangle {
            Layout.fillWidth: true
            height: 68
            radius: Theme.radiusMd
            color: Theme.bgSurface
            border.color: Theme.border
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 16
                anchors.rightMargin: 16
                spacing: 12

                Text {
                    text: Theme.iconRocket
                    font.family: Theme.iconFont
                    font.pixelSize: 24
                    color: Theme.accent
                }

                ColumnLayout {
                    spacing: 2
                    Text {
                        text: "OMARANK SILICON BENCHMARK"
                        font.family: Theme.fontFamily
                        font.pixelSize: 15
                        font.bold: true
                        color: Theme.textMain
                    }
                    Text {
                        text: "Hardware Scoring & Global Battlestation Telemetry • " + (root.battlestationId || "OMA-STATION")
                        font.family: Theme.fontFamily
                        font.pixelSize: 11
                        color: Theme.textMuted
                    }
                }

                Item { Layout.fillWidth: true }

                // Global Rank Pill
                Rectangle {
                    height: 32
                    implicitWidth: rankRow.implicitWidth + 20
                    radius: Theme.radiusSm
                    color: Theme.bgCard
                    border.color: Theme.border
                    border.width: 1

                    RowLayout {
                        id: rankRow
                        anchors.centerIn: parent
                        spacing: 8

                        Text {
                            text: Theme.iconTrophy
                            font.family: Theme.iconFont
                            font.pixelSize: 12
                            color: Theme.accentWarning
                        }

                        Text {
                            text: root.percentileText ? root.percentileText : "Rank Calculating..."
                            font.family: Theme.monoFont
                            font.pixelSize: 11
                            font.bold: true
                            color: Theme.textMain
                        }
                    }
                }

                // Rescan Button
                Rectangle {
                    height: 34
                    implicitWidth: rescanRow.implicitWidth + 20
                    radius: Theme.radiusSm
                    color: rescanArea.containsMouse ? Theme.bgCardHover : Theme.bgCard
                    border.color: Theme.border
                    border.width: 1

                    RowLayout {
                        id: rescanRow
                        anchors.centerIn: parent
                        spacing: 6

                        Text {
                            text: Theme.iconRefresh
                            font.family: Theme.iconFont
                            font.pixelSize: 12
                            color: Theme.textMain
                        }
                        Text {
                            text: "Rescan"
                            font.family: Theme.fontFamily
                            font.pixelSize: 11
                            color: Theme.textMain
                        }
                    }

                    MouseArea {
                        id: rescanArea
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.rescan()
                    }
                }
            }
        }

        // Main Body: Scores & Hardware Cards
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 14

            // Left Column: Tier & Subscores
            Rectangle {
                Layout.preferredWidth: 380
                Layout.fillHeight: true
                radius: Theme.radiusMd
                color: Theme.bgSurface
                border.color: Theme.border
                border.width: 1

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 16
                    spacing: 14

                    // Score Big Gauge
                    Rectangle {
                        Layout.fillWidth: true
                        height: 120
                        radius: Theme.radiusSm
                        color: Theme.bgCard
                        border.color: Theme.borderLight
                        border.width: 1

                        ColumnLayout {
                            anchors.centerIn: parent
                            spacing: 4

                            RowLayout {
                                Layout.alignment: Qt.AlignHCenter
                                spacing: 4
                                Text {
                                    text: String(root.totalScore)
                                    font.family: Theme.monoFont
                                    font.pixelSize: 42
                                    font.bold: true
                                    color: Theme.textMain
                                }
                                Text {
                                    text: "/ 100"
                                    font.family: Theme.monoFont
                                    font.pixelSize: 16
                                    color: Theme.textMuted
                                    Layout.alignment: Qt.AlignBaseline
                                }
                            }

                            Text {
                                Layout.alignment: Qt.AlignHCenter
                                text: root.tierName
                                font.family: Theme.fontFamily
                                font.pixelSize: 13
                                font.bold: true
                                color: Theme.accent
                            }
                        }
                    }

                    // Tier Quote
                    Text {
                        Layout.fillWidth: true
                        text: root.tierQuote || ""
                        font.family: Theme.fontFamily
                        font.pixelSize: 11
                        color: Theme.textMuted
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.WordWrap
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        height: 1
                        color: Theme.border
                    }

                    Text {
                        text: "SILICON PERFORMANCE SUBSCORES"
                        font.family: Theme.fontFamily
                        font.pixelSize: 11
                        font.bold: true
                        font.letterSpacing: 1.0
                        color: Theme.textMuted
                    }

                    // Subscore bars
                    Repeater {
                        model: [
                            { key: "cpu", name: "Processor (Compute)", score: root.subScores.cpu || 0, icon: Theme.iconCpu },
                            { key: "gpu", name: "Graphics (Raster/RT)", score: root.subScores.gpu || 0, icon: Theme.iconGpu },
                            { key: "ram", name: "Memory (Bandwidth)", score: root.subScores.ram || 0, icon: Theme.iconMemory },
                            { key: "storage", name: "Storage (I/O Throughput)", score: root.subScores.storage || 0, icon: Theme.iconStorage },
                            { key: "display", name: "Display (Frame Fluidity)", score: root.subScores.display || 0, icon: Theme.iconDisplay },
                            { key: "mobo", name: "Motherboard & Platform", score: root.subScores.mobo || 0, icon: Theme.iconMobo }
                        ]

                        delegate: ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 4

                            RowLayout {
                                Layout.fillWidth: true
                                Text {
                                    text: modelData.icon
                                    font.family: Theme.iconFont
                                    font.pixelSize: 11
                                    color: Theme.accent
                                }
                                Text {
                                    Layout.fillWidth: true
                                    text: modelData.name
                                    font.family: Theme.fontFamily
                                    font.pixelSize: 11
                                    color: Theme.textMain
                                }
                                Text {
                                    text: modelData.score + "/100"
                                    font.family: Theme.monoFont
                                    font.pixelSize: 11
                                    color: Theme.textMuted
                                }
                            }

                            // Progress Track
                            Rectangle {
                                Layout.fillWidth: true
                                height: 6
                                radius: 3
                                color: Theme.bgDark

                                Rectangle {
                                    width: parent.width * (Math.max(0, Math.min(100, modelData.score)) / 100.0)
                                    height: parent.height
                                    radius: 3
                                    color: Theme.accent
                                }
                            }
                        }
                    }

                    Item { Layout.fillHeight: true }
                }
            }

            // Right Column: Hardware Telemetry
            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                radius: Theme.radiusMd
                color: Theme.bgSurface
                border.color: Theme.border
                border.width: 1

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 16
                    spacing: 12

                    Text {
                        text: "IDENTIFIED HARDWARE CONFIGURATION"
                        font.family: Theme.fontFamily
                        font.pixelSize: 11
                        font.bold: true
                        font.letterSpacing: 1.0
                        color: Theme.textMuted
                    }

                    // Hardware Items Grid
                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        spacing: 10

                        // CPU Card
                        Rectangle {
                            Layout.fillWidth: true
                            height: 64
                            radius: Theme.radiusSm
                            color: Theme.bgCard
                            border.color: Theme.border
                            border.width: 1

                            RowLayout {
                                anchors.fill: parent
                                anchors.margins: 12
                                spacing: 12

                                Text { text: Theme.iconCpu; font.family: Theme.iconFont; font.pixelSize: 18; color: Theme.accent }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Text { text: "Central Processing Unit"; font.family: Theme.fontFamily; font.pixelSize: 10; color: Theme.textMuted }
                                    Text { text: root.hardware.cpu_name || "Unknown CPU"; font.family: Theme.monoFont; font.pixelSize: 12; font.bold: true; color: Theme.textMain; elide: Text.ElideRight }
                                }
                                Text {
                                    text: (root.hardware.cpu_cores || 0) + "C / " + (root.hardware.cpu_threads || 0) + "T"
                                    font.family: Theme.monoFont
                                    font.pixelSize: 11
                                    color: Theme.textMuted
                                }
                            }
                        }

                        // GPU Card
                        Rectangle {
                            Layout.fillWidth: true
                            height: 64
                            radius: Theme.radiusSm
                            color: Theme.bgCard
                            border.color: Theme.border
                            border.width: 1

                            RowLayout {
                                anchors.fill: parent
                                anchors.margins: 12
                                spacing: 12

                                Text { text: Theme.iconGpu; font.family: Theme.iconFont; font.pixelSize: 18; color: Theme.accent }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Text { text: "Graphics Processor"; font.family: Theme.fontFamily; font.pixelSize: 10; color: Theme.textMuted }
                                    Text { text: root.hardware.gpu_name || "Integrated Graphics"; font.family: Theme.monoFont; font.pixelSize: 12; font.bold: true; color: Theme.textMain; elide: Text.ElideRight }
                                }
                                Text {
                                    text: root.hardware.gpu_driver || ""
                                    font.family: Theme.monoFont
                                    font.pixelSize: 10
                                    color: Theme.textMuted
                                }
                            }
                        }

                        // RAM Card
                        Rectangle {
                            Layout.fillWidth: true
                            height: 64
                            radius: Theme.radiusSm
                            color: Theme.bgCard
                            border.color: Theme.border
                            border.width: 1

                            RowLayout {
                                anchors.fill: parent
                                anchors.margins: 12
                                spacing: 12

                                Text { text: Theme.iconMemory; font.family: Theme.iconFont; font.pixelSize: 18; color: Theme.accent }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Text { text: "System Memory"; font.family: Theme.fontFamily; font.pixelSize: 10; color: Theme.textMuted }
                                    Text {
                                        text: (root.hardware.ram_total_gb || 0) + " GB " + (root.hardware.ram_type || "DDR") + " @" + (root.hardware.ram_speed_mts || "") + " MT/s"
                                        font.family: Theme.monoFont
                                        font.pixelSize: 12
                                        font.bold: true
                                        color: Theme.textMain
                                    }
                                }
                                Text {
                                    text: (root.hardware.ram_modules || 1) + "/" + (root.hardware.ram_slots || 2) + " Slots"
                                    font.family: Theme.monoFont
                                    font.pixelSize: 11
                                    color: Theme.textMuted
                                }
                            }
                        }

                        // Storage Card
                        Rectangle {
                            Layout.fillWidth: true
                            height: 64
                            radius: Theme.radiusSm
                            color: Theme.bgCard
                            border.color: Theme.border
                            border.width: 1

                            RowLayout {
                                anchors.fill: parent
                                anchors.margins: 12
                                spacing: 12

                                Text { text: Theme.iconStorage; font.family: Theme.iconFont; font.pixelSize: 18; color: Theme.accent }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Text { text: "Primary Storage Subsystem"; font.family: Theme.fontFamily; font.pixelSize: 10; color: Theme.textMuted }
                                    Text { text: root.hardware.storage_model || "Solid State Drive"; font.family: Theme.monoFont; font.pixelSize: 12; font.bold: true; color: Theme.textMain; elide: Text.ElideRight }
                                }
                                Text {
                                    text: root.hardware.storage_type || ""
                                    font.family: Theme.monoFont
                                    font.pixelSize: 10
                                    color: Theme.textMuted
                                }
                            }
                        }

                        // Motherboard & Platform
                        Rectangle {
                            Layout.fillWidth: true
                            height: 64
                            radius: Theme.radiusSm
                            color: Theme.bgCard
                            border.color: Theme.border
                            border.width: 1

                            RowLayout {
                                anchors.fill: parent
                                anchors.margins: 12
                                spacing: 12

                                Text { text: Theme.iconMobo; font.family: Theme.iconFont; font.pixelSize: 18; color: Theme.accent }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2
                                    Text { text: "Motherboard & Firmware"; font.family: Theme.fontFamily; font.pixelSize: 10; color: Theme.textMuted }
                                    Text {
                                        text: (root.hardware.mobo_vendor ? (root.hardware.mobo_vendor + " ") : "") + (root.hardware.mobo_name || "Platform")
                                        font.family: Theme.monoFont
                                        font.pixelSize: 12
                                        font.bold: true
                                        color: Theme.textMain
                                        elide: Text.ElideRight
                                    }
                                }
                                Text {
                                    text: "BIOS: " + (root.hardware.mobo_bios || "UEFI")
                                    font.family: Theme.monoFont
                                    font.pixelSize: 10
                                    color: Theme.textMuted
                                }
                            }
                        }

                        Item { Layout.fillHeight: true }
                    }
                }
            }
        }
    }
}
