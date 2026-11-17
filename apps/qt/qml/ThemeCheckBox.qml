/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick
import QtQuick.Controls as Controls

Controls.CheckBox {
    id: control
    required property Theme theme
    property color textColor: theme.text
    property ThemePalette themePalette: ThemePalette { targetControl: control; theme: control.theme }
    padding: 4
    spacing: 8
    indicator: Rectangle {
        implicitWidth: 18
        implicitHeight: 18
        x: control.leftPadding
        y: control.topPadding + (control.availableHeight-height)/2
        radius: 3
        color: control.checked ? control.theme.accentSoft : control.theme.button
        border.width: control.checked || control.visualFocus ? 2 : 1
        border.color: control.checked || control.visualFocus ? control.theme.accent : control.theme.muted
        Text {
            anchors.centerIn: parent
            text: control.checked ? "✓" : ""
            color: control.enabled ? control.theme.accent : control.theme.muted
            font.pixelSize: 14
            font.bold: true
        }
    }
    contentItem: Text {
        text: control.text
        font: control.font
        color: control.enabled ? control.textColor : control.theme.muted
        leftPadding: control.indicator.width + control.spacing
        verticalAlignment: Text.AlignVCenter
    }
}
