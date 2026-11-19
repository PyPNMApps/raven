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
    property var report: ({})
    property string errorMessage:""
    objectName:"dynamicEngineeringReport"
    function scrollToEnd(){scroll.contentItem.contentY=Math.max(0,scroll.contentHeight-scroll.availableHeight)}
    onReportChanged: {if(scroll.contentItem)scroll.contentItem.contentY=0}
    Label {text:root.errorMessage || root.report.title || "No engineering report loaded"; textFormat:Text.PlainText; color:root.theme.text; font.pixelSize:20; font.bold:true; wrapMode:Text.Wrap; Layout.fillWidth:true}
    Label {text:root.report.classification ? root.report.classification + (root.report.interpretation ? " · " + root.report.interpretation : "") : ""; textFormat:Text.PlainText; color:root.theme.muted; visible:text.length>0; Layout.fillWidth:true; wrapMode:Text.Wrap}
    ScrollView {
        id:scroll
        objectName:"engineeringReportScroll"
        Layout.fillWidth:true; Layout.fillHeight:true
        contentWidth:availableWidth; contentHeight:blocks.implicitHeight
        clip:true
        ScrollBar.vertical: ScrollBar {
            objectName: "reportVerticalScroll"
            parent: scroll
            orientation: Qt.Vertical
            x: scroll.width - width
            y: 0
            width: 12
            height: scroll.height
            size: Math.min(1, scroll.availableHeight / Math.max(1, scroll.contentHeight))
            position: scroll.contentItem ? scroll.contentItem.contentY / Math.max(1, scroll.contentHeight) : 0
            onPositionChanged: {
                if (pressed && scroll.contentItem) {
                    scroll.contentItem.contentY = position * scroll.contentHeight
                }
            }
            policy: ScrollBar.AlwaysOn
        }
        ScrollBar.horizontal: ScrollBar {policy:ScrollBar.AlwaysOff}
        ReportBlocks {
            id:blocks
            width:scroll.availableWidth
            theme:root.theme; blocks:root.report.blocks || []
            onAnchorRequested:(target)=> {
                const item=blocks.itemFor(target)
                const owner=root.report
                if(item) Qt.callLater(function(){if(root.report!==owner)return;scroll.contentItem.contentY=Math.max(0,Math.min(scroll.contentHeight-scroll.availableHeight,item.mapToItem(blocks,0,0).y))})
            }
        }
    }
}
