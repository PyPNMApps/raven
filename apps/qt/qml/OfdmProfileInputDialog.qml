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
    property alias filePicker: picker
    property url initialFolder
    property ThemePalette themePalette: ThemePalette { targetControl: dialog; theme: dialog.theme }
    readonly property var inputs: JSON.parse(service.inputs_json)
    property bool awaitingPreparation: false
    property var selectedProfiles: []
    property string profileCatalogKey: ""
    function toggleProfile(profileId) {
        const next = selectedProfiles.slice()
        const index = next.indexOf(profileId)
        if (index >= 0) next.splice(index,1)
        else next.push(profileId)
        selectedProfiles = next
    }
    onInputsChanged: {
        const key = JSON.stringify([inputs.profile, inputs.profiles])
        if (profileCatalogKey !== key) {
            profileCatalogKey = key
            selectedProfiles = (inputs.profiles || []).slice()
        }
    }
    property string selectedRole: "profile"
    signal completed()
    anchors.centerIn: parent
    width: Math.min(700, parent.width - 32)
    modal: true
    title: "OFDM Profile Performance · Inputs"
    closePolicy: service.busy ? Popup.NoAutoClose : Popup.CloseOnEscape
    background: Rectangle { radius: 10; color: dialog.theme.panel; border.color: dialog.theme.line }
    header: Label { text: dialog.title; padding: 16; color: dialog.theme.text; font.bold: true }
    function selectFile(role) {
        selectedRole = role
        picker.open()
    }
    FileDialog {
        id: picker
        objectName: "ofdmInputFileDialog"
        title: "Open " + dialog.selectedRole + " PNM capture"
        currentFolder: dialog.initialFolder
        fileMode: FileDialog.OpenFile
        nameFilters: ["PNM captures (*)"]
        onAccepted: dialog.service.choose_file(dialog.selectedRole, selectedFile)
    }
    FileDialog {
        id: capturePicker
        objectName: "ofdmMultiInputFileDialog"
        title: "Select modulation profile, RxMER and optional channel estimation"
        currentFolder: dialog.initialFolder
        fileMode: FileDialog.OpenFiles
        nameFilters: ["PNM captures (*)"]
        onAccepted: dialog.service.choose_files(JSON.stringify(selectedFiles.map(function(url) { return url.toString() })), Number(tolerance.text))
    }
    Connections {
        target: dialog.service
        function onBusyChanged() {
            if (!dialog.service.busy && dialog.awaitingPreparation) {
                dialog.awaitingPreparation = false
                if (dialog.service.prepared && !dialog.service.error_message.length) {
                    dialog.close()
                    dialog.completed()
                }
            }
        }
        function onPreparedChanged() {
            if (dialog.service.prepared && dialog.awaitingPreparation && !dialog.service.busy) {
                dialog.awaitingPreparation = false
                dialog.close()
                dialog.completed()
            }
        }
    }
    contentItem: ColumnLayout {
        spacing: 12
        ThemeButton {
            objectName: "ofdmChooseCaptures"
            theme: dialog.theme
            text: "Select capture files…"
            enabled: !dialog.service.busy && tolerance.acceptableInput
            onClicked: capturePicker.open()
        }
        GridLayout {
            columns: 3
            Layout.fillWidth: true
            Repeater {
                model: [
                    { role: "profile", label: "Modulation profile · required" },
                    { role: "rxmer", label: "RxMER · required" },
                    { role: "channel", label: "Channel estimation · optional" }
                ]
                RowLayout {
                    required property var modelData
                    Layout.columnSpan: 3
                    Layout.fillWidth: true
                    Label { text: modelData.label; color: dialog.theme.text; Layout.preferredWidth: 210 }
                    Label {
                        text: dialog.inputs[modelData.role] || "Not selected"
                        color: dialog.theme.muted
                        elide: Text.ElideMiddle
                        Layout.fillWidth: true
                    }
                    ThemeButton {
                        theme: dialog.theme; text: "Browse…"; enabled: !dialog.service.busy
                        objectName: "ofdmChoose_" + modelData.role
                        onClicked: dialog.selectFile(modelData.role)
                    }
                }
            }
        }
        RowLayout {
            ThemeCheckBox {
                id: ignoreChannel
                objectName: "ofdmIgnoreChannel"
                theme: dialog.theme
                text: "Ignore channel estimation"
                enabled: !!dialog.inputs.channel && !dialog.service.busy
                checked: false
            }
            ThemeCheckBox {
                theme: dialog.theme
                text: "Use MCE"
                id: mce
                objectName: "ofdmUseMce"
                enabled: !!dialog.inputs.channel && !ignoreChannel.checked && !dialog.service.busy
                checked: false
            }
        }
        GridLayout {
            columns: 2; columnSpacing: 14; rowSpacing: 10
            Label { text: "Profiles to test"; color: dialog.theme.text; Layout.alignment: Qt.AlignTop }
            Flow {
                Layout.fillWidth: true; Layout.preferredHeight: childrenRect.height; spacing: 8
                Repeater {
                    model: dialog.inputs.profiles || []
                    ThemeCheckBox {
                        theme: dialog.theme
                        text: "Profile " + modelData
                        required property var modelData
                        objectName: "ofdmInputProfile" + modelData
                        enabled: !dialog.service.busy
                        checked: dialog.selectedProfiles.indexOf(modelData) >= 0
                        onClicked: dialog.toggleProfile(modelData)
                    }
                }
            }
            Label { text: "Max capture separation · seconds"; color: dialog.theme.text }
            ThemeTextField {
                id: tolerance; objectName: "ofdmCaptureTolerance"; theme: dialog.theme
                text: "86400"; enabled: !dialog.service.busy
                validator: IntValidator { bottom: 0; top: 2147483647 }
                Layout.preferredWidth: 180
            }
        }
        BusyIndicator { running: dialog.service.busy; visible: running; Layout.alignment: Qt.AlignHCenter }
    }
    footer: DialogButtonBox {
        ThemeButton {
            objectName: "ofdmContinue"; theme: dialog.theme; text: "Continue"
            enabled: !!dialog.inputs.profile && !!dialog.inputs.rxmer && dialog.selectedProfiles.length > 0
                && tolerance.acceptableInput && !dialog.service.busy
            onClicked: {
                dialog.awaitingPreparation = true
                dialog.service.prepare_selection(JSON.stringify(dialog.selectedProfiles), mce.enabled && mce.checked, ignoreChannel.checked, Number(tolerance.text))
            }
        }
        ThemeButton { theme: dialog.theme; text: "Close"; enabled: !dialog.service.busy; onClicked: dialog.close() }
    }
}
