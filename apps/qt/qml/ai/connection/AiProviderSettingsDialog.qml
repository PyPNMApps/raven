/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import "../.."

Dialog {
    id: dialog
    required property Theme theme
    required property var service
    required property var provider
    readonly property bool subscription: provider.id === "chatgpt"
    readonly property var account: JSON.parse(service.state_json).chatgpt || ({})
    readonly property var saved: JSON.parse(service.saved_json || "{}")
    readonly property bool remembered: !!(saved.providers || {})[provider.id]
    property string formProvider: ""
    readonly property var models: account.models || []
    property ThemePalette themePalette: ThemePalette { targetControl: dialog; theme: dialog.theme }
    objectName: "aiProviderSettings"
    title: (provider.label || "AI") + " · Key & model"
    anchors.centerIn: parent
    width: Math.min(620, parent.width - 48)
    modal: true
    closePolicy: Popup.CloseOnEscape
    background: Rectangle { radius: 10; color: dialog.theme.panel; border.color: dialog.theme.line }
    header: Label { text: dialog.title; textFormat: Text.PlainText; padding: 16; color: dialog.theme.text; font.bold: true }

    onOpened: {
        apiKey.clear()
        formProvider = provider.id || ""
        rememberKey.checked = remembered
        modelName.text = provider.model || (saved.providers || {})[provider.id] || ""
        service.clear_error()
    }
    onProviderChanged: {
        if (visible && provider.id !== formProvider) {
            close()
        }
    }
    onClosed: {
        keyPicker.close()
        apiKey.clear()
    }
    Connections {
        target: dialog.service
        function onState_jsonChanged() {
            if (dialog.visible && !modelName.text.length && dialog.provider.configured) {
                modelName.text = dialog.provider.model || ""
            }
        }
    }
    FileDialog {
        id: keyPicker
        objectName: "aiKeyFilePicker"
        title: "Open API key file"
        fileMode: FileDialog.OpenFile
        nameFilters: ["API key files (*.txt *.key)", "All files (*)"]
        onAccepted: {
            apiKey.clear()
            if (dialog.visible && !dialog.service.demo_mode && !dialog.subscription) {
                apiKey.text = dialog.service.read_key_file(selectedFile)
            }
            selectedFile = ""
        }
        onRejected: selectedFile = ""
    }
    contentItem: ColumnLayout {
        spacing: 12
        Label {
            text: dialog.subscription
                ? "Connect from AI Assistance first. Choose a model returned by your ChatGPT account; no API key is needed."
                : "Enter your provider's API key and exact model ID. Use Remember to store them in the OS credential vault. Saving never contacts the AI provider."
            color: dialog.theme.muted
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
        Label { text: "Model"; color: dialog.theme.text }
        ThemeComboBox {
            id: accountModel
            objectName: "aiChatGptModel"
            visible: dialog.subscription
            theme: dialog.theme
            model: dialog.models
            textRole: "label"
            enabled: !!dialog.account.signed_in && !dialog.service.busy
            currentIndex: Math.max(0, dialog.models.findIndex(function(model) { return model.id === dialog.account.model }))
            Layout.fillWidth: true
        }
        ThemeTextField {
            id: modelName
            objectName: "aiModel"
            visible: !dialog.subscription
            theme: dialog.theme
            enabled: !dialog.service.busy
            placeholderText: "Exact model ID from your provider"
            maximumLength: 128
            Accessible.name: "AI model identifier"
            Layout.fillWidth: true
        }
        Label { text: "API key"; color: dialog.theme.text; visible: !dialog.subscription }
        RowLayout {
            visible: !dialog.subscription
            Layout.fillWidth: true
            ThemeTextField {
                id: apiKey
                objectName: "aiApiKey"
                theme: dialog.theme
                enabled: !dialog.service.busy
                echoMode: TextInput.Password
                inputMethodHints: Qt.ImhHiddenText | Qt.ImhNoPredictiveText | Qt.ImhSensitiveData
                maximumLength: 4096
                placeholderText: dialog.provider.configured ? "Leave blank to keep this session's key" : "Paste your API key"
                Accessible.name: "AI API key"
                Layout.fillWidth: true
            }
            ThemeButton {
                objectName: "aiOpenKeyFile"
                theme: dialog.theme
                text: "Open file…"
                enabled: !dialog.service.busy
                onClicked: keyPicker.open()
            }
        }
        CheckBox {
            id: rememberKey
            objectName: "aiRememberKey"
            visible: !dialog.subscription
            enabled: !dialog.service.busy && !dialog.service.demo_mode
            text: "Remember this connection in the OS credential vault"
            palette: dialog.palette
        }
        ThemeButton {
            objectName: "aiRestoreSavedKey"
            theme: dialog.theme
            text: "Restore saved connection"
            visible: dialog.remembered && !dialog.provider.configured
            enabled: !dialog.service.busy
            onClicked: dialog.service.restore_connection(dialog.provider.id)
        }
        Label {
            objectName: "aiProviderSettingsError"
            text: dialog.service.error_message
            visible: text.length > 0
            color: dialog.theme.dark ? "#ffb4ab" : "#a92b26"
            textFormat: Text.PlainText
            wrapMode: Text.Wrap
            Layout.fillWidth: true
            Accessible.role: Accessible.AlertMessage
        }
    }
    footer: DialogButtonBox {
        id: actionButtons

        // Own the visual host explicitly: a deferred style host can remain null.
        contentItem: ListView {
            implicitWidth: contentWidth
            model: actionButtons.contentModel
            spacing: actionButtons.spacing
            orientation: ListView.Horizontal
            boundsBehavior: Flickable.StopAtBounds
            snapMode: ListView.SnapToItem
        }
        ThemeButton {
            objectName: "aiSaveSession"
            DialogButtonBox.buttonRole: DialogButtonBox.ActionRole
            theme: dialog.theme
            text: "Save"
            enabled: !dialog.service.demo_mode && !dialog.service.busy && (dialog.subscription
                ? !!dialog.account.signed_in && dialog.models.length > 0
                : modelName.text.trim().length > 0 && (apiKey.text.length > 0 || !!dialog.provider.configured))
            onClicked: {
                const providerId = dialog.provider.id
                const remember = dialog.subscription ? dialog.remembered : rememberKey.checked
                const accepted = dialog.subscription
                    ? dialog.service.select_chatgpt_model(dialog.models[accountModel.currentIndex].id)
                    : dialog.service.configure(dialog.provider.id, modelName.text, apiKey.text)
                apiKey.clear()
                if (accepted) {
                    dialog.service.remember_connection(providerId, remember)
                    dialog.close()
                }
            }
        }
        ThemeButton {
            theme: dialog.theme
            DialogButtonBox.buttonRole: DialogButtonBox.RejectRole
            text: "Cancel"
            onClicked: dialog.close()
        }
    }
}
