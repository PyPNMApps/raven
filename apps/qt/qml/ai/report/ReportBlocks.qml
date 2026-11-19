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
    required property var blocks
    signal anchorRequested(string target)
    spacing:20
    function componentFor(type) {
        // Finite application-owned registry. Never resolve a model-supplied path.
        switch(type) {
        case "markdown":return Qt.resolvedUrl("ReportMarkdown.qml")
        case "chart":return Qt.resolvedUrl("ReportChart.qml")
        case "table":return Qt.resolvedUrl("ReportTable.qml")
        case "image":return Qt.resolvedUrl("ReportImage.qml")
        case "evidence":return Qt.resolvedUrl("ReportEvidence.qml")
        case "expandable":return Qt.resolvedUrl("ReportExpandable.qml")
        default:return Qt.resolvedUrl("ReportUnavailable.qml")
        }
    }
    function itemFor(target) {
        for(let index=0;index<items.count;index++) {
            const loader=items.itemAt(index)
            if(loader.block.id===target) return loader.item
            if(loader.item && loader.item.findItem) {const found=loader.item.findItem(target);if(found)return found}
        }
        return null
    }
    Repeater {
        id:items
        model:root.blocks
        Loader {
            required property var modelData
            property var block:modelData
            objectName:"reportLoader-"+block.id
            Layout.fillWidth:true
            Component.onCompleted:setSource(root.componentFor(block.type),{theme:root.theme,block:block})
            onLoaded: {
                if(item.anchorRequested)item.anchorRequested.connect(root.anchorRequested)
            }
        }
    }
}
