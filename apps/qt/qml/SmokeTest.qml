/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick
import QtQuick.Dialogs

// Explicit CI-only command: --smoke-test INPUT_FILE_URL NEW_OUTPUT_FILE_URL.
// Optional developer capture: --screenshots OUTPUT_PREFIX [--view INDEX] [--density].
// Exercise the real asynchronous backend, JSON export and rendered frames, then exit.
Timer {
    id: smoke
    required property var theme
    required property var iqChart
    required property var viewerBackend
    required property var applicationWindow
    required property Item surface
    required property var openCaptureDialog
    required property var captureSelectionDialogs
    required property var exportCaptureDialog
    readonly property int argumentIndex: Qt.application.arguments.indexOf("--smoke-test")
    readonly property bool screenshots: Qt.application.arguments[argumentIndex + 3] === "--screenshots"
    readonly property real viewIndex: Qt.application.arguments[argumentIndex + 5] === "--view"
        ? Number(Qt.application.arguments[argumentIndex + 6]) : 0
    readonly property bool densityCapture: Qt.application.arguments[Qt.application.arguments.length - 1] === "--density"
    property int stage: 0
    property int ticks: 0
    interval: 150
    repeat: true
    running: argumentIndex >= 0
    /**
     * Save a native frame for the requested theme and advance the capture state.
     * suffix selects the output filename; next is the state after a successful save.
     * A failed capture exits with failure rather than reusing an earlier image.
     */
    function snapshot(suffix, next) {
        stop()
        if (!surface.grabToImage(function(image) {
            if (!image.saveToFile(Qt.application.arguments[argumentIndex + 4] + suffix)) {
                console.error("UI_SMOKE_FAILED: screenshot save failed")
                Qt.exit(1)
                return
            }
            stage = next
            start()
        })) {
            console.error("UI_SMOKE_FAILED: frame request rejected")
            Qt.exit(1)
        }
    }
    onTriggered: {
        ticks += 1
        if (ticks > 260 || viewerBackend.error_message.length > 0) {
            console.error("UI_SMOKE_FAILED: " + viewerBackend.error_message)
            stop()
            Qt.exit(1)
            return
        }
        if (stage === 0) {
            const examples = viewerBackend.examples_folder().toString()
            for (const captureDialog of captureSelectionDialogs) {
                const currentFolder = captureDialog.currentFolder.toString()
                if (examples.length > 0 && currentFolder !== examples &&
                    currentFolder !== examples + "/" && currentFolder + "/" !== examples) {
                    console.error("UI_SMOKE_FAILED: capture selection must start in files/demo: " + captureDialog.title)
                    stop()
                    Qt.exit(1)
                    return
                }
            }
            if (Qt.platform.os === "windows" &&
                ((openCaptureDialog.options & FileDialog.DontUseNativeDialog) !== 0 ||
                 (exportCaptureDialog.options & FileDialog.DontUseNativeDialog) !== 0)) {
                console.error("UI_SMOKE_FAILED: Windows native file dialogs disabled")
                stop()
                Qt.exit(1)
                return
            }
            if (Qt.platform.os === "linux" && Qt.application.name !== "raven") {
                console.error("UI_SMOKE_FAILED: desktop application identity mismatch")
                stop()
                Qt.exit(1)
                return
            }
            const applicationArguments = Qt.application.arguments
            const plainSmoke = applicationArguments.length === argumentIndex + 3
            const captureLength = applicationArguments.length - (densityCapture ? 1 : 0)
            const captureSmoke = screenshots && (captureLength === argumentIndex + 5 ||
                (captureLength === argumentIndex + 7 && applicationArguments[argumentIndex + 5] === "--view")) &&
                Number.isInteger(viewIndex) && viewIndex >= 0 && (!densityCapture || viewIndex === 2)
            if (!plainSmoke && !captureSmoke) {
                console.error("UI_SMOKE_FAILED: expected input and new output file URLs")
                stop()
                Qt.exit(2)
                return
            }
            if (screenshots) {
                theme.preference = 1
                applicationWindow.navigationVisible = false
                iqChart.densityMode = densityCapture
            }
            viewerBackend.open_file(Qt.application.arguments[argumentIndex + 1])
            stage = 1
        } else if (stage === 1 && viewerBackend.loaded && !viewerBackend.busy) {
            const viewport = JSON.parse(viewerBackend.viewport_json)
            if (Object.keys(viewport).length === 0 || !applicationWindow.visible) {
                console.error("UI_SMOKE_FAILED: no visible capture")
                stop()
                Qt.exit(1)
                return
            }
            if (screenshots && viewIndex > 0) {
                const metadata = JSON.parse(viewerBackend.metadata_json)
                if (viewIndex >= (metadata.views || []).length) {
                    console.error("UI_SMOKE_FAILED: unavailable view")
                    stop()
                    Qt.exit(2)
                    return
                }
                viewerBackend.select_view(viewIndex)
                stage = 6
            } else {
                viewerBackend.export_json(Qt.application.arguments[argumentIndex + 2])
                stage = 2
            }
        } else if (stage === 6 && !viewerBackend.busy) {
            viewerBackend.export_json(Qt.application.arguments[argumentIndex + 2])
            stage = 2
        } else if (stage === 2 && !viewerBackend.busy && viewerBackend.status === "Full-resolution JSON exported") {
            if (screenshots) {
                snapshot("-dark.png", 10)
                return
            }
            stage = 3
            stop()
            const queued = surface.grabToImage(function(result) {
                if (result.url.toString().length === 0) {
                    console.error("UI_SMOKE_FAILED: rendering failed")
                    Qt.exit(1)
                    return
                }
                console.log("UI_SMOKE_OK")
                Qt.quit()
            })
            if (!queued) {
                console.error("UI_SMOKE_FAILED: frame request rejected")
                Qt.exit(1)
            }
        } else if (stage === 10) {
            theme.preference = 2
            stage = 11
        } else if (stage === 11) {
            snapshot("-light.png", 12)
        } else if (stage === 12) {
            console.log("UI_SCREENSHOTS_OK")
            stop()
            Qt.quit()
        }
    }
}
