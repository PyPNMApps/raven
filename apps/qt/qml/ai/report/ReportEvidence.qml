/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../.."
ColumnLayout {
    id:root
    required property Theme theme
    required property var block
    Label {text:root.block.label; textFormat:Text.PlainText; color:root.theme.text; font.bold:true; wrapMode:Text.Wrap; Layout.fillWidth:true}
    Repeater {
        model:root.block.values || []
        ThemeTextArea {
            required property var modelData
            theme:root.theme; readOnly:true; wrapMode:TextEdit.Wrap; Layout.fillWidth:true
            text:modelData.label + ": " + (modelData.value===null ? "Unavailable" : modelData.value + " " + modelData.unit) + "\n" + modelData.classification + " · " + modelData.scope
        }
    }
    ThemeButton {theme:root.theme; text:"Open source measurement"; visible:!!root.block.action; enabled:false}
    Label {text:root.block.action ? root.block.action.reason : ""; visible:text.length>0; color:root.theme.muted; wrapMode:Text.Wrap; Layout.fillWidth:true}
}
