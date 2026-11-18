/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs

Dialog {
    id: dialog
    required property Theme theme
    required property var service
    required property var ofdmService
    signal setupRequested()
    readonly property var preview: JSON.parse(service.preview_json)
    readonly property var response: JSON.parse(service.report_json)
    readonly property var turns: JSON.parse(service.conversation_json)
    property ThemePalette themePalette: ThemePalette { targetControl: dialog; theme: dialog.theme }
    objectName: "aiAnalysisDialog"
    title: dialog.service.demo_mode ? "OFDM Profile Performance · Demo AI" : "OFDM Profile Performance · AI Analysis"
    anchors.centerIn: parent
    width: Math.min(1040, parent.width-32)
    height: Math.min(740, parent.height-32)
    modal: true
    closePolicy: Popup.CloseOnEscape
    background: Rectangle { radius: 10; color: dialog.theme.panel; border.color: dialog.theme.line }
    header: Label { text: dialog.title; color: dialog.theme.text; padding: 16; font.bold: true }
    function previewRequest() {
        service.prepare_analysis(ofdmService.evidence_json, question.text)
    }
    onClosed: service.invalidate()
    FileDialog {
        id: exportDialog
        title: "Export AI conversation"
        fileMode: FileDialog.SaveFile
        defaultSuffix: "json"
        nameFilters: ["JSON conversation (*.json)"]
        onAccepted: dialog.service.export_conversation(selectedFile)
    }
    contentItem: ColumnLayout {
        spacing: 10
        RowLayout {
            Layout.fillWidth: true
            Label { text: dialog.service.demo_mode ? "DEMO · Local simulated response. No API key, provider connection or AI charge." : "Review the exact request before sending. ChatGPT uses plan allowance; API-key requests may incur charges."; color: dialog.theme.muted; wrapMode: Text.Wrap; Layout.fillWidth: true }
            ThemeButton {
                theme: dialog.theme; text: "AI settings…"; enabled: !dialog.service.busy
                onClicked: { dialog.close(); dialog.setupRequested() }
            }
        }
        ThemeCheckBox {
            objectName: "aiDemoMode"
            theme: dialog.theme
            text: "Demo AI · local responses, no credentials"
            checked: !!dialog.service.demo_mode
            enabled: !dialog.service.busy && !dialog.service.demo_locked
            onClicked: dialog.service.choose_demo_mode(checked)
        }
        Label { text: dialog.service.demo_mode ? "Question · recorded for the demonstration; not interpreted by a language model" : "Question · each request uses this run only; no prior turns are sent"; color: dialog.theme.text }
        ScrollView {
            Layout.fillWidth: true; Layout.preferredHeight: 70
            ThemeTextArea {
                id: question
                objectName: "aiQuestion"
                theme: dialog.theme
                wrapMode: TextEdit.Wrap
                enabled: !dialog.service.busy
                text: "Review the selected OFDM profiles. Explain observed FEC, BER and LDPC outcomes, keeping scopes and limitations explicit."
                onTextChanged: dialog.service.invalidate()
            }
        }
        RowLayout {
            ThemeButton {
                objectName: "aiCreatePreview"; theme: dialog.theme; text: "Preview request"
                enabled: !dialog.service.busy && !!JSON.parse(dialog.ofdmService.evidence_json).run_id
                onClicked: dialog.previewRequest()
            }
            ThemeButton {
                objectName: "aiAnalyzeApproved"; theme: dialog.theme; text: dialog.service.demo_mode ? "Run approved demo" : "Analyze approved request"
                enabled: !dialog.service.busy && dialog.service.request_ready
                onClicked: dialog.service.analyze(dialog.preview.digest)
            }
            ThemeButton {
                objectName: "aiCancelAnalysis"; theme: dialog.theme; text: "Cancel"
                visible: dialog.service.busy
                onClicked: dialog.service.cancel()
            }
            BusyIndicator { visible: dialog.service.busy; running: visible; implicitWidth: 28; implicitHeight: 28 }
            Item { Layout.fillWidth: true }
        }
        Label {
            text: dialog.service.error_message || dialog.service.operation_status
            visible: text.length > 0
            color: dialog.service.error_message.length > 0 ? (dialog.theme.dark ? "#ffb4ab" : "#a92b26") : dialog.theme.accent
            wrapMode: Text.Wrap; Layout.fillWidth: true
        }
        TabBar {
            id: tabs
            Layout.fillWidth: true
            TabButton { text: "Request preview" }
            TabButton { text: "Local findings" }
            TabButton { text: "AI report" }
            TabButton { text: "Chat log (" + dialog.turns.length + ")" }
        }
        StackLayout {
            Layout.fillWidth: true; Layout.fillHeight: true
            currentIndex: tabs.currentIndex
            ColumnLayout {
                Label {
                    text: dialog.preview.destination ? dialog.preview.destination + "\nModel: " + dialog.preview.model + "\nSHA-256: " + dialog.preview.digest : "Create a preview to review the destination and complete request body."
                    color: dialog.theme.muted; wrapMode: Text.Wrap; Layout.fillWidth: true
                }
                ScrollView {
                    Layout.fillWidth: true; Layout.fillHeight: true
                    ThemeTextArea { theme: dialog.theme; readOnly: true; wrapMode: TextEdit.Wrap; text: dialog.preview.body || ""; objectName: "aiExactRequest" }
                }
            }
            ScrollView {
                ThemeTextArea { theme: dialog.theme; readOnly: true; wrapMode: TextEdit.Wrap; text: {
                    const evidence=JSON.parse(dialog.ofdmService.evidence_json)
                    return (evidence.findings || []).map(function(finding) {
                        return "Profile " + finding.profile_id + ": " + finding.text
                    }).join("\n\n") || "Local findings are unavailable for this run."
                } }
            }
            ScrollView {
                ThemeTextArea {
                    objectName: "aiReportText"; theme: dialog.theme; readOnly: true; wrapMode: TextEdit.Wrap
                    text: {
                        const report=dialog.response.report
                        if (!report) return "No accepted AI report yet."
                        const usage=dialog.response.usage || ({})
                        return (dialog.response.demo ? "DEMO · Simulated local response\n" : "AI commentary · advisory\n") + report.summary + "\n\n" + report.claims.map(function(claim) {
                            return claim.kind + ": " + claim.text + "\n" + claim.references.map(function(reference) {
                                return reference.metric_id + " = " + reference.value + " " + reference.unit + " [" + reference.scope + "]"
                            }).join("\n")
                        }).join("\n\n") + "\n\nLimitations\n" + report.limitations.join("\n") + "\n\nSuggested checks\n" + report.suggested_checks.join("\n")
                        + (dialog.response.demo ? "\n\nLocal demo · no provider or token usage." : "\n\nModel: " + dialog.response.returned_model + "\nProvider-reported tokens · input: " + (usage.input_tokens || "unavailable") + " · output: " + (usage.output_tokens || "unavailable") + " · total: " + (usage.total_tokens || "unavailable"))
                    }
                }
            }
            ScrollView {
                ThemeTextArea {
                    objectName: "aiChatLog"; theme: dialog.theme; readOnly: true; wrapMode: TextEdit.Wrap
                    text: dialog.turns.map(function(turn,index) {
                        return (turn.demo ? "DEMO · " : "") + "Exchange " + (index+1) + " · " + turn.provider + " / " + turn.returned_model + "\nQuestion: " + turn.question + "\n" + turn.report.summary + "\nRequest SHA-256: " + turn.request_digest
                    }).join("\n\n") || "Accepted exchanges for this run appear here. Export before closing RAVEN to keep a copy."
                }
            }
        }
    }
    footer: DialogButtonBox {
        ThemeButton { theme: dialog.theme; text: "Export conversation…"; enabled: dialog.turns.length > 0 && !dialog.service.busy; onClicked: exportDialog.open() }
        ThemeButton { theme: dialog.theme; text: "Close"; onClicked: dialog.close() }
    }
    Connections {
        target: dialog.service
        function onReport_jsonChanged() { if (dialog.response.report) tabs.currentIndex=2 }
    }
}
