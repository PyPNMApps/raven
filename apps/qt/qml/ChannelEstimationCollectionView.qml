/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
ColumnLayout {
    id: workspace
    required property Theme theme
    required property var service
    readonly property var catalog: JSON.parse(service.catalog_json || "{}")
    readonly property var viewport: JSON.parse(service.view_json || "{}")
    readonly property bool working: !!service.busy
    readonly property bool hasResult: !!viewport.group
    property bool playbackMode: false
    property bool playbackBlocked: false
    function pausePlayback() { playback.playing = false }
    function showFrame(frame) {
        if (!working && selectedGroup >= 0) service.show_frame(selectedGroup, frame)
    }
    onPlaybackModeChanged: {
        pausePlayback()
        if (selectedGroup >= 0) {
            if (playbackMode) showFrame(0)
            else service.select_group(selectedGroup)
        }
    }
    property string selectedMac: ""
    property int selectedGroup: -1
    readonly property var devices: Array.from(new Set((catalog.groups || []).map(group => group.mac_address)))
    readonly property var groups: (catalog.groups || []).filter(group => group.mac_address === selectedMac)
    readonly property bool popupVisible: deviceDialog.visible || detailsDialog.visible
    signal chooseDirectory()
    signal chooseArchive()
    signal exportRequested()
    onCatalogChanged: {
        pausePlayback()
        selectedGroup = -1
        selectedMac = ""
        if ((catalog.groups || []).length > 0) {
            Qt.callLater(function() {
                if (workspace.visible && workspace.devices.length > 0) {
                    deviceChoice.currentIndex = 0
                    deviceDialog.open()
                }
            })
        }
    }
    onVisibleChanged: {
        if (!visible) {
            deviceDialog.close()
            detailsDialog.close()
        } else if (selectedMac.length === 0 && devices.length > 0) {
            deviceChoice.currentIndex = 0
            deviceDialog.open()
        }
    }
    function selectGroup(mac, groupId) {
        if (working) return
        pausePlayback()
        const group = (catalog.groups || []).find(group => group.id === groupId && group.mac_address === mac)
        if (!group) return
        deviceDialog.close()
        selectedMac = mac
        selectedGroup = groupId
        if (playbackMode) service.show_frame(groupId, 0)
        else service.select_group(groupId)
    }
    spacing: 10
    Label { text: workspace.playbackMode ? "DS Channel Estimation · Playback" : "DS Channel Estimation · Multi Capture"; color: workspace.theme.text; font.pixelSize: 20; font.bold: true }
    Flow {
        Layout.fillWidth: true
        spacing: 8
        ThemeButton { objectName: "chooseChannelEstimationDirectory"; theme: workspace.theme; text: "Choose folder…"; enabled: !workspace.working; onClicked: workspace.chooseDirectory() }
        ThemeButton { objectName: "chooseChannelEstimationArchive"; theme: workspace.theme; text: "Open archive…"; enabled: !workspace.working; onClicked: workspace.chooseArchive() }
        ThemeButton { objectName: "chooseChannelEstimationMac"; theme: workspace.theme; text: "Choose MAC…"; enabled: !workspace.working && workspace.devices.length > 0; onClicked: deviceDialog.open() }
        ThemeComboBox { objectName: "channelEstimationGroupChoice"; theme: workspace.theme; model: workspace.groups; textRole: "label"; currentIndex: workspace.groups.findIndex(group => group.id === workspace.selectedGroup); enabled: !workspace.working && workspace.groups.length > 0; implicitWidth: 320; onActivated: index => workspace.selectGroup(workspace.selectedMac,workspace.groups[index].id) }
        ThemeButton { theme: workspace.theme; text: "Import details…"; enabled: !!workspace.catalog.groups; onClicked: detailsDialog.open() }
        ThemeButton { objectName: "exportChannelEstimationCollection"; theme: workspace.theme; text: "Export JSON"; enabled: workspace.hasResult && !workspace.working; onClicked: workspace.exportRequested() }
        ThemeButton { theme: workspace.theme; text: "Cancel"; visible: workspace.working; onClicked: workspace.service.cancel() }
        BusyIndicator { running: workspace.working; visible: running; implicitWidth: 28; implicitHeight: 28 }
    }
    Flow {
        Layout.fillWidth: true
        visible: workspace.hasResult
        spacing: 8
        MetadataChip { theme: workspace.theme; title: "MAC"; value: workspace.hasResult ? workspace.viewport.group.mac_address : "—" }
        MetadataChip { theme: workspace.theme; title: "Channel"; value: workspace.hasResult ? workspace.viewport.group.channel_id : "—" }
        MetadataChip { theme: workspace.theme; title: "Captures"; value: workspace.viewport.capture_count || 0 }
        MetadataChip { theme: workspace.theme; title: "Spacing"; value: workspace.hasResult ? workspace.viewport.group.spacing_hz/1000 + " kHz" : "—" }
        MetadataChip { theme: workspace.theme; title: "Carriers"; value: workspace.viewport.total || 0 }
    }
    Flow {
        Layout.fillWidth: true
        visible: workspace.hasResult
        spacing: 8
        ThemeButton { objectName: "channelEstimationMagnitude"; theme: workspace.theme; text: "Magnitude"; enabled: !workspace.working; borderHighlighted: !workspace.viewport.delay; onClicked: workspace.service.select_metric(false) }
        ThemeButton { objectName: "channelEstimationGroupDelay"; theme: workspace.theme; text: "Group delay"; enabled: !workspace.working; borderHighlighted: !!workspace.viewport.delay; onClicked: workspace.service.select_metric(true) }
        ThemeButton { objectName: "channelEstimationMinimum"; theme: workspace.theme; text: "Min"; implicitWidth: 104; visible: !workspace.viewport.delay && !workspace.playbackMode; checkable: true; checked: true; outlineColor: workspace.theme.rxmerMinimum; borderHighlighted: checked; onClicked: plot.showRxmerMinimum = checked }
        ThemeButton { objectName: "channelEstimationAverage"; theme: workspace.theme; text: "Avg I/Q"; implicitWidth: 104; visible: !workspace.viewport.delay && !workspace.playbackMode; checkable: true; checked: true; outlineColor: workspace.theme.rxmerAverage; borderHighlighted: checked; onClicked: plot.showRxmerAverage = checked }
        ThemeButton { objectName: "channelEstimationMaximum"; theme: workspace.theme; text: "Max"; implicitWidth: 104; visible: !workspace.viewport.delay && !workspace.playbackMode; checkable: true; checked: true; outlineColor: workspace.theme.rxmerMaximum; borderHighlighted: checked; onClicked: plot.showRxmerMaximum = checked }
        ThemeButton { objectName: "resetChannelEstimationZoom"; theme: workspace.theme; text: "Reset zoom"; enabled: !workspace.working; onClicked: plot.resetZoom() }
    }
    RxMerPlaybackControls {
        id: playback
        theme: workspace.theme
        namePrefix: "channelEstimation"
        visible: workspace.playbackMode && workspace.hasResult
        frameIndex: workspace.viewport.frame_index || 0
        frameCount: workspace.viewport.frame_count || 0
        suspended: !workspace.visible || !workspace.playbackMode || workspace.working || workspace.playbackBlocked || workspace.popupVisible
        onFrameRequested: frame => workspace.showFrame(frame)
    }
    Label {
        visible: workspace.playbackMode && workspace.hasResult
        text: workspace.viewport.capture_time === undefined ? "" : "Captured " + new Date(workspace.viewport.capture_time * 1000).toISOString().replace("T", " ").replace(".000Z", " UTC")
        color: workspace.theme.muted
    }
    MeasurementChart {
        id: plot
        objectName: "channelEstimationCollectionChart"
        visible: workspace.hasResult
        enabled: !workspace.working
        Layout.fillWidth: true
        Layout.fillHeight: true
        theme: workspace.theme
        viewport: workspace.hasResult ? workspace.viewport : ({points:[],start:0,count:0,total:0})
        axisLabel: workspace.viewport.delay ? "Group delay" : "Magnitude"
        unit: workspace.viewport.delay ? "µs" : "dB"
        minimum: workspace.viewport.minimum === undefined ? -1 : workspace.viewport.minimum
        maximum: workspace.viewport.maximum === undefined ? 1 : workspace.viewport.maximum
        onRangeRequested: (start,count) => workspace.service.show_range(start,count)
    }
    Item { visible: !workspace.hasResult; Layout.fillHeight: true }
    Dialog {
        id: deviceDialog
        objectName: "channelEstimationDeviceDialog"
        property ThemePalette themePalette: ThemePalette { targetControl: deviceDialog; theme: workspace.theme }
        parent: Overlay.overlay
        anchors.centerIn: parent
        width: 410
        modal: true
        title: "Select channel-estimation MAC address"
        background: Rectangle { color: workspace.theme.panel; border.color: workspace.theme.line; radius: 10 }
        contentItem: ThemeComboBox { id: deviceChoice; objectName: "channelEstimationDeviceChoice"; theme: workspace.theme; model: workspace.devices }
        footer: DialogButtonBox {
            ThemeButton { objectName:"channelEstimationDeviceContinue"; theme: workspace.theme; text: "Continue"; enabled: deviceChoice.currentIndex >= 0; DialogButtonBox.buttonRole: DialogButtonBox.AcceptRole }
            ThemeButton { theme: workspace.theme; text: "Close"; DialogButtonBox.buttonRole: DialogButtonBox.RejectRole }
            onAccepted: {
                workspace.selectedMac = workspace.devices[deviceChoice.currentIndex]
                deviceDialog.close()
                if (workspace.groups.length > 0) workspace.selectGroup(workspace.selectedMac,workspace.groups[0].id)
            }
            onRejected: deviceDialog.close()
        }
    }
    Dialog {
        id: detailsDialog
        property ThemePalette themePalette: ThemePalette { targetControl: detailsDialog; theme: workspace.theme }
        parent: Overlay.overlay
        anchors.centerIn: parent
        width: Math.min(700,parent ? parent.width-40 : 700)
        height: Math.min(420,parent ? parent.height-40 : 420)
        modal: true
        title: "Channel-estimation import details"
        background: Rectangle { color: workspace.theme.panel; border.color: workspace.theme.line; radius: 10 }
        contentItem: ScrollView { ThemeTextArea { theme: workspace.theme; readOnly: true; wrapMode: TextEdit.Wrap; text: "Accepted: "+(workspace.catalog.accepted_count || 0)+"\nDuplicates: "+(workspace.catalog.duplicate_count || 0)+"\nSkipped: "+(workspace.catalog.skipped_count || 0)+"\n\n"+(workspace.catalog.issues || []).join("\n") } }
        footer: DialogButtonBox { ThemeButton { theme: workspace.theme; text: "Close"; DialogButtonBox.buttonRole: DialogButtonBox.RejectRole } onRejected: detailsDialog.close() }
    }
}
