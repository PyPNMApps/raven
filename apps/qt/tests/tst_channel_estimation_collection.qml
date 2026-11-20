/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick
import QtQuick.Controls
import QtTest
import "helpers"
import "../qml"
RenderTestCase {
    id:testCase
    name:"ChannelEstimationCollections"
    width:1280; height:820; visible:true; when:windowShown
    Theme {id:appearance}
    QtObject {
        id:service
        property string catalog_json:"{}"
        property string view_json:"{}"
        property bool busy:false
        property int selected:-1
        property bool delay:false
        property int start:-1
        property int count:-1
        property int frame:-1
        function show_frame(group,index){selected=group;frame=index}
        function select_group(id){selected=id}
        function select_metric(value){delay=value}
        function show_range(first,length){start=first;count=length}
        function cancel(){busy=false}
    }
    ChannelEstimationCollectionView {id:workspace;anchors.fill:parent;theme:appearance;service:service}
    function init(){workspace.playbackMode=false;workspace.playbackBlocked=false;service.frame=-1;service.busy=false;service.catalog_json="{}";service.view_json="{}";service.selected=-1;service.delay=false;service.start=-1;service.count=-1;appearance.preference=1}
    function cleanup(){const dialog=findChild(workspace,"channelEstimationDeviceDialog");dialog.close();tryCompare(dialog,"visible",false)}
    function result(delay){
        return {group:{mac_address:"00:ca:00:00:00:01",channel_id:159,spacing_hz:50000},capture_count:39,total:1000,start:0,count:1000,delay:delay,minimum:-2,maximum:2,
            points:[{frequency_hz:600000000,sample_index:0,value:0.2},{frequency_hz:610000000,sample_index:500,value:-0.4},{frequency_hz:620000000,sample_index:999,value:0.5}],
            channel_estimation_statistics:delay ? undefined : {minimum:[{frequency_hz:600000000,value:0},{frequency_hz:620000000,value:0.2}],maximum:[{frequency_hz:600000000,value:0.5},{frequency_hz:620000000,value:0.9}]}}
    }
    function test_playback_controls_order_pause_and_zoom(){
        workspace.selectedGroup=7
        workspace.playbackMode=true
        compare(service.frame,0)
        let view=result(false)
        view.frame_index=0;view.frame_count=3;view.capture_time=1000
        service.view_json=JSON.stringify(view)
        const next=findChild(workspace,"channelEstimationNextFrame")
        verify(waitForFrame(next));mouseClick(next);compare(service.frame,1)
        const play=findChild(workspace,"channelEstimationPlayPause")
        verify(waitForFrame(play));mouseClick(play);compare(play.text,"Pause")
        service.frame=-1;tryCompare(service,"frame",1,1500)
        workspace.playbackBlocked=true;compare(play.text,"Play")
        workspace.playbackBlocked=false
        const reset=findChild(workspace,"resetChannelEstimationZoom")
        verify(waitForFrame(reset));mouseClick(reset);compare(service.count,1000)
        workspace.playbackMode=false;compare(service.selected,7)
    }
    function test_header_group_selection_and_busy_guard(){
        service.catalog_json=JSON.stringify({groups:[{id:7,mac_address:"00:ca:00:00:00:01",label:"Channel 159",sample_count:1000},{id:8,mac_address:"00:ca:00:00:00:01",label:"Channel 160",sample_count:1000}]})
        const dialog=findChild(workspace,"channelEstimationDeviceDialog")
        tryCompare(dialog,"opened",true)
        const accept=findChild(workspace,"channelEstimationDeviceContinue")
        verify(waitForFrame(accept));mouseClick(accept)
        compare(service.selected,7);compare(workspace.selectedMac,"00:ca:00:00:00:01")
        tryCompare(dialog,"visible",false)
        service.busy=true;workspace.selectGroup("00:ca:00:00:00:01",8);compare(service.selected,7)
    }
    function test_metrics_trace_controls_and_reset_in_both_themes(){
        for(const preference of [1,2]){
            appearance.preference=preference
            service.view_json=JSON.stringify(result(false))
            const chart=findChild(workspace,"channelEstimationCollectionChart")
            verify(waitForFrame(chart));compare(chart.unit,"dB");compare(chart.axisLabel,"Magnitude")
            const minimum=findChild(workspace,"channelEstimationMinimum")
            verify(waitForFrame(minimum));mouseClick(minimum);compare(chart.showRxmerMinimum,false);mouseClick(minimum)
            const delay=findChild(workspace,"channelEstimationGroupDelay")
            verify(waitForFrame(delay));mouseClick(delay);compare(service.delay,true)
            service.view_json=JSON.stringify(result(true));verify(waitForFrame(chart));compare(chart.unit,"µs");compare(chart.axisLabel,"Group delay")
            compare(minimum.visible,false)
            const reset=findChild(workspace,"resetChannelEstimationZoom")
            verify(waitForFrame(reset));mouseClick(reset);compare(service.start,0);compare(service.count,1000)
        }
    }
    function test_drag_zoom_uses_the_shared_chart_source_range(){
        for(const delay of [false,true]) {
            service.view_json=JSON.stringify(result(delay))
            const chart=findChild(workspace,"channelEstimationCollectionChart")
            const plot=findChild(chart,"measurementPlot")
            verify(waitForFrame(plot))
            const width=plot.width-plot.leftMargin-plot.rightMargin
            const y=plot.topMargin+30
            mousePress(plot,plot.leftMargin+width*0.2,y)
            mouseMove(plot,plot.leftMargin+width*0.8,y)
            mouseRelease(plot,plot.leftMargin+width*0.8,y)
            verify(Math.abs(service.start-200)<=1)
            verify(Math.abs(service.count-600)<=2)
            mouseDoubleClickSequence(plot,plot.leftMargin+width*0.5,y)
            compare(service.start,0);compare(service.count,1000)
        }
    }
}
