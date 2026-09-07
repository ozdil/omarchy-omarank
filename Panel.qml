import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Ui

Panel {
  id: root
  moduleName: "ozdil.omarank"
  ipcTarget: "ozdil.omarank"
  manageIpc: false

  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  property int totalScore: 0
  property int globalRank: 0
  property int totalMachines: 12480
  property string tierName: "Benchmarking..."
  property string tierIcon: ""
  property string rankNerdIcon: ""
  property string tierColor: "#38bdf8"
  readonly property color tierColorObj: Qt.color(root.tierColor)
  property string tierQuote: "Analyzing silicon architecture..."
  property string percentileText: "CALCULATING PERCENTILE..."

  property int cpuScore: 0
  property int gpuScore: 0
  property int ramScore: 0
  property int moboScore: 0
  property int displayScore: 0
  property int storageScore: 0

  property string cpuDesc: "--"
  property string gpuDesc: "--"
  property string ramDesc: "--"
  property string moboDesc: "--"
  property string displayDesc: "--"
  property string storageDesc: "--"
  property string worldRankDesc: "--"
  property string osDesc: "Omarchy Linux"
  property string battlestationId: "OMA-????"
  property string archetypeSignature: "OMA-BUILD"

  property string surveyStatusMsg: ""
  property bool isSubmittingSurvey: false

  function rankNerdIconFor(score) {
    if (score >= 96) return "" // Trophy
    if (score >= 89) return "󰓅" // Speedometer
    if (score >= 76) return "" // Rocket
    if (score >= 61) return "" // Gamepad
    if (score >= 46) return "" // Monitor
    if (score >= 31) return "" // Chip
    if (score >= 16) return "" // Laptop
    return "" // Alert
  }

  function cleanSanitized(str, maxLen) {
    if (!str) return ""
    var s = String(str).replace(/[\x00-\x1f\x7f-\x9f<>&`'"\\]/g, "").trim()
    return s.slice(0, maxLen || 80)
  }

  function resolveEnginePath() {
    return Qt.resolvedUrl("omarank-engine").toString().replace(/^file:\/\//, "")
  }

  function refresh() {
    if (!statusProc.running) {
      statusProc.running = true
    }
  }

  property string copyToastMsg: ""

  function copyToClipboard(val) {
    if (!val) return
    copyProc.command = ["wl-copy", String(val)]
    copyProc.running = true
    root.copyToastMsg = "Copied spec to clipboard"
    copyToastTimer.restart()
  }

  function launchDashboard() {
    root.close()
    var dashPath = Qt.resolvedUrl("omarank-dashboard").toString().replace(/^file:\/\//, "")
    launchProc.command = ["omarchy-launch-floating-terminal-with-presentation", dashPath]
    launchDeadlineTimer.restart()
    launchProc.running = true
  }

  function triggerSurvey() {
    if (root.isSubmittingSurvey) return
    root.isSubmittingSurvey = true
    root.surveyStatusMsg = "Sending anonymous survey..."
    surveyProc.running = true
  }

  IpcHandler {
    target: "ozdil.omarank"

    function open() { root.open() }
    function close() { root.close() }
    function show() { root.open() }
    function hide() { root.close() }
    function toggle() { root.toggle() }
    function refresh() { root.refresh() }
  }

  Process {
    id: statusProc
    command: [root.resolveEnginePath(), "--json"]
    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: {
        try {
          var cleanText = String(text || "").slice(0, 65536)
          var d = JSON.parse(cleanText)
          root.totalScore = Number(d.total_score) || 0
          root.globalRank = Number(d.global_rank) || 0
          root.totalMachines = Number(d.total_machines) || 12480
          root.tierName = root.cleanSanitized(d.tier_name || "Unknown", 40)
          root.tierIcon = root.cleanSanitized(d.tier_icon || "󰢮", 8)
          root.rankNerdIcon = root.cleanSanitized(d.tier_nerd_icon || root.rankNerdIconFor(root.totalScore), 8)
          root.tierColor = root.cleanSanitized(d.tier_color || "#38bdf8", 16)
          root.tierQuote = root.cleanSanitized(d.tier_quote || "", 140)
          root.percentileText = root.cleanSanitized(d.percentile_text || "", 100)
          root.battlestationId = root.cleanSanitized(d.battlestation_id || "OMA-????", 24)
          root.archetypeSignature = root.cleanSanitized(d.archetype_signature || "OMA-BUILD", 30)

          if (root.globalRank > 0) {
            root.worldRankDesc = "#" + root.globalRank.toLocaleString() + " / " + root.totalMachines.toLocaleString()
          }

          if (d.sub_scores) {
            root.cpuScore = Number(d.sub_scores.cpu) || 0
            root.gpuScore = Number(d.sub_scores.gpu) || 0
            root.ramScore = Number(d.sub_scores.ram) || 0
            root.moboScore = Number(d.sub_scores.mobo) || 0
            root.displayScore = Number(d.sub_scores.display) || 0
            root.storageScore = Number(d.sub_scores.storage) || 0
          }

          if (d.hardware) {
            var rawCpu = String(d.hardware.cpu_name || "")
            var cleanCpu = rawCpu.replace(/Intel\(R\)\s+Core\(TM\)\s+/g, "").replace(/AMD\s+Ryzen\s+/g, "Ryzen ").trim()
            var threads = d.hardware.cpu_threads ? (" (" + d.hardware.cpu_threads + " Threads)") : ""
            root.cpuDesc = root.cleanSanitized(cleanCpu + threads, 65)

            var rawGpu = String(d.hardware.gpu_name || "")
            var cleanGpu = rawGpu.replace(/NVIDIA Corporation /g, "").replace(/Advanced Micro Devices, Inc\. /g, "").replace(/Intel Corporation /g, "").trim()
            root.gpuDesc = root.cleanSanitized(cleanGpu, 65)

            var ramType = d.hardware.ram_type || "RAM"
            var ramSpeed = d.hardware.ram_speed_mts ? (" @ " + d.hardware.ram_speed_mts + " MT/s") : ""
            var ramGb = (Number(d.hardware.ram_total_gb) || 0).toFixed(1)
            root.ramDesc = root.cleanSanitized(ramGb + " GB " + ramType + ramSpeed, 65)

            var moboName = String(d.hardware.mobo_name || "").replace(/DDR4/g, "").replace(/DDR5/g, "").trim()
            var vendor = String(d.hardware.mobo_vendor || "").trim()
            var chipset = String(d.hardware.chipset || "").replace(/Intel /g, "").replace(/AMD /g, "").trim()
            var fullMobo = moboName
            if (vendor && !moboName.toLowerCase().includes(vendor.toLowerCase())) {
              fullMobo = vendor + " " + moboName
            }
            if (chipset && !fullMobo.includes(chipset) && chipset !== "Mainstream Chipset") {
              fullMobo = fullMobo + " (" + chipset + ")"
            }
            root.moboDesc = root.cleanSanitized(fullMobo, 65)

            var rawStorage = String(d.hardware.storage_model || d.hardware.storage_type || "NVMe")
            root.storageDesc = root.cleanSanitized(rawStorage, 65)

            if (d.hardware.monitors && d.hardware.monitors.length > 0) {
              var m = d.hardware.monitors[0]
              root.displayDesc = root.cleanSanitized(m.width + "x" + m.height + " @" + Math.round(m.refresh_rate) + "Hz (" + m.name + ")", 65)
            }
            root.osDesc = root.cleanSanitized(d.hardware.os_name || "Omarchy Linux", 35)
          }
        } catch(e) {
          // Preserve previous state on parse errors
        }
      }
    }
  }

  Process {
    id: surveyProc
    command: [root.resolveEnginePath(), "--submit-survey"]
    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: {
        root.isSubmittingSurvey = false
        try {
          var cleanText = String(text || "").slice(0, 65536)
          var res = JSON.parse(cleanText)
          root.surveyStatusMsg = root.cleanSanitized(res.message || "Survey submitted!", 100)
        } catch(e) {
          root.surveyStatusMsg = "Survey recorded locally."
        }
      }
    }
  }

  Process {
    id: copyProc
  }

  Process {
    id: launchProc
    onExited: function(exitCode) {
      launchDeadlineTimer.stop()
    }
  }

  Timer {
    id: launchDeadlineTimer
    interval: 5000
    repeat: false
    onTriggered: {
      if (launchProc.running) launchProc.running = false
    }
  }

  Component.onDestruction: {
    if (statusProc.running) statusProc.running = false
    if (surveyProc.running) surveyProc.running = false
    if (copyProc.running) copyProc.running = false
    if (launchProc.running) launchProc.running = false
  }

  Component.onCompleted: {
    if (!statusProc.running) statusProc.running = true
  }

  Timer {
    interval: 30000
    running: true
    repeat: true
    triggeredOnStart: false
    onTriggered: {
      if (!statusProc.running) statusProc.running = true
    }
  }

  Timer {
    id: copyToastTimer
    interval: 2000
    repeat: false
    onTriggered: root.copyToastMsg = ""
  }

  // Top Bar Icon: Dynamic rank Nerd Font glyph matching current score tier!
  BarIconButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    text: root.rankNerdIcon || root.rankNerdIconFor(root.totalScore)
    tooltipText: "OmaRank: " + root.tierName + " (" + (root.totalScore > 0 ? (root.totalScore + "/100") : "Calculating...") + ")"
    onPressed: function(b) {
      if (root.opened) root.close()
      else root.open()
    }
  }

  KeyboardPanel {
    id: panel
    anchorItem: button
    owner: root
    bar: root.bar
    open: root.opened
    contentWidth: panel.fittedContentWidth(Style.space(520))
    contentHeight: panel.fittedContentHeight(column.implicitHeight)

    Column {
      id: column
      anchors.left: parent.left
      anchors.right: parent.right
      anchors.top: parent.top
      spacing: Style.space(12)

      // ---------- Hero Header (Matching Omarchy Network Panel) ----------
      Item {
        width: parent.width
        implicitHeight: Math.max(heroIcon.implicitHeight, heroLabels.implicitHeight, heroActions.implicitHeight)

        Text {
          id: heroIcon
          textFormat: Text.PlainText
          text: root.rankNerdIcon || root.rankNerdIconFor(root.totalScore)
          color: root.bar ? root.bar.foreground : Color.foreground
          font.family: root.bar ? root.bar.fontFamily : Style.font.family
          font.pixelSize: Style.font.display
          anchors.left: parent.left
          anchors.verticalCenter: parent.verticalCenter
        }

        RowLayout {
          id: heroActions
          spacing: Style.space(6)
          anchors.right: parent.right
          anchors.verticalCenter: parent.verticalCenter

          Button {
            id: refreshAction
            iconText: "󰑐"
            tooltipText: "Re-check specs"
            foreground: root.bar ? root.bar.foreground : Color.foreground
            fontFamily: root.bar ? root.bar.fontFamily : Style.font.family
            iconSize: Style.font.subtitle * 1.3
            horizontalPadding: Style.space(5)
            verticalPadding: Style.space(2)
            Layout.alignment: Qt.AlignVCenter
            onClicked: root.refresh()
          }

          Button {
            id: dashAction
            iconText: "󰍹"
            tooltipText: "Open Terminal Dashboard"
            foreground: root.bar ? root.bar.foreground : Color.foreground
            fontFamily: root.bar ? root.bar.fontFamily : Style.font.family
            iconSize: Style.font.subtitle * 1.3
            horizontalPadding: Style.space(5)
            verticalPadding: Style.space(2)
            Layout.alignment: Qt.AlignVCenter
            onClicked: root.launchDashboard()
          }
        }

        Column {
          id: heroLabels
          anchors.left: heroIcon.right
          anchors.leftMargin: Style.space(14)
          anchors.right: heroActions.left
          anchors.rightMargin: Style.space(10)
          anchors.verticalCenter: parent.verticalCenter
          spacing: Style.space(2)

          Text {
            id: heroTitle
            textFormat: Text.PlainText
            width: parent.width
            text: root.tierName
            color: root.bar ? root.bar.foreground : Color.foreground
            font.family: root.bar ? root.bar.fontFamily : Style.font.family
            font.pixelSize: Style.font.title
            font.bold: true
            elide: Text.ElideRight
          }

          Text {
            id: heroMeta
            textFormat: Text.PlainText
            width: parent.width
            text: (root.globalRank > 0 ? ("WORLD RANK #" + root.globalRank.toLocaleString() + " • ") : "") + "SCORE: " + (root.totalScore > 0 ? root.totalScore : "--") + " / 100"
            color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.4)
            font.family: root.bar ? root.bar.fontFamily : Style.font.family
            font.pixelSize: Style.font.caption
            font.bold: true
            font.letterSpacing: 1.2
            elide: Text.ElideRight
          }
        }
      }

      // ---------- Hardware Specs Telemetry (Unclipped Modern Omarchy Cards) ----------
      Column {
        width: parent.width
        spacing: Style.space(6)

        GridLayout {
          width: parent.width
          columns: 2
          columnSpacing: Style.space(14)
          rowSpacing: Style.space(5)

          RowLayout {
            spacing: Style.space(6)
            Text {
              textFormat: Text.PlainText
              text: "󰍛"
              font.family: root.bar ? root.bar.fontFamily : Style.font.family
              font.pixelSize: Style.font.bodySmall
              color: root.bar ? root.bar.foreground : Color.foreground
              opacity: 0.5
            }
            InfoLabel { text: "Processor" }
          }
          DetailValue {
            text: root.cpuDesc
            copyable: true
            tooltipText: "Click to copy CPU: " + root.cpuDesc
          }

          RowLayout {
            spacing: Style.space(6)
            Text {
              textFormat: Text.PlainText
              text: "󰢮"
              font.family: root.bar ? root.bar.fontFamily : Style.font.family
              font.pixelSize: Style.font.bodySmall
              color: root.bar ? root.bar.foreground : Color.foreground
              opacity: 0.5
            }
            InfoLabel { text: "Graphics" }
          }
          DetailValue {
            text: root.gpuDesc
            copyable: true
            tooltipText: "Click to copy GPU: " + root.gpuDesc
          }

          RowLayout {
            spacing: Style.space(6)
            Text {
              textFormat: Text.PlainText
              text: "󰘚"
              font.family: root.bar ? root.bar.fontFamily : Style.font.family
              font.pixelSize: Style.font.bodySmall
              color: root.bar ? root.bar.foreground : Color.foreground
              opacity: 0.5
            }
            InfoLabel { text: "Memory" }
          }
          DetailValue {
            text: root.ramDesc
            copyable: true
            tooltipText: "Click to copy RAM: " + root.ramDesc
          }

          RowLayout {
            spacing: Style.space(6)
            Text {
              textFormat: Text.PlainText
              text: "󰋊"
              font.family: root.bar ? root.bar.fontFamily : Style.font.family
              font.pixelSize: Style.font.bodySmall
              color: root.bar ? root.bar.foreground : Color.foreground
              opacity: 0.5
            }
            InfoLabel { text: "Storage" }
          }
          DetailValue {
            text: root.storageDesc
            copyable: true
            tooltipText: "Click to copy Storage: " + root.storageDesc
          }

          RowLayout {
            spacing: Style.space(6)
            Text {
              textFormat: Text.PlainText
              text: "󰌢"
              font.family: root.bar ? root.bar.fontFamily : Style.font.family
              font.pixelSize: Style.font.bodySmall
              color: root.bar ? root.bar.foreground : Color.foreground
              opacity: 0.5
            }
            InfoLabel { text: "Display" }
          }
          DetailValue {
            text: root.displayDesc
            copyable: true
            tooltipText: "Click to copy Display: " + root.displayDesc
          }

          RowLayout {
            spacing: Style.space(6)
            Text {
              textFormat: Text.PlainText
              text: "󱤓"
              font.family: root.bar ? root.bar.fontFamily : Style.font.family
              font.pixelSize: Style.font.bodySmall
              color: root.bar ? root.bar.foreground : Color.foreground
              opacity: 0.5
            }
            InfoLabel { text: "Motherboard" }
          }
          DetailValue {
            text: root.moboDesc
            copyable: true
            tooltipText: "Click to copy Motherboard: " + root.moboDesc
          }
        }

        // Battlestation ID & Archetype Sub-Row
        Item {
          width: parent.width
          implicitHeight: Style.space(18)

          RowLayout {
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            spacing: Style.space(4)

            Text {
              textFormat: Text.PlainText
              text: "󰌽"
              font.family: root.bar ? root.bar.fontFamily : Style.font.family
              font.pixelSize: Style.font.caption
              color: root.bar ? root.bar.foreground : Color.foreground
              opacity: 0.4
            }
            Text {
              textFormat: Text.PlainText
              text: "ID: " + root.battlestationId
              font.family: root.bar ? root.bar.fontFamily : Style.font.family
              font.pixelSize: Style.font.caption
              color: root.bar ? root.bar.foreground : Color.foreground
              opacity: 0.6
            }
          }

          Text {
            visible: root.copyToastMsg.length > 0
            textFormat: Text.PlainText
            text: root.copyToastMsg
            font.family: root.bar ? root.bar.fontFamily : Style.font.family
            font.pixelSize: Style.font.caption
            font.bold: true
            color: Color.accent ? Color.accent : (root.bar ? root.bar.foreground : Color.foreground)
            anchors.centerIn: parent
          }

          RowLayout {
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            spacing: Style.space(4)

            Text {
              textFormat: Text.PlainText
              text: "󰚥"
              font.family: root.bar ? root.bar.fontFamily : Style.font.family
              font.pixelSize: Style.font.caption
              color: root.bar ? root.bar.foreground : Color.foreground
              opacity: 0.4
            }
            Text {
              textFormat: Text.PlainText
              text: root.archetypeSignature
              font.family: root.bar ? root.bar.fontFamily : Style.font.family
              font.pixelSize: Style.font.caption
              color: root.bar ? root.bar.foreground : Color.foreground
              opacity: 0.6
            }
          }
        }
      }

      // ---------- Sub-Scores Section Separator ----------
      PanelSeparator {
        foreground: root.bar ? root.bar.foreground : Color.foreground
      }

      // ---------- Sub-Scores Breakdown (Exact DNS Provider Segmented Pill Row) ----------
      Column {
        width: parent.width
        spacing: Style.space(10)

        PanelSectionHeader {
          text: "SUB-SCORES BREAKDOWN"
          foreground: root.bar ? root.bar.foreground : Color.foreground
          fontFamily: root.bar ? root.bar.fontFamily : Style.font.family
        }

        Row {
          id: subScoresRow
          width: parent.width
          spacing: Style.space(6)

          readonly property int count: 6
          readonly property real cellWidth: (width - spacing * (count - 1)) / count

          SubScorePill {
            label: "CPU " + root.cpuScore
            tooltip: "Processor Score: " + root.cpuScore + "/100 (" + root.cpuDesc + ")"
            width: subScoresRow.cellWidth
          }

          SubScorePill {
            label: "GPU " + root.gpuScore
            tooltip: "Graphics Score: " + root.gpuScore + "/100 (" + root.gpuDesc + ")"
            width: subScoresRow.cellWidth
          }

          SubScorePill {
            label: "RAM " + root.ramScore
            tooltip: "Memory Score: " + root.ramScore + "/100 (" + root.ramDesc + ")"
            width: subScoresRow.cellWidth
          }

          SubScorePill {
            label: "MOBO " + root.moboScore
            tooltip: "Motherboard Score: " + root.moboScore + "/100 (" + root.moboDesc + ")"
            width: subScoresRow.cellWidth
          }

          SubScorePill {
            label: "DISP " + root.displayScore
            tooltip: "Display Score: " + root.displayScore + "/100 (" + root.displayDesc + ")"
            width: subScoresRow.cellWidth
          }

          SubScorePill {
            label: "DISK " + root.storageScore
            tooltip: "Storage Score: " + root.storageScore + "/100 (" + root.storageDesc + ")"
            width: subScoresRow.cellWidth
          }
        }
      }

      // ---------- Verdict Section Separator ----------
      PanelSeparator {
        foreground: root.bar ? root.bar.foreground : Color.foreground
      }

      // ---------- System Verdict Card (Exact Known Networks Box Style) ----------
      Column {
        width: parent.width
        spacing: Style.space(10)

        PanelSectionHeader {
          text: "SYSTEM VERDICT"
          foreground: root.bar ? root.bar.foreground : Color.foreground
          fontFamily: root.bar ? root.bar.fontFamily : Style.font.family
        }

        BorderSurface {
          id: verdictCard
          width: parent.width
          implicitHeight: verdictRow.implicitHeight + Style.space(16)
          radius: Style.cornerRadius
          color: Qt.rgba(0, 0, 0, 0.25)
          border.color: Qt.rgba(root.bar ? root.bar.foreground.r : 1, root.bar ? root.bar.foreground.g : 1, root.bar ? root.bar.foreground.b : 1, 0.22)
          border.width: 1

          RowLayout {
            id: verdictRow
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: Style.space(12)
            anchors.rightMargin: Style.space(12)
            spacing: Style.space(12)

            Text {
              textFormat: Text.PlainText
              text: root.rankNerdIcon || root.rankNerdIconFor(root.totalScore)
              color: root.bar ? root.bar.foreground : Color.foreground
              font.family: root.bar ? root.bar.fontFamily : Style.font.family
              font.pixelSize: Style.font.title
              Layout.alignment: Qt.AlignVCenter
            }

            Column {
              Layout.fillWidth: true
              spacing: Style.space(2)

              RowLayout {
                spacing: Style.space(8)
                Text {
                  textFormat: Text.PlainText
                  text: root.tierName
                  color: root.bar ? root.bar.foreground : Color.foreground
                  font.family: root.bar ? root.bar.fontFamily : Style.font.family
                  font.pixelSize: Style.font.bodySmall
                  font.bold: true
                }
                Text {
                  visible: root.percentileText.length > 0
                  textFormat: Text.PlainText
                  text: "• " + root.percentileText
                  color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.4)
                  font.family: root.bar ? root.bar.fontFamily : Style.font.family
                  font.pixelSize: Style.font.caption
                }
              }

              Text {
                textFormat: Text.PlainText
                text: "“" + root.tierQuote + "”"
                color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.4)
                font.family: root.bar ? root.bar.fontFamily : Style.font.family
                font.pixelSize: Style.font.caption
                wrapMode: Text.WordWrap
                width: parent.width
              }
            }

            Text {
              textFormat: Text.PlainText
              text: (root.totalScore > 0 ? String(root.totalScore) : "--") + " pts"
              color: root.bar ? root.bar.foreground : Color.foreground
              font.family: root.bar ? root.bar.fontFamily : Style.font.family
              font.pixelSize: Style.font.bodySmall
              font.bold: true
              Layout.alignment: Qt.AlignVCenter
            }
          }
        }
      }

      // ---------- Action Buttons Separator ----------
      PanelSeparator {
        foreground: root.bar ? root.bar.foreground : Color.foreground
      }

      // ---------- Action Buttons ----------
      Column {
        width: parent.width
        spacing: Style.space(6)

        Button {
          width: parent.width
          bordered: true
          text: root.isSubmittingSurvey ? "Submitting to OmaStat..." : "Submit Anonymous Specs to OmaStat"
          fontSize: Style.font.bodySmall
          foreground: root.bar ? root.bar.foreground : Color.foreground
          fontFamily: root.bar ? root.bar.fontFamily : Style.font.family
          horizontalPadding: Style.spacing.controlPaddingX
          verticalPadding: Style.spacing.controlPaddingY
          enabled: !root.isSubmittingSurvey
          onClicked: root.triggerSurvey()
        }

        Text {
          visible: root.surveyStatusMsg.length > 0
          textFormat: Text.PlainText
          text: root.surveyStatusMsg
          font.pixelSize: Style.font.caption
          color: root.bar ? root.bar.foreground : Color.foreground
          anchors.horizontalCenter: parent.horizontalCenter
        }

        Button {
          width: parent.width
          bordered: true
          text: "Open Terminal Benchmark & Ladder"
          fontSize: Style.font.bodySmall
          foreground: root.bar ? root.bar.foreground : Color.foreground
          fontFamily: root.bar ? root.bar.fontFamily : Style.font.family
          horizontalPadding: Style.spacing.controlPaddingX
          verticalPadding: Style.spacing.controlPaddingY
          onClicked: root.launchDashboard()
        }
      }
    }
  }

  // Segmented sub-score pill (matching DnsProviderPill)
  component SubScorePill: Button {
    id: pill
    required property string label
    required property string tooltip

    text: label
    tooltipText: tooltip
    fontSize: Style.font.bodySmall
    foreground: root.bar ? root.bar.foreground : Color.foreground
    fontFamily: root.bar ? root.bar.fontFamily : Style.font.family
    horizontalPadding: Style.space(4)
    verticalPadding: Style.spacing.controlPaddingY + Style.space(2)
    bordered: true
  }

  // Label text matching Omarchy InfoLabel
  component InfoLabel: Text {
    textFormat: Text.PlainText
    color: root.bar ? root.bar.foreground : Color.foreground
    opacity: 0.6
    font.family: root.bar ? root.bar.fontFamily : Style.font.family
    font.pixelSize: Style.font.bodySmall
  }

  // Value text matching Omarchy InfoValue
  component InfoValue: Text {
    textFormat: Text.PlainText
    color: root.bar ? root.bar.foreground : Color.foreground
    font.family: root.bar ? root.bar.fontFamily : Style.font.family
    font.pixelSize: Style.font.bodySmall
  }

  // Detail value matching Omarchy DetailValue with copy-to-clipboard
  component DetailValue: InfoValue {
    property bool copyable: false
    property string tooltipText: "Copy to clipboard"

    Layout.fillWidth: true
    horizontalAlignment: Text.AlignRight
    elide: Text.ElideRight

    MouseArea {
      id: valueMouse
      anchors.fill: parent
      enabled: copyable && parent.text !== "" && parent.text !== "--"
      hoverEnabled: enabled
      cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
      onClicked: root.copyToClipboard(parent.text)
    }

    PanelToolTip {
      visible: valueMouse.enabled && valueMouse.containsMouse
      text: tooltipText
      fontFamily: root.bar ? root.bar.fontFamily : Style.font.family
    }
  }
}
