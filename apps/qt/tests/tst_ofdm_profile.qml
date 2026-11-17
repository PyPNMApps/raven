/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtTest
import "../qml"
import "helpers"

RenderTestCase {
    id:testCase
    name:"OfdmProfilePerformance"
    width:1280;height:820;visible:true;when:windowShown
    Theme {id:appearance}
    QtObject {
        id:service
        property bool busy:false
        property bool prepared:false
        property bool running:false
        property bool selection_available:prepared || JSON.parse(chart_json).available_profiles !== undefined
        property string inputs_json:"{}"
        property string chart_json:"{}"
        property string result_json:"{}"
        property string error_message:""
        property int preparations:0
        property bool requestedMce:false
        property int selectedProfile:-1
        property int runs:0
        property var selectedProfiles:[]
        function choose_file(role,url){}
        function clear_channel(){inputs_json=JSON.stringify({profile:"profile.bin",rxmer:"rxmer.bin",profiles:[0,3]});prepared=false}
        function prepare_many(profiles,mce,tolerance){busy=true;preparations++;selectedProfiles=JSON.parse(profiles);selectedProfile=selectedProfiles[0];requestedMce=mce}
        function select_profiles(profiles){
            const chart=JSON.parse(chart_json)
            chart.selected_profiles=JSON.parse(profiles)
            chart_json=JSON.stringify(chart)
            prepared=chart.selected_profiles.length>0
            result_json="{}"
        }
        function run(){runs++;running=true;busy=true}
        function cancel(){running=false;busy=false}
        function show_range(start,count){}
    }
    OfdmProfileWorkspace {id:workspace;anchors.fill:parent;theme:appearance;service:service}
    function cleanup(){
        service.busy=false;service.running=false;service.prepared=false
        service.inputs_json="{}";service.chart_json="{}";service.result_json="{}";service.error_message=""
        service.preparations=0;service.runs=0
        workspace.closeDialogs()
    }
    function test_required_inputs_mce_and_continue_wait_for_preflight(){
        compare(workspace.capturePicker.options & FileDialog.DontUseNativeDialog,0)
        workspace.openInputs()
        const next=findChild(workspace,"ofdmContinue")
        const mce=findChild(workspace,"ofdmUseMce")
        verify(!next.enabled);verify(!mce.enabled)
        service.inputs_json=JSON.stringify({profile:"profile.bin",rxmer:"rxmer.bin",profiles:[0,3]})
        tryCompare(next,"enabled",true)
        mouseClick(next)
        compare(service.preparations,1);compare(service.selectedProfiles,[0,3]);compare(service.requestedMce,false)
        verify(workspace.inputsVisible)
        service.error_message="Grid mismatch";service.busy=false
        verify(workspace.inputsVisible)
        service.error_message=""
        service.inputs_json=JSON.stringify({profile:"profile.bin",rxmer:"rxmer.bin",channel:"channel.bin",profiles:[0,3]})
        tryCompare(mce,"enabled",true)
        mce.checked=true
        mouseClick(next)
        compare(service.requestedMce,true)
        service.prepared=true;service.busy=false
        tryCompare(workspace,"inputsVisible",false)
    }
    function test_checkboxes_choose_a_subset_and_disable_empty_runs(){
        workspace.openInputs()
        service.inputs_json=JSON.stringify({profile:"profile.bin",rxmer:"rxmer.bin",profiles:[0,3]})
        tryVerify(function(){return findChild(workspace.inputSurface,"ofdmInputProfile0")!==null})
        const first=findChild(workspace.inputSurface,"ofdmInputProfile0")
        const second=findChild(workspace.inputSurface,"ofdmInputProfile3")
        verify(first.checked && second.checked)
        verify(waitForFrame(first))
        mouseClick(first)
        mouseClick(findChild(workspace,"ofdmContinue"))
        compare(service.selectedProfiles,[3])
        service.prepared=true;service.busy=false
        service.chart_json=JSON.stringify({available_profiles:[0,3],selected_profiles:[3],viewport:{start:0,count:0,total:0,points:[]},profiles:[]})
        const run=findChild(workspace,"ofdmFecPerformance")
        verify(run.enabled)
        tryVerify(function(){return findChild(workspace,"ofdmRunProfile3")!==null})
        const third=findChild(workspace,"ofdmRunProfile3")
        verify(waitForFrame(third))
        mouseClick(third)
        tryCompare(run,"enabled",false)
        mouseClick(findChild(workspace,"ofdmRunProfile0"))
        tryCompare(run,"enabled",true)
        compare(JSON.parse(service.chart_json).selected_profiles,[0])
    }
    function test_fec_results_preserve_independent_profile_rows(){
        workspace.openResults()
        service.result_json=JSON.stringify({status:"Complete",entries:[
            {profile_id:0,status:"Complete",fec:{counters:{total_cw:5,ccw:1,clean_cw:4,ufec:0}},profile:{compared_bits:100,bit_errors:0,post_fec_ber:0}},
            {profile_id:3,status:"NotStarted"}
        ]})
        tryVerify(function(){return findChild(workspace.resultsSurface,"ofdmFecProfile0Column2")!==null})
        compare(findChild(workspace.resultsSurface,"ofdmFecProfile0Column2").text,"5")
        compare(findChild(workspace.resultsSurface,"ofdmFecProfile3Column2").text,"—")
    }
    function test_chart_has_overlay_and_results_require_successful_preflight(){
        const fec=findChild(workspace,"ofdmFecPerformance")
        verify(!fec.enabled)
        service.chart_json=JSON.stringify({available_profiles:[0,3],selected_profiles:[3],plc:[{first_hz:101000000,last_hz:102000000}],
            viewport:{start:0,count:3,total:3,points:[{sample_index:0,frequency_hz:100000000,value:40},{sample_index:1,frequency_hz:101000000,value:38},{sample_index:2,frequency_hz:102000000,value:41}]},
            profiles:[{profile_id:3,points:[{sample_index:0,frequency_hz:100000000,value:36},{sample_index:1,frequency_hz:101000000,value:null},{sample_index:2,frequency_hz:102000000,value:36}]}]})
        service.prepared=true
        tryCompare(fec,"enabled",true)
        const chart=findChild(workspace,"ofdmProfileChart")
        compare(chart.extraSeries.length,1);compare(chart.shadedBands.length,1)
        mouseClick(fec)
        compare(service.runs,1)
        verify(service.running)
    }
}
