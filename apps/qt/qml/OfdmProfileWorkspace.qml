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
    property url initialFolder
    readonly property var chart: JSON.parse(service.chart_json)
    property alias inputsVisible: inputs.visible
    readonly property Item inputSurface: inputs.contentItem
    readonly property var capturePicker: inputs.filePicker
    readonly property Item resultsSurface: results.contentItem
    readonly property bool popupVisible: inputs.visible || results.visible
    function openInputs() { inputs.open() }
    function openResults() { results.open() }
    function closeDialogs() { inputs.close(); results.close() }
    signal inputCompleted()
    spacing: 12
    RowLayout {
        Layout.fillWidth: true
        Label { text: "OFDM Profile Performance"; color:workspace.theme.text;font.pixelSize:20;font.bold:true }
        Item { Layout.fillWidth: true }
        ThemeButton { objectName:"ofdmOpenInputs";theme:workspace.theme;text:"Select captures…";enabled:!workspace.service.busy;onClicked:inputs.open() }
        ThemeButton {
            objectName:"ofdmFecPerformance";theme:workspace.theme;text:"FEC Performance"
            enabled:workspace.service.prepared && (!workspace.service.busy || workspace.service.running)
            onClicked: {
                results.open()
                if (!workspace.service.busy && !JSON.parse(workspace.service.result_json).status) workspace.service.run()
            }
        }
    }
    Flow {
        Layout.fillWidth:true;Layout.preferredHeight:childrenRect.height;spacing:8
        visible:!!workspace.chart.viewport
        Repeater { model:workspace.chart.chips || [];MetadataChip { required property var modelData;theme:workspace.theme;title:modelData.title;value:modelData.value } }
        MetadataChip { theme:workspace.theme;title:"Receiver";value:workspace.chart.mce ? "MCE" : workspace.chart.supplied_channel ? "Supplied CH-EST" : "Unity channel" }
    }
    RowLayout {
        visible:!!workspace.chart.viewport
        Label {text:"RxMER";color:workspace.theme.chartLineColor}
        Label {text:"Profiles · Shannon reference (dB)";color:workspace.theme.muted}
        Label {text:"PLC shaded";color:workspace.theme.muted}
        Item {Layout.fillWidth:true}
    }
    Flow {
        visible: !!workspace.chart.viewport
        Layout.fillWidth: true; Layout.preferredHeight: childrenRect.height; spacing: 12
        Label { text: "Profiles to test"; color: workspace.theme.muted; topPadding: 8 }
        Repeater {
            model: workspace.chart.available_profiles || []
            ThemeCheckBox {
                theme: workspace.theme
                text: "Profile " + modelData
                textColor: workspace.theme.profileColor(modelData+1)
                required property var modelData
                objectName: "ofdmRunProfile" + modelData
                enabled: workspace.service.selection_available && !workspace.service.busy
                checked: (workspace.chart.selected_profiles || []).indexOf(modelData) >= 0
                onClicked: {
                    const selected = (workspace.chart.selected_profiles || []).slice()
                    const index = selected.indexOf(modelData)
                    if (index >= 0) selected.splice(index,1)
                    else selected.push(modelData)
                    workspace.service.select_profiles(JSON.stringify(selected))
                }
            }
        }
    }
    Label {
        visible:!workspace.chart.viewport
        text:"Select a modulation profile and RxMER capture to begin."
        color:workspace.theme.muted
        Layout.alignment:Qt.AlignCenter
        Layout.fillHeight:true
    }
    MeasurementChart {
        objectName:"ofdmProfileChart"
        visible:!!workspace.chart.viewport
        Layout.fillWidth:true;Layout.fillHeight:true
        theme:workspace.theme
        viewport:workspace.chart.viewport || ({start:0,count:0,total:0,points:[]})
        extraSeries:(workspace.chart.profiles || []).map(function(profile) {
            return {points:profile.points,color:workspace.theme.profileColor(profile.profile_id+1),step:true}
        })
        shadedBands:workspace.chart.plc || []
        axisLabel:"MER / profile reference";unit:"dB";minimum:0;maximum:64
        enabled:!workspace.service.busy
        onRangeRequested:(start,count)=>workspace.service.show_range(start,count)
    }
    OfdmProfileInputDialog {
        id:inputs;theme:workspace.theme;service:workspace.service;initialFolder:workspace.initialFolder
        parent:Overlay.overlay
        onCompleted:workspace.inputCompleted()
    }
    OfdmFecResultsDialog { id:results;theme:workspace.theme;service:workspace.service;parent:Overlay.overlay }
}
