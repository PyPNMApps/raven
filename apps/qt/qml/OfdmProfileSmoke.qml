/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick

// Explicit developer smoke: --ofdm-smoke PROFILE_URL RXMER_URL CHANNEL_URL_OR_NONE
// OUTPUT_PREFIX [--mce]. Original local files only; no network or installation.
Timer {
    id: smoke
    required property var service
    required property var workspace
    required property var applicationWindow
    required property Item surface
    required property var theme
    readonly property int argumentIndex: Qt.application.arguments.indexOf("--ofdm-smoke")
    property int stage: 0
    property int ticks: 0
    readonly property int testProfileCount: {
        const selection=Qt.application.arguments.find(function(value){return value.indexOf("--ofdm-profiles=")===0})
        return selection ? selection.split("=")[1].split(",").length : 1
    }
    readonly property int timeoutTicks: 2300*Math.max(1,Math.min(16,testProfileCount))
    interval: 150; repeat: true; running: argumentIndex >= 0
    function fail(message) { console.error("OFDM_SMOKE_FAILED: " + message); stop(); Qt.exit(1) }
    function snapshot(suffix, next) {
        stop()
        const target = suffix.indexOf("-fec-") >= 0 ? workspace.resultsSurface : surface
        if (!target.grabToImage(function(image) {
            if (!image.saveToFile(Qt.application.arguments[argumentIndex+4] + suffix)) {
                smoke.fail("Could not save native workspace frame")
                return
            }
            stage=next
            start()
        })) fail("Could not request native workspace frame")
    }
    onTriggered: {
        ticks++
        if (ticks > timeoutTicks || service.error_message.length) { fail(service.error_message || "Timeout"); return }
        const args=Qt.application.arguments
        if (stage===0) {
            if (args.length < argumentIndex+5) { fail("Missing smoke arguments"); return }
            applicationWindow.selectWorkspace("ofdm")
            theme.preference=1
            service.choose_file("profile",args[argumentIndex+1]);stage=1
        } else if (stage===1 && !service.busy) {
            service.choose_file("rxmer",args[argumentIndex+2]);stage=2
        } else if (stage===2 && !service.busy) {
            if (args[argumentIndex+3]!=="none") service.choose_file("channel",args[argumentIndex+3])
            stage=3
        } else if (stage===3 && !service.busy) {
            const selection=args.find(function(value){return value.indexOf("--ofdm-profiles=")===0})
            const profiles=selection ? selection.split("=")[1].split(",").map(Number) : [0]
            service.prepare_many(JSON.stringify(profiles),args.indexOf("--mce")>=0,86400);stage=4
        } else if (stage===4 && !service.busy && service.prepared) {
            const chart=JSON.parse(service.chart_json)
            if (!chart.viewport.points.length || !chart.profiles.length || !chart.profiles[0].points.length) { fail("No overlay");return }
            snapshot("-dark.png",5)
        } else if (stage===5) {
            theme.preference=2;stage=6
        } else if (stage===6) {
            snapshot("-light.png",7)
        } else if (stage===7) {
            if (args.indexOf("--ofdm-view-only")>=0) {
                console.log("OFDM_VIEW_OK")
                stop();Qt.quit();return
            }
            theme.preference=1
            workspace.openResults();service.run();stage=8
        } else if (stage===8 && !service.busy) {
            const report=JSON.parse(service.result_json)
            if (report.status!=="Complete" || !report.entries || report.entries.some(function(entry){return !entry.profile || !(entry.profile.compared_bits>0)})) {
                fail("Incomplete decoder results: "+JSON.stringify(report));return
            }
            snapshot("-fec-dark.png",9)
        } else if (stage===9) {
            theme.preference=2;stage=10
        } else if (stage===10) {
            snapshot("-fec-light.png",11)
        } else if (stage===11) {
            console.log("OFDM_SMOKE_OK "+service.result_json)
            stop();Qt.quit()
        }
    }
}
