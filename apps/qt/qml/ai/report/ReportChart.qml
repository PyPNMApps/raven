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
    property var shown: block.viewport || ({start:0,total:0,count:0,points:[]})
    implicitHeight: 360
    Label {text:root.block.title; textFormat:Text.PlainText; color:root.theme.text; font.bold:true; Layout.fillWidth:true; wrapMode:Text.Wrap}
    MeasurementChart {
        theme:root.theme
        Layout.fillWidth:true; Layout.fillHeight:true
        viewport:root.shown
        axisLabel:root.block.axis_label || "RxMER"; unit:root.block.unit || "dB"
        minimum:root.block.minimum || 0; maximum:root.block.maximum || 64
        onRangeRequested: (start,count) => {
            const original=root.block.viewport
            root.shown={start:start,count:count,total:original.total,points:original.points.filter(function(point){return point.sample_index>=start && point.sample_index<start+count})}
        }
    }
    Label {text:"Bounded display samples · drag to zoom, double-click to reset"; color:root.theme.muted; font.pixelSize:11; Layout.fillWidth:true; wrapMode:Text.Wrap}
}
