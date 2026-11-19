/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "ai/connection"

Dialog {
    id: dialog
    required property Theme theme
    required property var service
    readonly property var configuration: JSON.parse(service.state_json)
    readonly property var providers: configuration.providers || []
    readonly property var saved: JSON.parse(service.saved_json || "{}")
    readonly property bool remembered: !!(saved.providers || {})[selectedProvider.id]
    readonly property var chatgpt: configuration.chatgpt || ({})
    readonly property var selectedProvider: providers[providerChoice.currentIndex] || ({})
    readonly property bool subscriptionSelected: selectedProvider.id === "chatgpt"
    readonly property bool connectionSupported: subscriptionSelected || selectedProvider.id === "openai"
    property alias providerSettingsDialog: providerSettings
    property ThemePalette themePalette: ThemePalette { targetControl: dialog; theme: dialog.theme }
    objectName: "aiServicesDialog"
    title: "AI Assistance"
    anchors.centerIn: parent
    width: Math.min(620, parent.width - 32)
    modal: true
    closePolicy: Popup.CloseOnEscape
    background: Rectangle { radius: 10; color: dialog.theme.panel; border.color: dialog.theme.line }
    header: Label { text: dialog.title; padding: 16; color: dialog.theme.text; font.bold: true }

    function selectProvider() {
        providerSettings.close()
        service.clear_error()
        rememberSignIn.checked = remembered
    }
    function configureProvider() {
        if (!service.demo_mode && !service.busy) {
            providerSettings.open()
        }
    }
    function connectProvider() {
        if (service.demo_mode || service.busy || !connectionSupported) {
            return
        }
        service.clear_error()
        if (remembered && !selectedProvider.configured && service.reauth_provider !== selectedProvider.id) {
            service.restore_connection(selectedProvider.id)
        } else if (subscriptionSelected) {
            service.remember_next_signin(rememberSignIn.checked)
            const address = service.sign_in_chatgpt()
            if (address.length > 0 && !Qt.openUrlExternally(address)) {
                service.cancel()
            }
        } else if (!selectedProvider.configured) {
            configureProvider()
        } else if (service.configure(selectedProvider.id, selectedProvider.model, "")) {
            service.test_connection()
        }
    }
    onOpened: {
        let index = providers.findIndex(function(provider) { return provider.id === (configuration.selected || saved.preferred) })
        if (index < 0) {
            index = providers.findIndex(function(provider) { return provider.id === "chatgpt" })
        }
        providerChoice.currentIndex = Math.max(0, index)
        selectProvider()
        service.check_credential_status()
    }
    Timer {
        interval: 60000
        repeat: true
        running: dialog.visible && !dialog.service.demo_mode && !dialog.service.busy && dialog.configuration.selected === "chatgpt" && !!dialog.chatgpt.signed_in
        onTriggered: dialog.service.check_credential_status()
    }
    Connections {
        target: dialog.service
        function onState_jsonChanged() {
            if (dialog.visible && dialog.configuration.selected === "chatgpt" && !providerSettings.visible) {
                providerChoice.currentIndex = dialog.providers.findIndex(function(provider) { return provider.id === "chatgpt" })
            }
        }
        function onDemo_modeChanged() {
            if (dialog.service.demo_mode) {
                providerSettings.close()
            }
        }
    }
    onClosed: {
        providerSettings.close()
        if (service.busy) {
            service.cancel()
        }
    }
    AiProviderSettingsDialog {
        id: providerSettings
        theme: dialog.theme
        service: dialog.service
        provider: dialog.selectedProvider
        parent: dialog.parent
    }
    contentItem: ColumnLayout {
        spacing: 12
        Label {
            text: dialog.service.demo_mode ? "Local Demo AI · no connection needed" : "Choose a provider, then connect."
            color: dialog.theme.text
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
        RowLayout {
            visible: !dialog.service.demo_mode
            Layout.fillWidth: true
            Label { text: "Provider"; color: dialog.theme.text }
            ThemeComboBox {
                id: providerChoice
                objectName: "aiProvider"
                theme: dialog.theme
                enabled: !dialog.service.busy
                model: dialog.providers
                textRole: "label"
                Layout.fillWidth: true
                onActivated: dialog.selectProvider()
            }
        }
        Label {
            objectName: "aiConfigurationStatus"
            text: dialog.service.demo_mode ? "Demo active · no provider connection"
                : !dialog.connectionSupported ? "Setup only · connection is not available in RAVEN yet"
                : dialog.subscriptionSelected && dialog.chatgpt.signed_in ? "Signed in to ChatGPT"
                : dialog.selectedProvider.configured ? "Configured · use Connect to check access" : "Not connected"
            color: dialog.theme.muted
            textFormat: Text.PlainText
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
        Label {
            visible: !dialog.service.demo_mode
            text: dialog.subscriptionSelected ? "Uses browser sign-in and your eligible ChatGPT plan. Choose your model in Key & model."
                : "Uses an API key and model ID from this provider. API usage has separate billing."
            color: dialog.theme.muted
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
        Label {
            visible: !dialog.service.demo_mode && !!dialog.selectedProvider.model
            text: "Model: " + (dialog.selectedProvider.model || "")
            textFormat: Text.PlainText
            color: dialog.theme.text
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
        CheckBox {
            id: rememberSignIn
            objectName: "aiRememberSignIn"
            visible: !dialog.service.demo_mode && dialog.subscriptionSelected
            enabled: !dialog.service.busy
            text: "Remember this connection in the OS credential vault"
            palette: dialog.palette
        }
        Label {
            objectName: "aiSavedStatus"
            visible: !dialog.service.demo_mode && dialog.remembered
            text: "Saved connection available · Connect restores it without re-entering credentials."
            color: dialog.theme.muted
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
        RowLayout {
            visible: !dialog.service.demo_mode
            ThemeButton {
                objectName: "aiConnect"
                theme: dialog.theme
                text: "Connect"
                enabled: !dialog.service.busy && dialog.connectionSupported
                onClicked: dialog.connectProvider()
            }
            ThemeButton {
                objectName: "aiConfigureProvider"
                theme: dialog.theme
                text: "Key & model…"
                enabled: !dialog.service.busy && !!dialog.selectedProvider.id
                onClicked: dialog.configureProvider()
            }
            ThemeButton {
                objectName: "aiDisconnect"
                theme: dialog.theme
                text: "Forget connection"
                enabled: !dialog.service.busy && (!!dialog.selectedProvider.configured || dialog.remembered)
                onClicked: {
                    dialog.service.forget_connection(dialog.selectedProvider.id)
                }
            }
            ThemeButton {
                objectName: "aiCancelConnection"
                theme: dialog.theme
                text: "Cancel"
                visible: dialog.service.busy
                onClicked: dialog.service.cancel()
            }
        }
        Label {
            objectName: "aiCredentialStatus"
            text: dialog.service.credential_status || ""
            visible: text.length > 0 && !dialog.service.demo_mode
            color: dialog.theme.muted
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
        Label {
            objectName: "aiSetupError"
            visible: dialog.service.error_message.length > 0
            text: dialog.service.error_message
            color: dialog.theme.dark ? "#ffb4ab" : "#a92b26"
            textFormat: Text.PlainText
            wrapMode: Text.Wrap
            Layout.fillWidth: true
            Accessible.role: Accessible.AlertMessage
        }
        Label {
            text: dialog.service.operation_status || ""
            color: dialog.theme.accent
            textFormat: Text.PlainText
            wrapMode: Text.Wrap
            Layout.fillWidth: true
            visible: text.length > 0 && (dialog.service.busy || dialog.configuration.selected === dialog.selectedProvider.id)
        }
        Label {
            text: dialog.service.demo_mode ? "Open OFDM Profile Performance → AI Analysis to try the local demo."
                : "Remembered credentials use the OS vault; others last until RAVEN closes. Connecting does not send capture data."
            color: dialog.theme.muted
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
    }
    footer: DialogButtonBox {
        ThemeButton {
            objectName: "aiDisable"
            theme: dialog.theme
            text: "Disable AI"
            visible: !dialog.service.demo_mode
            enabled: !!dialog.configuration.enabled && !dialog.service.busy
            onClicked: dialog.service.set_enabled(false)
        }
        ThemeButton {
            objectName: "aiSetupClose"
            theme: dialog.theme
            text: "Close"
            onClicked: dialog.close()
        }
    }
}
