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
    readonly property var configuration: JSON.parse(service.state_json)
    readonly property var providers: configuration.providers || []
    readonly property var selectedProvider: providers[providerChoice.currentIndex] || ({})
    property ThemePalette themePalette: ThemePalette { targetControl: dialog; theme: dialog.theme }
    objectName: "aiServicesDialog"
    title: "AI Assistance"
    anchors.centerIn: parent
    width: Math.min(600, parent.width - 32)
    modal: true
    closePolicy: Popup.CloseOnEscape
    background: Rectangle { radius: 10; color: dialog.theme.panel; border.color: dialog.theme.line }
    header: Label { text: dialog.title; padding: 16; color: dialog.theme.text; font.bold: true }

    function selectProvider() {
        apiKey.clear()
        modelName.text = selectedProvider.model || ""
        service.clear_error()
    }
    onOpened: {
        const index = providers.findIndex(function(provider) { return provider.id === configuration.selected })
        providerChoice.currentIndex = Math.max(0, index)
        selectProvider()
    }
    onClosed: {
        if (service.busy) service.cancel()
        keyPicker.close()
        apiKey.clear()
    }
    FileDialog {
        id: keyPicker
        objectName: "aiKeyFilePicker"
        title: "Open API key file"
        fileMode: FileDialog.OpenFile
        nameFilters: ["API key files (*.txt *.key)", "All files (*)"]
        onAccepted: {
            apiKey.clear()
            if (dialog.visible) {
                apiKey.text = dialog.service.read_key_file(selectedFile)
            }
            selectedFile = ""
        }
        onRejected: selectedFile = ""
    }

    contentItem: ColumnLayout {
        spacing: 12
        Label {
            visible: !dialog.service.demo_mode
            text: "Choose your provider, then enter its exact model ID and an API key from its developer console."
            color: dialog.theme.text
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
        Label {
            visible: !dialog.service.demo_mode
            text: "Paste the key or open a .txt/.key file containing only the key on one line. A chat subscription may not include API access."
            color: dialog.theme.muted
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
        GridLayout {
            visible: !dialog.service.demo_mode
            enabled: !dialog.service.demo_mode
            columns: 2
            columnSpacing: 16
            rowSpacing: 12
            Layout.fillWidth: true
            Label { text: "Provider"; color: dialog.theme.text }
            ThemeComboBox {
                id: providerChoice
                objectName: "aiProvider"
                theme: dialog.theme
                model: dialog.providers
                textRole: "label"
                Layout.fillWidth: true
                onActivated: dialog.selectProvider()
            }
            Label { text: "Model"; color: dialog.theme.text }
            ThemeTextField {
                id: modelName
                objectName: "aiModel"
                theme: dialog.theme
                placeholderText: "Enter a model ID from your provider"
                maximumLength: 128
                Layout.fillWidth: true
                Accessible.name: "AI model identifier"
            }
            Label { text: "API key"; color: dialog.theme.text }
            RowLayout {
                Layout.fillWidth: true
                ThemeTextField {
                    id: apiKey
                    objectName: "aiApiKey"
                    theme: dialog.theme
                    echoMode: TextInput.Password
                    inputMethodHints: Qt.ImhHiddenText | Qt.ImhNoPredictiveText | Qt.ImhSensitiveData
                    maximumLength: 4096
                    placeholderText: dialog.selectedProvider.configured ? "Stored for this session; leave blank to keep" : "Paste your API key"
                    Layout.fillWidth: true
                    Accessible.name: "AI API key"
                }
                ThemeButton {
                    objectName: "aiOpenKeyFile"
                    theme: dialog.theme
                    text: "Open key file…"
                    onClicked: keyPicker.open()
                }
            }
        }
        Label {
            objectName: "aiConfigurationStatus"
            text: dialog.service.demo_mode ? "Demo active · no provider connection" : dialog.selectedProvider.configured ? "Configured · not verified" : "Not configured"
            color: dialog.selectedProvider.configured ? dialog.theme.accent : dialog.theme.muted
        }
        Label {
            text: dialog.service.demo_mode ? "Demo AI is active. No credentials or provider connection are needed. Open OFDM Profile Performance → AI Analysis to try it." : "Save stores settings until RAVEN closes. OpenAI supports connection testing and OFDM analysis; other providers are setup-only. Test connection checks model metadata without capture data or inference."
            color: dialog.theme.muted
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
        Label {
            objectName: "aiSetupError"
            visible: dialog.service.error_message.length > 0
            text: dialog.service.error_message
            color: dialog.theme.dark ? "#ffb4ab" : "#a92b26"
            wrapMode: Text.Wrap
            Layout.fillWidth: true
            Accessible.role: Accessible.AlertMessage
        }
        Label {
            text: dialog.service.operation_status || ""
            color: dialog.theme.accent
            wrapMode: Text.Wrap
            Layout.fillWidth: true
            visible: text.length > 0
        }
        RowLayout {
            visible: !dialog.service.demo_mode
            ThemeButton {
                objectName: "aiTestConnection"
                theme: dialog.theme
                text: "Test connection"
                enabled: !dialog.service.demo_mode && !!dialog.selectedProvider.configured && dialog.selectedProvider.id === "openai" && dialog.configuration.selected === "openai" && !dialog.service.busy && modelName.text === dialog.selectedProvider.model && apiKey.text.length === 0
                onClicked: dialog.service.test_connection()
            }
            ThemeButton {
                theme: dialog.theme
                text: "Cancel test"
                visible: !!dialog.service.busy
                onClicked: dialog.service.cancel()
            }
            Item { Layout.fillWidth: true }
        }
        RowLayout {
            visible: !dialog.service.demo_mode
            ThemeButton {
                objectName: "aiForgetProvider"
                theme: dialog.theme
                text: "Forget provider"
                enabled: !!dialog.selectedProvider.configured && !dialog.service.busy
                onClicked: {
                    apiKey.clear()
                    if (dialog.service.forget(dialog.selectedProvider.id)) modelName.clear()
                }
            }
            ThemeButton {
                objectName: "aiDisable"
                theme: dialog.theme
                text: "Disable AI Assistance"
                enabled: !!dialog.configuration.enabled && !dialog.service.busy
                onClicked: dialog.service.set_enabled(false)
            }
            Item { Layout.fillWidth: true }
        }
    }
    footer: DialogButtonBox {
        ThemeButton {
            objectName: "aiSaveSession"
            visible: !dialog.service.demo_mode
            theme: dialog.theme
            text: "Save for this session"
            enabled: !dialog.service.demo_mode && !dialog.service.busy && modelName.text.trim().length > 0 && (apiKey.text.length > 0 || !!dialog.selectedProvider.configured)
            onClicked: {
                const accepted = dialog.service.configure(dialog.selectedProvider.id, modelName.text, apiKey.text)
                apiKey.clear()
                if (accepted) dialog.close()
            }
        }
        ThemeButton {
            objectName: "aiSetupClose"
            theme: dialog.theme
            text: "Close"
            onClicked: dialog.close()
        }
    }
}
