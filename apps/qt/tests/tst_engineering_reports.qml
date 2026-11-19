/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick
import QtQuick.Controls
import QtTest
import "../qml"
import "../qml/ai/report"
import "helpers"
RenderTestCase {
    id:testCase
    name:"EngineeringReports"
    width:1000;height:760;visible:true;when:windowShown
    Theme {id:appearance}
    AiReportView {id:view;theme:appearance;anchors.fill:parent}
    function report(blocks) {return {title:"Offline engineering report",classification:"demonstration",context_id:"test",generation:"1",blocks:blocks}}
    function init(){appearance.preference=1;view.errorMessage="";view.report=({})}
    function cleanup(){view.report=({})}
    function loader(id) {return findChild(view,"reportLoader-"+id)}
    function test_markdown_native_selection_internal_links_and_theme() {
        for(let preference of [0,1,2]) {
            appearance.preference=preference
            view.report=report([{type:"markdown",id:"intro",content:"# Heading\n**Bold**, *italic*, `code`\n\n- Item\n\n> Quote\n\n[Details](#details)",links:["#details"]},{type:"expandable",id:"details",title:"Details",blocks:[{type:"markdown",id:"nested",content:"Nested evidence",links:[]}]}])
            tryVerify(function(){return !!loader("intro") && !!loader("intro").item})
            const text=findChild(loader("intro").item,"reportMarkdownText")
            compare(text.textFormat,TextEdit.MarkdownText)
            verify(text.readOnly);verify(text.selectByMouse)
            verify(waitForFrame(text))
            text.selectAll();verify(text.selectedText.length>0)
            text.linkActivated("https://not-permitted.invalid")
            text.linkActivated("#details")
            compare(text.color.toString(),appearance.text.toString())
        }
    }
    function test_vertical_scrollbar_geometry_and_long_document() {
        const blocks=[]
        for(let index=0;index<16;index++) blocks.push({id:"section-"+index,type:"markdown",content:"## Engineering section\nSeveral lines of source evidence.\n\nAdditional observations.",links:[]})
        view.report=report(blocks)
        const scroll=findChild(view,"engineeringReportScroll")
        const bar=findChild(view,"reportVerticalScroll")
        tryVerify(function(){return scroll.contentHeight>scroll.availableHeight})
        verify(waitForFrame(scroll))
        compare(bar.orientation,Qt.Vertical)
        verify(bar.height>200 && bar.width>=10)
        verify(bar.x>scroll.width-20)
        scroll.contentItem.contentY=180
        tryCompare(scroll.contentItem,"contentY",180)
        tryVerify(function(){return bar.position>0})
        const before=scroll.contentItem.contentY
        mousePress(bar,bar.width/2,bar.position*bar.height+8)
        mouseMove(bar,bar.width/2,bar.position*bar.height+80,50)
        mouseRelease(bar,bar.width/2,bar.position*bar.height+80)
        tryVerify(function(){return scroll.contentItem.contentY>before})
    }
    function test_table_missing_values_and_horizontal_scrolling() {
        view.report=report([{id:"table",type:"table",title:"Engineering table",classification:"demonstration",columns:["Profile","MER (dB)","Decoder iterations","Bits","Scope","Additional evidence"],rows:[["A","35.0",null,"8192","Observation","Local"]]}])
        tryVerify(function(){return !!findChild(view,"reportTableScroll")})
        const scroll=findChild(view,"reportTableScroll")
        verify(waitForFrame(scroll))
        verify(scroll.contentWidth>scroll.width)
        scroll.contentX=120;tryCompare(scroll,"contentX",120)
    }
    function test_finite_registry_never_loads_component_paths() {
        view.report=report([{id:"bad",type:"file:///tmp/never-load.qml",reason:"Rejected type"}])
        tryVerify(function(){return !!loader("bad") && !!loader("bad").item})
        verify(loader("bad").source.toString().endsWith("ReportUnavailable.qml"))
        compare(loader("bad").item.textFormat,Text.PlainText)
    }
    function test_replacement_disposes_old_controls_and_collapse_state() {
        view.report=report([{id:"section",type:"expandable",title:"Technical details",blocks:[{id:"old",type:"markdown",content:"Old generation",links:[]}]}])
        tryVerify(function(){return !!loader("section") && !!loader("section").item})
        loader("section").item.expanded=true
        view.report=report([{id:"new",type:"markdown",content:"Replacement generation",links:[]}])
        tryVerify(function(){return loader("section")===null && !!loader("new") && !!loader("new").item})
        view.report=report([{id:"section",type:"expandable",title:"Technical details",blocks:[]}])
        tryVerify(function(){return !!loader("section") && !!loader("section").item})
        verify(!loader("section").item.expanded)
        view.report=({})
        tryVerify(function(){return loader("section")===null})
    }
    function test_queued_anchor_cannot_target_a_replaced_report() {
        view.report=report([{id:"intro",type:"markdown",content:"[Details](#details)",links:["#details"]},{id:"details",type:"markdown",content:"Old details",links:[]}])
        tryVerify(function(){return !!loader("intro") && !!loader("intro").item})
        findChild(loader("intro").item,"reportMarkdownText").linkActivated("#details")
        view.report=report([{id:"new",type:"markdown",content:"New report",links:[]}])
        tryVerify(function(){return !!loader("new") && !!loader("new").item && loader("details")===null})
        verify(waitForFrame(loader("new").item))
    }
    function test_image_rejects_arbitrary_urls_even_at_presentation_boundary() {
        view.report=report([{id:"image",type:"image",source_dark:"https://never-fetch.invalid/image.png",source_light:"file:///private/image.png",caption:"Unavailable image"}])
        tryVerify(function(){return !!findChild(view,"reportApprovedImage")})
        const image=findChild(view,"reportApprovedImage")
        compare(image.source.toString(),"")
        appearance.preference=2;compare(image.source.toString(),"")
    }
    function test_section_titles_are_literal_text_not_html() {
        view.report=report([{id:"section",type:"expandable",title:"<img src='https://never-fetch.invalid'>",blocks:[]}])
        tryVerify(function(){return !!findChild(view,"reportExpandButton")})
        const button=findChild(view,"reportExpandButton")
        compare(button.contentItem.textFormat,Text.PlainText)
        verify(button.text.indexOf("<img")>=0)
    }
    function test_real_chart_uses_existing_plot_and_zoom_controls() {
        view.report=report([{id:"chart",type:"chart",title:"RxMER",axis_label:"RxMER",unit:"dB",minimum:0,maximum:64,viewport:{start:0,count:3,total:3,points:[{sample_index:0,frequency_hz:640000000,value:40},{sample_index:1,frequency_hz:640025000,value:null},{sample_index:2,frequency_hz:640050000,value:42}]}}])
        tryVerify(function(){return !!loader("chart") && !!loader("chart").item})
        verify(waitForFrame(loader("chart").item))
        compare(loader("chart").item.shown.points[1].value,null)
    }
}
