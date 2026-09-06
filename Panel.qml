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

  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  property int totalScore: 0
  property string tierName: "Benchmarking..."
  property string tierIcon: "🏆"
  property string tierColor: "#38bdf8"
  readonly property color tierColorObj: Qt.color(root.tierColor)
  property string tierQuote: ""
  property string percentileText: ""

  property int cpuScore: 0
  property int gpuScore: 0
  property int ramScore: 0
  property int displayScore: 0
  property int storageScore: 0

  property string cpuDesc: ""
  property string gpuDesc: ""
  property string ramDesc: ""
  property string displayDesc: ""
  property string storageDesc: ""

  property string surveyStatusMsg: ""
  property bool isSubmittingSurvey: false

  function cleanSanitized(str, maxLen) {
    if (!str) return ""
    var s = String(str).replace(/[\x00-\x1f\x7f-\x9f<>&`'"\\]/g, "").trim()
    return s.slice(0, maxLen || 80)
  }

  function resolveEnginePath() {
    return Qt.resolvedUrl("omarank-engine").toString().replace(/^file:\/\//, "")
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
          root.tierName = root.cleanSanitized(d.tier_name || "Unknown", 40)
          root.tierIcon = root.cleanSanitized(d.tier_icon || "🏆", 8)
          root.tierColor = root.cleanSanitized(d.tier_color || "#38bdf8", 16)
          root.tierQuote = root.cleanSanitized(d.tier_quote || "", 120)
          root.percentileText = root.cleanSanitized(d.percentile_text || "", 100)

          if (d.sub_scores) {
            root.cpuScore = Number(d.sub_scores.cpu) || 0
            root.gpuScore = Number(d.sub_scores.gpu) || 0
            root.ramScore = Number(d.sub_scores.ram) || 0
            root.displayScore = Number(d.sub_scores.display) || 0
            root.storageScore = Number(d.sub_scores.storage) || 0
          }

          if (d.hardware) {
            root.cpuDesc = root.cleanSanitized(d.hardware.cpu_name + " (" + d.hardware.cpu_threads + " threads)", 60)
            root.gpuDesc = root.cleanSanitized(d.hardware.gpu_name, 50)
            root.ramDesc = root.cleanSanitized(d.hardware.ram_total_gb + " GB System RAM", 30)
            if (d.hardware.monitors && d.hardware.monitors.length > 0) {
              var m = d.hardware.monitors[0]
              root.displayDesc = root.cleanSanitized(m.width + "x" + m.height + " @" + m.refresh_rate + "Hz", 30)
            }
            root.storageDesc = root.cleanSanitized(d.hardware.storage_type, 30)
          }
        } catch(e) {
          // Keep prior state
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
      if (launchProc.running) launchProc.kill()
    }
  }

  Component.onDestruction: {
    if (statusProc.running) statusProc.kill()
    if (surveyProc.running) surveyProc.kill()
    if (launchProc.running) launchProc.kill()
  }

  Component.onCompleted: {
    if (!statusProc.running) statusProc.running = true
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

  Timer {
    interval: 30000
    running: true
    repeat: true
    triggeredOnStart: true
    onTriggered: {
      if (!statusProc.running) statusProc.running = true
    }
  }

  WidgetButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    text: root.tierIcon + " " + (root.totalScore > 0 ? String(root.totalScore) : "--")
    foreground: root.tierColor
    tooltipText: "OmaRank: " + root.totalScore + " / 100 (" + root.tierName + ")"
    onPressed: function(b) { root.toggle() }
  }

  KeyboardPanel {
    id: panel
    anchorItem: button
    owner: root
    bar: root.bar
    open: root.opened
    contentWidth: panel.fittedContentWidth(Style.space(430))
    contentHeight: panel.fittedContentHeight(column.implicitHeight)

    Column {
      id: column
      anchors.left: parent.left
      anchors.right: parent.right
      anchors.top: parent.top
      spacing: Style.space(12)

      // ---------- Hero Header ----------
      Item {
        width: parent.width
        implicitHeight: Math.max(heroLabels.implicitHeight, heroScoreRow.implicitHeight)

        Column {
          id: heroLabels
          anchors.left: parent.left
          anchors.right: heroScoreRow.left
          anchors.rightMargin: Style.space(8)
          anchors.verticalCenter: parent.verticalCenter
          spacing: Style.space(4)

          RowLayout {
            spacing: Style.space(8)
            Text {
              textFormat: Text.PlainText
              text: "🏆 OmaRank"
              color: root.bar ? root.bar.foreground : Color.foreground
              font.family: root.bar ? root.bar.fontFamily : Style.font.family
              font.pixelSize: Style.font.title
              font.bold: true
            }

            Rectangle {
              height: Style.space(18)
              width: tierBadgeText.implicitWidth + Style.space(12)
              radius: Style.space(9)
              color: Qt.rgba(root.tierColorObj.r, root.tierColorObj.g, root.tierColorObj.b, 0.16)
              border.color: root.tierColor
              border.width: 1

              Text {
                id: tierBadgeText
                textFormat: Text.PlainText
                anchors.centerIn: parent
                text: root.tierIcon + " " + root.tierName
                font.pixelSize: 10
                font.bold: true
                color: root.tierColor
              }
            }
          }

          Text {
            textFormat: Text.PlainText
            text: root.percentileText.toUpperCase()
            color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.4)
            font.family: root.bar ? root.bar.fontFamily : Style.font.family
            font.pixelSize: 9
            font.bold: true
            font.letterSpacing: 1.1
            elide: Text.ElideRight
            width: parent.width
          }
        }

        Row {
          id: heroScoreRow
          anchors.right: parent.right
          anchors.verticalCenter: parent.verticalCenter
          spacing: Style.space(1)

          Text {
            textFormat: Text.PlainText
            text: root.totalScore > 0 ? String(root.totalScore) : "—"
            color: root.tierColor
            font.family: root.bar ? root.bar.fontFamily : Style.font.family
            font.pixelSize: Style.font.displayLarge
            font.bold: true
            anchors.baseline: scoreMax.baseline
          }

          Text {
            id: scoreMax
            textFormat: Text.PlainText
            text: "/100"
            color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.8)
            font.family: root.bar ? root.bar.fontFamily : Style.font.family
            font.pixelSize: Style.font.caption
            font.bold: true
          }
        }
      }

      // ---------- Critic Quote Callout ----------
      BorderSurface {
        width: parent.width
        implicitHeight: quoteText.implicitHeight + Style.space(14)
        radius: Style.cornerRadius
        color: Qt.rgba(0, 0, 0, 0.25)
        border.color: Qt.rgba(1, 1, 1, 0.08)
        border.width: 1

        Text {
          id: quoteText
          textFormat: Text.PlainText
          anchors.fill: parent
          anchors.margins: Style.space(8)
          text: "“" + root.tierQuote + "”"
          font.family: root.bar ? root.bar.fontFamily : Style.font.family
          font.pixelSize: Style.font.caption
          font.italic: true
          color: root.bar ? root.bar.foreground : Color.foreground
          wrapMode: Text.WordWrap
          verticalAlignment: Text.AlignVCenter
        }
      }

      PanelSeparator {
        width: parent.width
        foreground: root.bar ? root.bar.foreground : Color.foreground
      }

      // ---------- Hardware Breakdown Bars (All 5 Components) ----------
      Column {
        width: parent.width
        spacing: Style.space(8)

        // CPU
        Column {
          width: parent.width
          spacing: Style.space(3)
          RowLayout {
            width: parent.width
            Text {
              textFormat: Text.PlainText
              text: "Processor (CPU)"
              font.pixelSize: Style.font.caption
              font.bold: true
              color: root.bar ? root.bar.foreground : Color.foreground
            }
            Text {
              textFormat: Text.PlainText
              text: root.cpuDesc
              font.pixelSize: 9
              color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.5)
              Layout.fillWidth: true
              horizontalAlignment: Text.AlignRight
              elide: Text.ElideRight
            }
            Text {
              textFormat: Text.PlainText
              text: root.cpuScore + "/100"
              font.pixelSize: Style.font.caption
              font.bold: true
              color: "#38bdf8"
            }
          }
          Rectangle {
            width: parent.width
            height: 4
            radius: 2
            color: Qt.rgba(1, 1, 1, 0.08)
            Rectangle {
              width: Math.min(parent.width, Math.max(0, (root.cpuScore / 100) * parent.width))
              height: parent.height
              radius: 2
              color: "#38bdf8"
            }
          }
        }

        // GPU
        Column {
          width: parent.width
          spacing: Style.space(3)
          RowLayout {
            width: parent.width
            Text {
              textFormat: Text.PlainText
              text: "Graphics (GPU)"
              font.pixelSize: Style.font.caption
              font.bold: true
              color: root.bar ? root.bar.foreground : Color.foreground
            }
            Text {
              textFormat: Text.PlainText
              text: root.gpuDesc
              font.pixelSize: 9
              color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.5)
              Layout.fillWidth: true
              horizontalAlignment: Text.AlignRight
              elide: Text.ElideRight
            }
            Text {
              textFormat: Text.PlainText
              text: root.gpuScore + "/100"
              font.pixelSize: Style.font.caption
              font.bold: true
              color: "#818cf8"
            }
          }
          Rectangle {
            width: parent.width
            height: 4
            radius: 2
            color: Qt.rgba(1, 1, 1, 0.08)
            Rectangle {
              width: Math.min(parent.width, Math.max(0, (root.gpuScore / 100) * parent.width))
              height: parent.height
              radius: 2
              color: "#818cf8"
            }
          }
        }

        // Memory (RAM)
        Column {
          width: parent.width
          spacing: Style.space(3)
          RowLayout {
            width: parent.width
            Text {
              textFormat: Text.PlainText
              text: "Memory (RAM)"
              font.pixelSize: Style.font.caption
              font.bold: true
              color: root.bar ? root.bar.foreground : Color.foreground
            }
            Text {
              textFormat: Text.PlainText
              text: root.ramDesc
              font.pixelSize: 9
              color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.5)
              Layout.fillWidth: true
              horizontalAlignment: Text.AlignRight
              elide: Text.ElideRight
            }
            Text {
              textFormat: Text.PlainText
              text: root.ramScore + "/100"
              font.pixelSize: Style.font.caption
              font.bold: true
              color: "#22c55e"
            }
          }
          Rectangle {
            width: parent.width
            height: 4
            radius: 2
            color: Qt.rgba(1, 1, 1, 0.08)
            Rectangle {
              width: Math.min(parent.width, Math.max(0, (root.ramScore / 100) * parent.width))
              height: parent.height
              radius: 2
              color: "#22c55e"
            }
          }
        }

        // Display
        Column {
          width: parent.width
          spacing: Style.space(3)
          RowLayout {
            width: parent.width
            Text {
              textFormat: Text.PlainText
              text: "Display & Hz"
              font.pixelSize: Style.font.caption
              font.bold: true
              color: root.bar ? root.bar.foreground : Color.foreground
            }
            Text {
              textFormat: Text.PlainText
              text: root.displayDesc
              font.pixelSize: 9
              color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.5)
              Layout.fillWidth: true
              horizontalAlignment: Text.AlignRight
              elide: Text.ElideRight
            }
            Text {
              textFormat: Text.PlainText
              text: root.displayScore + "/100"
              font.pixelSize: Style.font.caption
              font.bold: true
              color: "#f59e0b"
            }
          }
          Rectangle {
            width: parent.width
            height: 4
            radius: 2
            color: Qt.rgba(1, 1, 1, 0.08)
            Rectangle {
              width: Math.min(parent.width, Math.max(0, (root.displayScore / 100) * parent.width))
              height: parent.height
              radius: 2
              color: "#f59e0b"
            }
          }
        }

        // Storage (Disk)
        Column {
          width: parent.width
          spacing: Style.space(3)
          RowLayout {
            width: parent.width
            Text {
              textFormat: Text.PlainText
              text: "Storage (Disk)"
              font.pixelSize: Style.font.caption
              font.bold: true
              color: root.bar ? root.bar.foreground : Color.foreground
            }
            Text {
              textFormat: Text.PlainText
              text: root.storageDesc
              font.pixelSize: 9
              color: Qt.darker(root.bar ? root.bar.foreground : Color.foreground, 1.5)
              Layout.fillWidth: true
              horizontalAlignment: Text.AlignRight
              elide: Text.ElideRight
            }
            Text {
              textFormat: Text.PlainText
              text: root.storageScore + "/100"
              font.pixelSize: Style.font.caption
              font.bold: true
              color: "#a855f7"
            }
          }
          Rectangle {
            width: parent.width
            height: 4
            radius: 2
            color: Qt.rgba(1, 1, 1, 0.08)
            Rectangle {
              width: Math.min(parent.width, Math.max(0, (root.storageScore / 100) * parent.width))
              height: parent.height
              radius: 2
              color: "#a855f7"
            }
          }
        }
      }

      PanelSeparator {
        width: parent.width
        foreground: root.bar ? root.bar.foreground : Color.foreground
      }

      // ---------- Action Buttons ----------
      Column {
        width: parent.width
        spacing: Style.space(6)

        Button {
          width: parent.width
          bordered: true
          iconText: "🌐"
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
          color: "#4ade80"
          anchors.horizontalCenter: parent.horizontalCenter
        }

        Button {
          width: parent.width
          bordered: true
          iconText: "⚡"
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
}
