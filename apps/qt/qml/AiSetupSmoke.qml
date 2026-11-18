/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick

// Offline developer check: --ai-setup-smoke OUTPUT_PREFIX. Synthetic keys only.
Timer {
    id: smoke
    required property var service
    required property var settingsDialog
    required property var assistanceChip
    required property var systemDialog
    required property var systemAiAction
    required property var theme
    required property Item surface
    readonly property int argumentIndex: Qt.application.arguments.indexOf("--ai-setup-smoke")
    property int stage: 0
    property int ticks: 0
    interval: 150
    repeat: true
    running: argumentIndex >= 0
    function fail(message) {
        stop()
        console.error("AI_SETUP_FAILED: " + message)
        Qt.exit(1)
    }
    function snapshot(suffix,next) {
        stop()
        if (!surface.grabToImage(function(image) {
            if (!image.saveToFile(Qt.application.arguments[argumentIndex+1]+suffix)) {
                smoke.fail("Could not save setup frame")
                return
            }
            stage=next
            start()
        })) fail("Could not capture setup frame")
    }
    onTriggered: {
        if (++ticks>150) { fail("Timeout"); return }
        if (stage===0) {
            if (Qt.application.arguments.length!==argumentIndex+2) { fail("Expected screenshot output prefix"); return }
            const state=JSON.parse(service.state_json)
            if (state.enabled || state.selected || state.analysis_available || state.providers.length!==5 || !assistanceChip.enabled) {
                fail("Setup must start offline and unconfigured");return
            }
            theme.preference=1
            assistanceChip.clicked()
            stage=1
        } else if (stage===1 && settingsDialog.visible) {
            snapshot("-dark.png",2)
        } else if (stage===2) {
            theme.preference=2
            stage=3
        } else if (stage===3) {
            snapshot("-light.png",4)
        } else if (stage===4) {
            settingsDialog.close()
            systemDialog.open()
            stage=5
        } else if (stage===5 && systemDialog.visible) {
            systemAiAction.clicked()
            stage=6
        } else if (stage===6 && settingsDialog.visible && !systemDialog.visible) {
            if (!service.configure("openai","offline-test-model","synthetic-ai-setup-sentinel")) {
                fail("Offline configuration rejected");return
            }
            const state=JSON.parse(service.state_json)
            if (!state.enabled || state.selected!=="openai" || state.verified || !state.analysis_available
                || service.state_json.indexOf("sentinel")>=0) {
                fail("Setup leaked a key or claimed unsupported verification");return
            }
            if (!service.set_enabled(false) || !service.forget("openai") || JSON.parse(service.state_json).selected) {
                fail("Disable/forget failed");return
            }
            settingsDialog.close()
            stop()
            console.log("AI_SETUP_OK")
            Qt.quit()
        }
    }
}
