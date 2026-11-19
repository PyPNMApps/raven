/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick
import QtQuick.Layouts
import "../.."
ColumnLayout {
    id:root
    required property Theme theme
    required property var block
    signal anchorRequested(string target)
    ThemeTextArea {
        objectName:"reportMarkdownText"
        theme:root.theme
        Layout.fillWidth:true
        readOnly:true; textFormat:TextEdit.MarkdownText
        text:root.block.display_content===undefined ? (root.block.content || "") : root.block.display_content
        wrapMode:TextEdit.Wrap;selectByKeyboard:true;font.pixelSize:14;padding:0
        onLinkActivated:(link)=>{if((root.block.links || []).indexOf(link)>=0)root.anchorRequested(link.substring(1))}
    }
    Flow {
        Layout.fillWidth:true;spacing:6
        Repeater {
            model:root.block.navigation || []
            ThemeButton {
                required property var modelData
                theme:root.theme;text:modelData.label
                onClicked:root.anchorRequested(modelData.target.substring(1))
            }
        }
    }
}
