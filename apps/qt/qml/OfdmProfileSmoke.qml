/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick

// Explicit developer smoke: --ofdm-smoke PROFILE_URL RXMER_URL CHANNEL_URL_OR_NONE
// OUTPUT_PREFIX [--mce] [--home-only]. Home-only skips capture loading and FEC.
// Original local files only; no network or installation.
Timer {
    id: smoke
    required property var service
    required property var aiService
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
        const homeOnly = Qt.application.arguments.indexOf("--home-only") >= 0
        const target = homeOnly ? applicationWindow.contentItem.parent
            : Qt.application.arguments.indexOf("--ofdm-input-only") >= 0 ? applicationWindow.contentItem.parent
            : suffix.indexOf("-ai-") >= 0 ? applicationWindow.contentItem.parent
            : suffix.indexOf("-fec-") >= 0 ? workspace.resultsSurface : surface
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
            if (args.indexOf("--home-only") >= 0) {
                applicationWindow.selectWorkspace("single")
                applicationWindow.navigationVisible=true
                theme.preference=1
                stage=20
                return
            }
            applicationWindow.selectWorkspace("ofdm")
            theme.preference=1
            const files=[args[argumentIndex+1],args[argumentIndex+2]]
            if (args[argumentIndex+3]!=="none") files.push(args[argumentIndex+3])
            service.choose_files(JSON.stringify(files),86400)
            stage=3
        } else if (stage===3 && !service.busy) {
            const selection=args.find(function(value){return value.indexOf("--ofdm-profiles=")===0})
            const profiles=selection ? selection.split("=")[1].split(",").map(Number) : [0]
            service.prepare_selection(JSON.stringify(profiles),args.indexOf("--mce")>=0,args.indexOf("--ignore-channel")>=0,86400);stage=4
        } else if (stage===4 && !service.busy && service.prepared) {
            const chart=JSON.parse(service.chart_json)
            if (args.indexOf("--ignore-channel")>=0 && (chart.supplied_channel || chart.mce || !JSON.parse(service.inputs_json).channel)) {
                fail("Ignored channel was detached or used");return
            }
            if (!chart.viewport.points.length || !chart.profiles.length || !chart.profiles[0].points.length) { fail("No overlay");return }
            if (args.indexOf("--ofdm-input-only")>=0) workspace.openInputs()
            stage=30
        } else if (stage===30) {
            snapshot("-dark.png",5)
        } else if (stage===5) {
            theme.preference=2;stage=6
        } else if (stage===6) {
            snapshot("-light.png",7)
        } else if (stage===7) {
            if (args.indexOf("--ofdm-input-only")>=0) {
                console.log("OFDM_INPUTS_OK")
                stop();Qt.quit();return
            }
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
            if (args.indexOf("--ai-preview-only")>=0 || args.indexOf("--ai-demo-only")>=0) {
                const evidence=JSON.parse(service.evidence_json)
                if (!evidence.run_id || evidence.metrics.length<13) {fail("Native evidence missing");return}
                workspace.closeDialogs()
                if (args.indexOf("--ai-demo-only")>=0) {
                    aiService.choose_demo_mode(true)
                    if (JSON.parse(aiService.state_json).enabled) {fail("Demo must not need credentials");return}
                } else if (!aiService.configure("openai","offline-test-model","synthetic-ai-flow-sentinel")) {fail("Offline setup rejected");return}
                workspace.openAiAnalysis()
                workspace.previewAiQuestion()
                if (!aiService.request_ready || aiService.preview_json.indexOf("sentinel")>=0) {fail("Native preview failed: "+aiService.error_message);return}
                if (args.indexOf("--ai-demo-only")>=0) {
                    aiService.analyze(JSON.parse(aiService.preview_json).digest)
                    stage=50
                } else stage=40
                return
            }
            snapshot("-fec-dark.png",9)
        } else if (stage===50 && !aiService.busy) {
            const demoReport=JSON.parse(aiService.report_json)
            if(!demoReport.engineering_report || !demoReport.engineering_report.blocks.length){fail("Native rich AI adapter missing");return}
            if (!demoReport.demo || demoReport.provider!=="demo" || demoReport.usage.total_tokens!==null || JSON.parse(aiService.conversation_json).length!==1) {fail("Native demo failed: "+aiService.error_message);return}
            snapshot("-ai-demo-dark.png",51)
        } else if (stage===51) {
            theme.preference=2;stage=52
        } else if (stage===52) {
            snapshot("-ai-demo-light.png",53)
        } else if (stage===53) {
            aiService.choose_demo_mode(false)
            if (!aiService.demo_mode || !aiService.demo_locked) {fail("--ai-demo must lock out live mode");return}
            workspace.closeDialogs()
            console.log("AI_DEMO_OK")
            stop();Qt.quit()
        } else if (stage===40) {
            snapshot("-ai-dark.png",41)
        } else if (stage===41) {
            theme.preference=2;stage=42
        } else if (stage===42) {
            snapshot("-ai-light.png",43)
        } else if (stage===43) {
            workspace.closeDialogs()
            if (aiService.request_ready) {fail("Close retained approval");return}
            aiService.forget("openai")
            console.log("AI_PREVIEW_OK")
            stop();Qt.quit()
        } else if (stage===9) {
            theme.preference=2;stage=10
        } else if (stage===10) {
            snapshot("-fec-light.png",11)
        } else if (stage===20) {
            snapshot("-dark.png",21)
        } else if (stage===21) {
            theme.preference=2;stage=22
        } else if (stage===22) {
            snapshot("-light.png",23)
        } else if (stage===23) {
            console.log("HOME_SCREENSHOTS_OK")
            stop();Qt.quit()
        } else if (stage===11) {
            console.log("OFDM_SMOKE_OK "+service.result_json)
            stop();Qt.quit()
        }
    }
}
