/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: dialog
    required property Theme theme
    required property var service
    property ThemePalette themePalette: ThemePalette { targetControl: dialog; theme: dialog.theme }
    readonly property var performance: JSON.parse(service.result_json)
    readonly property var entries: performance.entries || []
    anchors.centerIn: parent
    width: Math.min(1100, parent.width-32)
    height: Math.min(620, parent.height-32)
    modal: true
    title: "FEC Performance · Selected profiles"
    closePolicy: Popup.CloseOnEscape
    function number(value, decimals) {
        return value === null || value === undefined ? "—" : Number(value).toLocaleString(Qt.locale(), 'f', decimals || 0)
    }
    function ber(entry) {
        const value = entry.profile ? entry.profile.post_fec_ber : null
        return value === null || value === undefined ? "—" : Number(value).toExponential(3)
    }
    background: Rectangle { radius: 10; color: dialog.theme.panel; border.color: dialog.theme.line }
    header: Label { text: dialog.title; color: dialog.theme.text; font.bold: true; padding: 16 }
    contentItem: ScrollView {
        id: scroll
        background: Rectangle { color: dialog.theme.panel }
        clip: true
        contentWidth: Math.max(960, availableWidth)
        ColumnLayout {
            width: Math.max(960, scroll.availableWidth)
            spacing: 16
            RowLayout {
                Label { text: dialog.service.running ? "Running selected profiles…" : (dialog.performance.status || "Ready"); color: dialog.theme.accent }
                Item { Layout.fillWidth: true }
                BusyIndicator { visible: dialog.service.running; running: visible; implicitWidth: 28; implicitHeight: 28 }
            }
            Label { text: "FEC · Observation window"; color: dialog.theme.text; font.bold: true }
            GridLayout {
                columns: 9; columnSpacing: 10; rowSpacing: 10; Layout.fillWidth: true
                Repeater {
                    model: ["Profile", "Outcome", "Total", "Clean", "Corrected", "UFEC", "Compared bits", "Bit errors", "BER"]
                    Label { required property var modelData; text:modelData; color:dialog.theme.muted; font.bold:true; Layout.fillWidth:true }
                }
                Repeater {
                    model: dialog.entries.length * 9
                    Label {
                        required property int index
                        readonly property var entry: dialog.entries[Math.floor(index/9)]
                        readonly property var counters: entry.fec ? entry.fec.counters : ({})
                        text: ["Profile " + entry.profile_id, entry.status,
                            dialog.number(counters.total_cw),dialog.number(counters.clean_cw),dialog.number(counters.ccw),dialog.number(counters.ufec),
                            dialog.number(entry.profile ? entry.profile.compared_bits : null),
                            dialog.number(entry.profile ? entry.profile.bit_errors : null),dialog.ber(entry)][index%9]
                        color:dialog.theme.text;Layout.fillWidth:true;Layout.preferredWidth:100
                        objectName: "ofdmFecProfile" + entry.profile_id + "Column" + (index%9)
                    }
                }
            }
            Label { text: "LDPC · All completed codewords"; color: dialog.theme.text; font.bold: true }
            GridLayout {
                columns: 7; columnSpacing: 10; rowSpacing: 10; Layout.fillWidth: true
                Repeater {
                    model: ["Profile", "Mean iterations", "Payload iterations", "Payload failures", "BCH failures", "NCP iterations", "PLC iterations"]
                    Label { required property var modelData; text:modelData; color:dialog.theme.muted; font.bold:true; Layout.fillWidth:true }
                }
                Repeater {
                    model: dialog.entries.length * 7
                    Label {
                        required property int index
                        readonly property var entry: dialog.entries[Math.floor(index/7)]
                        readonly property var ldpc: entry.ldpc || ({})
                        text: ["Profile " + entry.profile_id,dialog.number(ldpc.mean_iterations,3),dialog.number(ldpc.iteration_sum),
                            dialog.number(ldpc.failures),dialog.number(ldpc.bch_failures),dialog.number(ldpc.ncp_iterations),dialog.number(ldpc.plc_iterations)][index%7]
                        color:dialog.theme.text;Layout.fillWidth:true;Layout.preferredWidth:100
                    }
                }
            }
            Repeater {
                model: dialog.entries
                Label {
                    required property var modelData
                    visible: !!modelData.error || !!modelData.run_error
                    text: "Profile " + modelData.profile_id + ": " + JSON.stringify(modelData.error || modelData.run_error || "")
                    color:dialog.theme.text;wrapMode:Text.Wrap;Layout.fillWidth:true
                }
            }
        }
    }
    footer: DialogButtonBox {
        ThemeButton { theme:dialog.theme;text:"Stop";visible:dialog.service.running;onClicked:dialog.service.cancel() }
        ThemeButton { theme:dialog.theme;text:"Run again";enabled:dialog.service.prepared && !dialog.service.busy;onClicked:dialog.service.run() }
        ThemeButton { theme:dialog.theme;text:"Close";onClicked:dialog.close() }
    }
}
