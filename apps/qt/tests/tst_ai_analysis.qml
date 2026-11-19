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
        property string state_json: JSON.stringify({analysis_prompts:[{id:"overview",label:"Profile performance overview",question:"Review each selected profile separately."},{id:"fec",label:"FEC and decoder effort",question:"Explain the FEC and LDPC results."}]})
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
            preview_json=JSON.stringify({destination:"https://api.openai.com/v1/chat/completions",model:"test-model",body:"exact synthetic request",review_text:"QUESTION\n"+question+"\n\nPROFILE 3\nCorrected codewords: 2",digest:"test-digest"})
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
        verify(!findChild(dialog,"aiDemoMode").visible)
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
        verify(!toggle.visible)
        service.choose_demo_mode(true)
        verify(toggle.visible)
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
    function test_prepared_question_selection_invalidates_without_sending() {
        dialog.open()
        const choice=findChild(dialog,"aiPreparedPrompt")
        compare(choice.count,2)
        service.prepare_analysis("","Question")
        choice.currentIndex=1;choice.activated(1)
        compare(findChild(dialog,"aiQuestion").text,"Explain the FEC and LDPC results.")
        verify(!service.request_ready)
        service.prepare_analysis("", "Explain the FEC and LDPC results.")
        choice.activated(1)
        verify(!service.request_ready)
        compare(service.submissions,0)
    }
    function test_review_is_readable_and_long_content_scrolls_in_both_themes() {
        for (let preference of [1,2]) {
            appearance.preference=preference
            dialog.open()
            service.preview_json=JSON.stringify({review_text:"PROFILE 3\nCorrected codewords: 2\n".repeat(120),body:"exact synthetic request",destination:"https://example.invalid",model:"test",digest:"test"})
            const review=findChild(dialog,"aiRequestReview")
            const scroll=findChild(dialog,"aiReviewScroll")
            verify(waitForFrame(review))
            verify(review.text.indexOf("Corrected codewords: 2")>=0)
            verify(review.text.indexOf("exact synthetic request")<0)
            compare(review.textFormat,TextEdit.PlainText)
            verify(scroll.contentHeight>scroll.availableHeight)
            compare(scroll.contentWidth,scroll.availableWidth)
            const flickable=scroll.contentItem
            flickable.contentY=150
            tryCompare(flickable,"contentY",150)
            compare(service.submissions,0)
            dialog.close()
        }
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
