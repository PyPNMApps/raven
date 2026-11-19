/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick
import QtQuick.Controls
import "../.."
Label {
    required property Theme theme
    required property var block
    text:(block.title || "Report block unavailable") + "\n" + (block.reason || "Unsupported renderer block.")
    textFormat:Text.PlainText; wrapMode:Text.Wrap; color:theme.muted
}
