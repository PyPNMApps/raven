/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick
import QtQuick.Controls

ScrollView {
    id: pane
    required property Theme theme
    property alias text: textArea.text
    property string textObjectName: ""
    clip: true
    contentWidth: availableWidth
    contentHeight: textArea.implicitHeight
    ScrollBar.vertical.policy: ScrollBar.AlwaysOn
    ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
    ThemeTextArea {
        id: textArea
        objectName: pane.textObjectName
        theme: pane.theme
        width: pane.availableWidth
        readOnly: true
        wrapMode: TextEdit.Wrap
        padding: 12
        font.pixelSize: 14
    }
}
