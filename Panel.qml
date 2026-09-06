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

  property int totalScore: 0
  property string tierName: "Benchmarking..."
  property string tierIcon: "🏆"
  property string tierColor: "#38bdf8"
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
            root.ramDesc = root.cleanSanitized(d.hardware.ram_total_gb + " GB RAM", 30)
            if (d.hardware.monitors && d.hardware.monitors.length > 0) {
              var m = d.hardware.monitors[0]
              root.displayDesc = root.cleanSanitized(m.width + "x" + m.height + " @" + m.refresh_rate + "Hz", 30)
            }
            root.storageDesc = root.cleanSanitized(d.hardware.storage_type, 30)
          }
        } catch(e) {
          // Keep prior state on parse error
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

  BarIconButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    text: root.tierIcon + " " + (root.totalScore > 0 ? String(root.totalScore) : "--")
    color: root.tierColor
    slotSize: Style.bar.statusSlot
    tooltipText: "OmaRank: " + root.totalScore + " / 100 (" + root.tierName + ")"
    onPressed: root.toggle()
  }

  KeyboardPanel {
    id: panel
    anchorItem: button
    owner: root
    width: 480
    contentHeight: panel.fittedContentHeight(mainCol.implicitHeight)

    Column {
      id: mainCol
      anchors.left: parent.left
      anchors.right: parent.right
      anchors.top: parent.top
      spacing: Style.space(12)

      // Header Row
      RowLayout {
        width: parent.width

        Text {
          textFormat: Text.PlainText
          text: "🏆 OmaRank • Hardware Benchmark"
          font.pixelSize: Style.font.title
          font.bold: true
          color: root.bar ? root.bar.foreground : "#ffffff"
          Layout.fillWidth: true
        }

        Rectangle {
          width: 140
          height: 24
          radius: 12
          color: root.tierColor
          opacity: 0.9

          Text {
            textFormat: Text.PlainText
            anchors.centerIn: parent
            text: root.tierIcon + " " + root.tierName
            font.pixelSize: 10
            font.bold: true
            color: "#0f172a"
          }
        }
      }

      // Main Score & Tier Quote Card
      Rectangle {
        width: parent.width
        height: 84
        radius: 10
        color: "#0f172a"
        border.color: "#1e293b"
        border.width: 1

        RowLayout {
          anchors.fill: parent
          anchors.margins: 12
          spacing: 14

          // Big Score Badge
          Rectangle {
            width: 60
            height: 60
            radius: 30
            color: "#1e293b"
            border.color: root.tierColor
            border.width: 2

            Column {
              anchors.centerIn: parent
              spacing: -2
              Text {
                textFormat: Text.PlainText
                anchors.horizontalCenter: parent.horizontalCenter
                text: String(root.totalScore)
                font.bold: true
                font.pixelSize: 20
                color: root.tierColor
              }
              Text {
                textFormat: Text.PlainText
                anchors.horizontalCenter: parent.horizontalCenter
                text: "/100"
                font.pixelSize: 9
                color: "#94a3b8"
              }
            }
          }

          // Tier Quote & Percentile
          Column {
            Layout.fillWidth: true
            spacing: 4

            Text {
              textFormat: Text.PlainText
              text: root.tierQuote
              font.pixelSize: Style.font.caption
              color: "#f8fafc"
              wrapMode: Text.WordWrap
              width: 370
            }

            Text {
              textFormat: Text.PlainText
              text: "🌐 " + root.percentileText
              font.pixelSize: 10
              font.bold: true
              color: root.tierColor
            }
          }
        }
      }

      // Hardware Subscore Breakdown Bars
      Rectangle {
        width: parent.width
        implicitHeight: breakdownCol.implicitHeight + 20
        radius: 8
        color: "#090d16"
        border.color: "#1e293b"
        border.width: 1

        Column {
          id: breakdownCol
          anchors.fill: parent
          anchors.margins: 10
          spacing: 8

          // CPU Bar
          Column {
            width: parent.width
            spacing: 2
            RowLayout {
              width: parent.width
              Text {
                textFormat: Text.PlainText
                text: "CPU: " + root.cpuDesc
                font.pixelSize: 10
                color: "#cbd5e1"
                Layout.fillWidth: true
                elide: Text.ElideRight
              }
              Text {
                textFormat: Text.PlainText
                text: root.cpuScore + "/100"
                font.pixelSize: 10
                font.bold: true
                color: "#38bdf8"
              }
            }
            Rectangle {
              width: parent.width
              height: 5
              radius: 2.5
              color: "#1e293b"
              Rectangle {
                width: Math.min(parent.width, Math.max(0, (root.cpuScore / 100) * parent.width))
                height: parent.height
                radius: 2.5
                color: "#38bdf8"
              }
            }
          }

          // GPU Bar
          Column {
            width: parent.width
            spacing: 2
            RowLayout {
              width: parent.width
              Text {
                textFormat: Text.PlainText
                text: "GPU: " + root.gpuDesc
                font.pixelSize: 10
                color: "#cbd5e1"
                Layout.fillWidth: true
                elide: Text.ElideRight
              }
              Text {
                textFormat: Text.PlainText
                text: root.gpuScore + "/100"
                font.pixelSize: 10
                font.bold: true
                color: "#818cf8"
              }
            }
            Rectangle {
              width: parent.width
              height: 5
              radius: 2.5
              color: "#1e293b"
              Rectangle {
                width: Math.min(parent.width, Math.max(0, (root.gpuScore / 100) * parent.width))
                height: parent.height
                radius: 2.5
                color: "#818cf8"
              }
            }
          }

          // RAM & Display in compact row
          RowLayout {
            width: parent.width
            spacing: 12

            // RAM
            Column {
              Layout.fillWidth: true
              spacing: 2
              RowLayout {
                width: parent.width
                Text {
                  textFormat: Text.PlainText
                  text: "RAM: " + root.ramDesc
                  font.pixelSize: 10
                  color: "#cbd5e1"
                  Layout.fillWidth: true
                }
                Text {
                  textFormat: Text.PlainText
                  text: root.ramScore + "/100"
                  font.pixelSize: 10
                  font.bold: true
                  color: "#22c55e"
                }
              }
              Rectangle {
                width: parent.width
                height: 5
                radius: 2.5
                color: "#1e293b"
                Rectangle {
                  width: Math.min(parent.width, Math.max(0, (root.ramScore / 100) * parent.width))
                  height: parent.height
                  radius: 2.5
                  color: "#22c55e"
                }
              }
            }

            // Display
            Column {
              Layout.fillWidth: true
              spacing: 2
              RowLayout {
                width: parent.width
                Text {
                  textFormat: Text.PlainText
                  text: "Display: " + root.displayDesc
                  font.pixelSize: 10
                  color: "#cbd5e1"
                  Layout.fillWidth: true
                }
                Text {
                  textFormat: Text.PlainText
                  text: root.displayScore + "/100"
                  font.pixelSize: 10
                  font.bold: true
                  color: "#f59e0b"
                }
              }
              Rectangle {
                width: parent.width
                height: 5
                radius: 2.5
                color: "#1e293b"
                Rectangle {
                  width: Math.min(parent.width, Math.max(0, (root.displayScore / 100) * parent.width))
                  height: parent.height
                  radius: 2.5
                  color: "#f59e0b"
                }
              }
            }
          }
        }
      }

      // OmaStat Community Survey Action Button
      Button {
        width: parent.width
        text: root.isSubmittingSurvey ? "⏳ Submitting to OmaStat..." : "🌐 Submit Anonymous Specs to OmaStat Survey"
        enabled: !root.isSubmittingSurvey
        color: "#2563eb"
        onClicked: root.triggerSurvey()
      }

      // Survey Status Feedback
      Text {
        visible: root.surveyStatusMsg.length > 0
        textFormat: Text.PlainText
        text: root.surveyStatusMsg
        font.pixelSize: 10
        color: "#4ade80"
        anchors.horizontalCenter: parent.horizontalCenter
      }

      // Open Detailed Terminal Dashboard & Ladder Button
      Button {
        width: parent.width
        text: "⚡ Open Detailed CLI Benchmark & Ladder"
        onClicked: root.launchDashboard()
      }
    }
  }
}
