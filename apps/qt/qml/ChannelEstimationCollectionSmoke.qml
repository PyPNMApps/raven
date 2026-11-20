/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick
// Explicit offline developer check: --channel-estimation-smoke INPUT_URL OUTPUT_PREFIX.
Timer {
    id: smoke
    required property var service
    required property var workspace
    required property var applicationWindow
    required property var theme
    required property Item surface
    readonly property int argumentIndex: Qt.application.arguments.indexOf("--channel-estimation-smoke")
    property int stage: 0
    property int ticks: 0
    interval: 180
    repeat: true
    running: argumentIndex >= 0
    function fail(message) { console.error("CE_COLLECTION_FAILED: "+message); stop(); Qt.exit(1) }
    function snapshot(suffix,next) {
        stop()
        surface.Window.window.update()
        if (!surface.grabToImage(function(image){
            if(!image.saveToFile(Qt.application.arguments[argumentIndex+2]+suffix)) { fail("Screenshot failed"); return }
            stage=next; start()
        })) fail("Could not request frame")
    }
    onTriggered: {
        ticks++
        if(ticks>600 || service.error_message.length) { fail(service.error_message || "Timeout"); return }
        const args=Qt.application.arguments
        if(stage===0) {
            if(args.length!==argumentIndex+3) { fail("Expected input URL and output prefix"); return }
            applicationWindow.selectWorkspace("channel-estimation")
            applicationWindow.navigationVisible=true
            theme.preference=1
            service.import_collection(args[argumentIndex+1]); stage=1
        } else if(stage===1 && !service.busy) {
            const catalog=JSON.parse(service.catalog_json)
            if(!(catalog.groups || []).length) {fail("No groups"); return}
            workspace.selectGroup(catalog.groups[0].mac_address,catalog.groups[0].id); stage=2
        } else if(stage===2 && !service.busy) {
            const view=JSON.parse(service.view_json)
            if(!view.group || !view.points.length || !(view.capture_count>1)) {fail("No multi-capture plot"); return}
            const report=JSON.parse(service.engineering_report())
            if(report.error || !report.prepared_question) {fail("Report unavailable"); return}
            service.export_json("file://"+args[argumentIndex+2]+".json"); stage=3
        } else if(stage===3 && !service.busy) {
            snapshot("-magnitude-dark.png",4)
        } else if(stage===4) {
            theme.preference=2; stage=5
        } else if(stage===5) {
            snapshot("-magnitude-light.png",6)
        } else if(stage===6) {
            theme.preference=1; service.select_metric(true); stage=7
        } else if(stage===7) {
            snapshot("-delay-dark.png",8)
        } else if(stage===8) {
            theme.preference=2; stage=9
        } else if(stage===9) {
            snapshot("-delay-light.png",10)
        } else if(stage===10) {
            service.show_range(100,300)
            const view=JSON.parse(service.view_json)
            if(view.start!==100 || view.count!==300) {fail("Zoom did not use source indices");return}
            service.show_range(0,view.total)
            if(JSON.parse(service.view_json).count!==view.total) {fail("Reset failed");return}
            workspace.playbackMode=true
            stage=11
        } else if(stage===11) {
            let view=JSON.parse(service.view_json)
            if(view.frame_index!==0 || !(view.frame_count>1) || view.average_method!=="single_capture" || view.channel_estimation_statistics) {fail("Playback metadata invalid");return}
            service.show_range(100,300)
            workspace.showFrame(1)
            view=JSON.parse(service.view_json)
            if(view.frame_index!==1 || view.start!==100 || view.count!==300) {fail("Playback lost zoom");return}
            service.show_range(0,view.total)
            service.select_metric(false)
            theme.preference=1
            stage=12
        } else if(stage===12) {
            service.select_metric(false)
            let first=JSON.parse(service.view_json)
            if(first.magnitude_axis!=="locked_full_group_aggregate") {fail("Missing locked magnitude axis");return}
            workspace.showFrame(0)
            service.show_range(100,300)
            let next=JSON.parse(service.view_json)
            if(next.minimum!==first.minimum || next.maximum!==first.maximum) {fail("Magnitude axis changed during stepping/zoom");return}
            service.select_metric(true)
            service.select_metric(false)
            next=JSON.parse(service.view_json)
            if(next.minimum!==first.minimum || next.maximum!==first.maximum) {fail("Magnitude axis changed on metric switch");return}
            service.show_range(0,next.total)
            workspace.showFrame(1)
            snapshot("-playback-dark.png",13)
        } else if(stage===13) {
            theme.preference=2; stage=14
        } else if(stage===14) {
            snapshot("-playback-light.png",15)
        } else if(stage===15) {
            console.log("CE_COLLECTION_OK")
            stop(); Qt.quit()
        }
    }
}
