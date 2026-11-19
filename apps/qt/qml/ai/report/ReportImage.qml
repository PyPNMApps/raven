/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../.."
ColumnLayout {
    id: root
    required property Theme theme
    required property var block
    readonly property string resolved:theme.dark ? block.source_dark : block.source_light
    Image {
        id:illustration
        objectName:"reportApprovedImage"
        Layout.fillWidth:true; Layout.preferredHeight:180
        source: ["qrc:/branding/icon-dark.png","qrc:/branding/icon-light.png","qrc:/reports/example.jpg","qrc:/reports/example.webp"].indexOf(root.resolved)>=0 ? root.resolved : ""
        fillMode:Image.PreserveAspectFit
        sourceSize.width:256
        Accessible.name:root.block.caption
    }
    Label {text:"This image format is unavailable in the current Qt runtime.";visible:illustration.status===Image.Error;color:root.theme.muted;Layout.fillWidth:true;wrapMode:Text.Wrap}
    Label {text:root.block.caption; textFormat:Text.PlainText; color:root.theme.muted; wrapMode:Text.Wrap; Layout.fillWidth:true}
}
