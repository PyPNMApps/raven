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
    name: "AiSettings"
    width: 900; height: 680; visible: true; when: windowShown
    Theme { id: appearance }
    QtObject {
        id: service
        property string state_json: "{}"
        property string error_message: ""
        property bool demo_mode: false
        property bool demo_locked: false
        property bool busy: false
        property string operation_status: ""
        property int signIns: 0
        function sign_in_chatgpt(){signIns++;return ""}
        function sign_out_chatgpt(){}
        function select_chatgpt_model(model){return true}
        property int submissions: 0
        property string receivedProvider: ""
        property string receivedModel: ""
        property bool receivedKey: false
        function configure(provider,model,key) {
            submissions++
            receivedProvider=provider;receivedModel=model;receivedKey=key.length>0
            return error_message.length===0
        }
        function read_key_file(url){return "synthetic-file-key"}
        function clear_error(){error_message=""}
        function forget(provider){return true}
        function set_enabled(enabled){return true}
    }
    AiServicesDialog { id: dialog; theme: appearance; service: service; parent: testCase }
    AiAssistanceChip {
        id: chip
        theme: appearance
        configuration: JSON.parse(service.state_json)
        onClicked: dialog.open()
    }
    function init() {
        service.signIns=0;service.demo_mode=false;service.demo_locked=false;
        appearance.preference=1
        service.state_json=JSON.stringify({selected:null,enabled:false,providers:[
            {id:"openai",label:"OpenAI",model:"",configured:false},
            {id:"gemini",label:"Gemini",model:"existing-model",configured:true},
            {id:"claude",label:"Claude",model:"",configured:false},
            {id:"grok",label:"Grok",model:"",configured:false}]})
        service.error_message="";service.submissions=0;service.receivedKey=false
    }
    function cleanup(){dialog.close()}
    function test_chatgpt_sign_in_is_explicit_and_hidden_in_demo() {
        dialog.open()
        compare(service.signIns,0)
        const signIn=findChild(dialog,"aiChatGptSignIn")
        verify(waitForFrame(signIn))
        mouseClick(signIn)
        compare(service.signIns,1)
        service.demo_mode=true
        verify(!signIn.visible)
        compare(service.submissions,0)
    }
    function test_chatgpt_selection_uses_catalog_without_api_key() {
        service.state_json=JSON.stringify({selected:"chatgpt",enabled:true,chatgpt:{signed_in:true,model:"account-model",models:[{id:"account-model",label:"Account model"}]},providers:[{id:"chatgpt",label:"ChatGPT (subscription)",model:"account-model",configured:true}]})
        dialog.open()
        verify(findChild(dialog,"aiChatGptModel").visible)
        verify(!findChild(dialog,"aiApiKey").visible)
        verify(!findChild(dialog,"aiSaveSession").visible)
        verify(findChild(dialog,"aiChatGptSignOut").visible)
        compare(service.signIns,0)
    }
    function test_demo_setup_has_no_credential_or_connection_controls() {
        service.demo_mode=true
        dialog.open()
        verify(!findChild(dialog,"aiApiKey").visible)
        verify(!findChild(dialog,"aiTestConnection").visible)
        verify(!findChild(dialog,"aiSaveSession").visible)
        verify(findChild(dialog,"aiConfigurationStatus").text.indexOf("Demo active")>=0)
        compare(service.submissions,0)
    }
    function test_gray_chip_is_clickable_and_opens_setup_data() {
        verify(chip.enabled)
        compare(chip.text,"AI Assistance · Off")
        verify(waitForFrame(chip))
        mouseClick(chip)
        tryCompare(dialog,"visible",true)
        compare(findChild(dialog,"aiProvider").count,4)
        compare(findChild(dialog,"aiApiKey").echoMode,TextInput.Password)
        verify(!findChild(dialog,"aiSaveSession").enabled)
    }
    function test_file_picker_populates_masked_field_without_saving() {
        dialog.open()
        const picker=findChild(dialog,"aiKeyFilePicker")
        verify(findChild(dialog,"aiOpenKeyFile").enabled)
        picker.selectedFile="file:///synthetic-key.txt"
        picker.accepted()
        compare(findChild(dialog,"aiApiKey").text,"synthetic-file-key")
        compare(findChild(dialog,"aiApiKey").echoMode,TextInput.Password)
        compare(service.submissions,0)
        compare(picker.selectedFile.toString(),"")
        dialog.close()
        compare(findChild(dialog,"aiApiKey").text,"")
    }
    function test_key_clears_on_provider_change_close_and_successful_submit() {
        dialog.open()
        const key=findChild(dialog,"aiApiKey")
        const model=findChild(dialog,"aiModel")
        const provider=findChild(dialog,"aiProvider")
        key.text="synthetic-sentinel"
        provider.currentIndex=1;provider.activated(1)
        compare(key.text,"")
        compare(model.text,"existing-model")
        key.text="synthetic-sentinel"
        dialog.close()
        compare(key.text,"")
        dialog.open()
        provider.currentIndex=3;provider.activated(3)
        model.text="grok-model";key.text="synthetic-sentinel"
        const save=findChild(dialog,"aiSaveSession")
        verify(waitForFrame(save))
        mouseClick(save)
        compare(service.submissions,1)
        compare(service.receivedProvider,"grok")
        compare(service.receivedModel,"grok-model")
        verify(service.receivedKey)
        compare(key.text,"")
        tryCompare(dialog,"visible",false)
    }
    function test_rejection_clears_password_and_preserves_open_dialog() {
        dialog.open()
        const key=findChild(dialog,"aiApiKey")
        findChild(dialog,"aiModel").text="model"
        key.text="synthetic-sentinel"
        service.error_message="Invalid configuration"
        const save=findChild(dialog,"aiSaveSession")
        verify(waitForFrame(save))
        mouseClick(save)
        compare(key.text,"")
        verify(dialog.visible)
        verify(findChild(dialog,"aiSetupError").visible)
    }
    function test_configured_chip_does_not_claim_verified_or_analysis_available() {
        service.state_json=JSON.stringify({selected:"openai",enabled:true,providers:[]})
        compare(chip.text,"AI Assistance · Configured")
        verify(chip.configured)
        appearance.preference=2
        verify(waitForFrame(chip))
        verify(chip.enabled)
    }
}
