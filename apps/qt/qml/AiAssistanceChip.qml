/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick
import QtQuick.Controls

ThemeButton {
    id: chip
    required property var configuration
    objectName: "aiAssistanceChip"
    property bool demoMode: false
    readonly property bool configured: !!configuration.enabled && !!configuration.selected
    text: "AI Assistance · " + (demoMode ? "Demo" : configured ? "Configured" : "Off")
    implicitHeight: 26
    topPadding: 3
    bottomPadding: 3
    Accessible.name: "AI Assistance settings"
    ToolTip.visible: hovered
    ToolTip.text: demoMode ? "Local demo responses; no provider connection" : configured ? "Session configured; connection not verified" : "Configure AI Assistance"
    contentItem: Text {
        text: chip.text
        font: chip.font
        color: (chip.configured || chip.demoMode) ? chip.theme.accent : chip.theme.muted
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
    }
    background: Rectangle {
        implicitWidth: 174
        radius: height / 2
        color: chip.hovered ? chip.theme.accentSoft : chip.theme.button
        border.color: chip.visualFocus || (chip.configured || chip.demoMode) ? chip.theme.accent : chip.theme.line
        border.width: chip.visualFocus ? 2 : 1
    }
}
