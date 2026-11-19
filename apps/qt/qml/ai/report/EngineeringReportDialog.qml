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
    id:dialog
    required property Theme theme
    required property var service
    property var localReport: ({})
    property bool localMode:false
    readonly property var examples:JSON.parse(service.fixture_catalog)
    readonly property var report:localMode ? localReport : JSON.parse(service.model_json)
    signal currentRequested()
    objectName:"engineeringReportDialog"
    title:"Engineering reports"
    anchors.centerIn:parent
    width:Math.min(1120,parent.width-32); height:Math.min(800,parent.height-32)
    modal:true
    property ThemePalette themePalette:ThemePalette {targetControl:dialog;theme:dialog.theme}
    background:Rectangle {color:dialog.theme.panel;border.color:dialog.theme.line;radius:10}
    header:Label {text:dialog.title;color:dialog.theme.text;font.bold:true;padding:14}
    function present(json) {localReport=JSON.parse(json);localMode=true;open()}
    function scrollToEnd(){reportView.scrollToEnd()}
    function gallery(exampleId) {
        if (typeof exampleId === "string") {
            for (var index = 0; index < examples.length; ++index) {
                if (examples[index].id === exampleId) {
                    examplesChoice.currentIndex = index
                    break
                }
            }
        }
        localMode = false
        service.load_fixture(examples[examplesChoice.currentIndex].id)
        open()
    }
    onClosed: {localReport=({});service.clear()}
    FileDialog {
        id:save
        title:"Export approved example presentation"
        fileMode:FileDialog.SaveFile; defaultSuffix:"json";nameFilters:["JSON report (*.json)"]
        onAccepted:dialog.service.export_report(selectedFile)
    }
    contentItem:ColumnLayout {
        RowLayout {
            Layout.fillWidth:true
            ThemeButton {theme:dialog.theme;text:"Current analysis";onClicked:dialog.currentRequested()}
            ThemeComboBox {
                id:examplesChoice;objectName:"reportExampleChoice"
                theme:dialog.theme;model:dialog.examples;textRole:"label";Layout.fillWidth:true
                onActivated: {dialog.localMode=false;dialog.service.load_fixture(dialog.examples[currentIndex].id)}
            }
            ThemeButton {theme:dialog.theme;text:"Show offline example";onClicked:dialog.gallery()}
        }
        ThemeTextArea {
            theme:dialog.theme;visible:!!dialog.report.prepared_question;readOnly:true;wrapMode:TextEdit.Wrap;Layout.fillWidth:true
            text:dialog.report.prepared_question ? "Prepared AI question (no request sent)\n"+dialog.report.prepared_question : ""
        }
        AiReportView {
            id:reportView
            theme:dialog.theme;report:dialog.report
            errorMessage:dialog.localMode ? (dialog.report.error || "") : dialog.service.error_message
            Layout.fillWidth:true;Layout.fillHeight:true
        }
    }
    footer:DialogButtonBox {
        ThemeButton {theme:dialog.theme;text:"Export example…";enabled:!dialog.localMode && !!dialog.report.blocks;onClicked:save.open()}
        ThemeButton {theme:dialog.theme;text:"Close";onClicked:dialog.close()}
    }
}
