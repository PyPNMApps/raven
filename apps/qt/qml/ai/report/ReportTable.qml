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
    readonly property var columns: block.columns || []
    readonly property var rows: block.rows || []
    Label {text:root.block.title + " · " + root.block.classification; textFormat:Text.PlainText; color:root.theme.text; font.bold:true; wrapMode:Text.Wrap; Layout.fillWidth:true}
    Label {text:"No results are available for this analysis yet.";visible:root.rows.length===0;color:root.theme.muted;Layout.fillWidth:true;wrapMode:Text.Wrap}
    Flickable {
        id: table
        objectName:"reportTableScroll"
        Layout.fillWidth:true; Layout.preferredHeight:Math.min(420, (root.rows.length+1)*52+16)
        contentWidth:Math.max(width,root.columns.length*190); contentHeight:rowsColumn.implicitHeight
        clip:true; boundsBehavior:Flickable.StopAtBounds
        ScrollBar.horizontal: ScrollBar {
            parent: table; orientation: Qt.Horizontal
            x: 0; y: table.height - height
            width: table.width; height: 12
            policy: ScrollBar.AsNeeded
        }
        ScrollBar.vertical: ScrollBar {
            parent: table; orientation: Qt.Vertical
            x: table.width - width; y: 0
            width: 12; height: table.height
            policy: ScrollBar.AsNeeded
        }
        Column {
            id:rowsColumn
            width:table.contentWidth
            Repeater {
                model:root.rows.length+1
                Row {
                    id:row
                    height:implicitHeight
                    required property int index
                    readonly property int rowIndex:index
                    Repeater {
                        model:root.columns.length
                        Item {
                            id:cell
                            required property int index
                            width:table.contentWidth/root.columns.length
                            height:Math.max(52,cellText.implicitHeight+8)
                            Rectangle {width:parent.width;height:row.implicitHeight;color:row.rowIndex===0 ? root.theme.button : "transparent";border.color:root.theme.line}
                            ThemeTextArea {
                                id:cellText
                                theme:root.theme; x:4;y:4;width:parent.width-8;height:implicitHeight
                                background:null; readOnly:true; wrapMode:TextEdit.Wrap
                                font.bold:row.rowIndex===0
                                text:row.rowIndex===0 ? root.columns[cell.index] : (root.rows[row.rowIndex-1][cell.index]===null ? "Unavailable" : root.rows[row.rowIndex-1][cell.index])
                            }
                        }
                    }
                }
            }
        }
    }
}
