/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick
import QtQuick.Controls
import QtTest
import "../qml"
import "helpers"

RenderTestCase {
    id: testCase
    name: "AiAnalysis"
    width: 1100; height: 820; visible: true; when: windowShown
    Theme { id: appearance }
    QtObject {
        id: service
        property string preview_json: "{}"
        property string report_json: "{}"
        property string conversation_json: "[]"
        property string findings_text: "Local immutable findings"
        property string error_message: ""
        property string operation_status: ""
        property bool demo_mode: false
        property bool demo_locked: false
        property bool busy: false
        property bool request_ready: false
        property int submissions: 0
        property int previews: 0
        property string approved: ""
        function choose_demo_mode(enabled) {demo_mode=enabled;invalidate()}
        function invalidate() { request_ready=false;preview_json="{}" }
        function prepare_analysis(evidence,question) {
            previews++
            preview_json=JSON.stringify({destination:"https://api.openai.com/v1/chat/completions",model:"test-model",body:"exact synthetic request",digest:"test-digest"})
            request_ready=true
        }
        function analyze(digest) {submissions++;approved=digest;request_ready=false;busy=true}
        function cancel() {busy=false;invalidate()}
        function export_conversation(url) {}
    }
    QtObject {id:ofdm;property string evidence_json:'{"run_id":"synthetic-run","findings":[{"profile_id":3,"text":"Local observation"}]}' }
    AiAnalysisDialog {id:dialog;theme:appearance;service:service;ofdmService:ofdm;parent:testCase}
    function init() {
        service.demo_mode=false;service.demo_locked=false;service.busy=false;service.error_message="";service.report_json="{}";service.conversation_json="[]"
        service.submissions=0;service.previews=0;service.invalidate()
        appearance.preference=1
    }
    function cleanup() {dialog.close()}
    function test_no_automatic_submission_preview_then_explicit_approval() {
        dialog.open()
        compare(service.submissions,0)
        verify(!findChild(dialog,"aiAnalyzeApproved").enabled)
        const preview=findChild(dialog,"aiCreatePreview")
        verify(waitForFrame(preview));mouseClick(preview)
        compare(service.previews,1);compare(service.submissions,0)
        const analyze=findChild(dialog,"aiAnalyzeApproved")
        verify(waitForFrame(analyze));mouseClick(analyze)
        compare(service.submissions,1);compare(service.approved,"test-digest")
        verify(!analyze.enabled)
    }
    function test_demo_mode_clearly_labels_and_invalidates_live_preview() {
        dialog.open();service.prepare_analysis("","")
        const toggle=findChild(dialog,"aiDemoMode")
        verify(waitForFrame(toggle));mouseClick(toggle)
        verify(service.demo_mode);verify(!service.request_ready)
        compare(dialog.title,"OFDM Profile Performance · Demo AI")
        compare(findChild(dialog,"aiAnalyzeApproved").text,"Run approved demo")
        service.demo_locked=true
        verify(!toggle.enabled)
        compare(service.submissions,0)
    }
    function test_question_edit_and_popup_close_invalidate_approval() {
        dialog.open();service.prepare_analysis("","")
        findChild(dialog,"aiQuestion").text="Changed question"
        verify(!service.request_ready)
        service.prepare_analysis("","");dialog.close()
        verify(!service.request_ready);compare(service.submissions,0)
    }
    function test_remote_markup_is_plain_text_and_report_uses_safe_controls() {
        appearance.preference=2
        dialog.open()
        service.report_json=JSON.stringify({returned_model:"test",usage:{},report:{summary:"<img src='file:///private'>",claims:[],limitations:[],suggested_checks:[]}})
        const report=findChild(dialog,"aiReportText")
        compare(report.textFormat,TextEdit.PlainText)
        verify(report.text.indexOf("<img")>=0)
        verify(waitForFrame(report))
        compare(service.submissions,0)
    }
}
