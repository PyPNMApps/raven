/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick
Timer {
    id:smoke
    required property var service
    required property var dialog
    required property var theme
    required property Item surface
    readonly property int argumentIndex:Qt.application.arguments.indexOf("--report-smoke")
    running:argumentIndex>=0;repeat:true;interval:250
    property int stage:0
    property int ticks:0
    function fail(message){console.error(message);stop();Qt.exit(1)}
    function snapshot(suffix,next){stop();surface.grabToImage(function(result){if(!result.saveToFile(Qt.application.arguments[argumentIndex+1]+suffix)){fail("Report screenshot failed");return}stage=next;start()})}
    onTriggered: {
        if(++ticks>180){fail("Report smoke timed out");return}
        if(stage===0){theme.preference=1;dialog.gallery("rich");stage=1}
        else if(stage===1 && ticks>3){
            if(service.error_message.length || JSON.parse(service.model_json).blocks.length!==6){fail("Rich report failed validation");return}
            snapshot("-dark.png",2)
        }
        else if(stage===2){theme.preference=2;stage=3}
        else if(stage===3){snapshot("-light.png",4)}
        else if(stage===4){theme.preference=1;dialog.scrollToEnd();stage=5}
        else if(stage===5){snapshot("-details-dark.png",6)}
        else if(stage===6){theme.preference=2;stage=7}
        else if(stage===7){snapshot("-details-light.png",8)}
        else if(stage===8){service.load_fixture("invalid");if(service.model_json!=="{}" || !service.error_message.length){fail("Invalid report retained content");return}service.load_fixture("engineering");stage=9}
        else if(stage===9){if(service.error_message.length){fail("Report replacement failed");return}dialog.close();if(service.model_json!=="{}"){fail("Closed report retained content");return}console.log("REPORTS_OK");stop();Qt.quit()}
    }
}
