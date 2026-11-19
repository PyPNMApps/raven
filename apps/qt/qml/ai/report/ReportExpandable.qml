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
    property bool expanded:false
    signal anchorRequested(string target)
    function findItem(target) { expanded=true; return childrenBlocks.itemFor(target) }
    ThemeButton {
        objectName:"reportExpandButton"
        theme:root.theme; text:(root.expanded ? "▾ " : "▸ ") + root.block.title
        onClicked:root.expanded=!root.expanded
    }
    ReportBlocks {
        id:childrenBlocks
        theme:root.theme; blocks:root.block.blocks || []
        visible:root.expanded; Layout.fillWidth:true
        onAnchorRequested:(target)=>root.anchorRequested(target)
    }
}
