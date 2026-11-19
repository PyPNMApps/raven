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
    width: 900; height: 720; visible: true; when: windowShown
    Theme { id: appearance }
    QtObject {
        id: service
        property string state_json: "{}"
        property string saved_json: "{}"
        property string credential_status: ""
        property string reauth_provider: ""
        property int rememberCalls: 0
        property bool remembered: false
        property int restores: 0
        function remember_connection(provider, remember) { rememberCalls++; remembered=remember }
        function restore_connection(provider) { restores++ }
        function forget_connection(provider) { if(provider==="chatgpt")sign_out_chatgpt(); else forget(provider) }
        function remember_next_signin(remember) { remembered=remember }
        function check_credential_status() {}
        property string error_message: ""
        property bool demo_mode: false
        property bool demo_locked: false
        property bool busy: false
        property string operation_status: ""
        property int signIns: 0
        property int signOuts: 0
        property int tests: 0
        property int cancellations: 0
        property int submissions: 0
        property string receivedProvider: ""
        property string receivedModel: ""
        property bool receivedKey: false
        function sign_in_chatgpt() { signIns++; return "" }
        function sign_out_chatgpt() { signOuts++ }
        function select_chatgpt_model(model) { receivedModel=model; return true }
        function test_connection() { tests++ }
        function cancel() { cancellations++ }
        function configure(provider,model,key) {
            submissions++
            receivedProvider=provider; receivedModel=model; receivedKey=key.length>0
            return error_message.length===0
        }
        function read_key_file(url) { return "synthetic-file-key" }
        function clear_error() { error_message="" }
        function forget(provider) { receivedProvider=provider; return true }
        function set_enabled(enabled) { return true }
    }
    AiServicesDialog { id: dialog; theme: appearance; service: service; parent: testCase }
    AiAssistanceChip {
        id: chip
        theme: appearance
        configuration: JSON.parse(service.state_json)
        onClicked: dialog.open()
    }
    function init() {
        service.rememberCalls=0; service.remembered=false; service.restores=0; service.saved_json="{}"; service.reauth_provider=""; service.credential_status="";
        service.signIns=0; service.signOuts=0; service.tests=0; service.cancellations=0
        service.demo_mode=false; service.demo_locked=false; service.busy=false
        appearance.preference=1
        service.state_json=JSON.stringify({selected:null,enabled:false,providers:[
            {id:"openai",label:"OpenAI",model:"",configured:false},
            {id:"chatgpt",label:"ChatGPT (subscription)",model:"",configured:false},
            {id:"gemini",label:"Gemini",model:"existing-model",configured:true},
            {id:"claude",label:"Claude",model:"",configured:false},
            {id:"grok",label:"Grok",model:"",configured:false}]})
        service.error_message=""; service.operation_status=""; service.submissions=0; service.receivedKey=false
        service.receivedProvider=""; service.receivedModel=""
    }
    function cleanup() { dialog.close() }
    function choose(id) {
        const choice=findChild(dialog,"aiProvider")
        choice.currentIndex=dialog.providers.findIndex(function(provider){return provider.id===id})
        choice.activated(choice.currentIndex)
    }
    function settingsControl(name) { return findChild(dialog.providerSettingsDialog,name) }
    function configure(id) {
        choose(id)
        dialog.configureProvider()
        tryCompare(dialog.providerSettingsDialog,"visible",true)
        verify(dialog.providerSettingsDialog.footer.contentItem !== null,
               "Provider footer has no visual host")
    }
    function click(control) {
        // Popup controls join the overlay window asynchronously after open().
        tryVerify(function() { return control.Window.window !== null }, 2000,
                  "Popup control did not attach to its window")
        verify(waitForFrame(control))
        mouseClick(control)
    }
    function test_main_popup_is_compact_and_sign_in_is_explicit() {
        dialog.open()
        compare(service.signIns,0)
        compare(findChild(dialog,"aiConnect").text,"Connect")
        verify(!dialog.providerSettingsDialog.visible)
        click(findChild(dialog,"aiConnect"))
        compare(service.signIns,1)
        compare(service.tests,0)
    }
    function test_chatgpt_model_catalog_is_separate_and_has_no_key() {
        service.state_json=JSON.stringify({selected:"chatgpt",enabled:true,chatgpt:{signed_in:true,model:"account-model",models:[{id:"account-model",label:"Account model"},{id:"second-model",label:"Second model"}]},providers:[{id:"chatgpt",label:"ChatGPT (subscription)",model:"account-model",configured:true}]})
        dialog.open()
        dialog.configureProvider()
        verify(settingsControl("aiChatGptModel").visible)
        verify(!settingsControl("aiApiKey").visible)
        const picker=settingsControl("aiChatGptModel")
        picker.currentIndex=1; picker.activated(1)
        compare(service.receivedModel,"")
        click(settingsControl("aiSaveSession"))
        compare(service.receivedModel,"second-model")
        compare(service.signIns,0)
        compare(service.submissions,0)
        click(findChild(dialog,"aiDisconnect"))
        compare(service.signOuts,1)
    }
    function test_demo_has_no_connection_or_configuration_controls() {
        service.demo_mode=true
        dialog.open()
        verify(!findChild(dialog,"aiConnect").visible)
        verify(!findChild(dialog,"aiConfigureProvider").visible)
        dialog.configureProvider()
        dialog.connectProvider()
        verify(!dialog.providerSettingsDialog.visible)
        compare(service.signIns,0); compare(service.submissions,0); compare(service.tests,0)
        verify(findChild(dialog,"aiConfigurationStatus").text.indexOf("Demo active")>=0)
    }
    function test_gray_chip_opens_compact_setup_in_both_themes() {
        for (let mode of [1,2]) {
            appearance.preference=mode
            compare(chip.text,"AI Assistance · Off")
            click(chip)
            tryCompare(dialog,"visible",true)
            compare(findChild(dialog,"aiProvider").count,5)
            verify(!dialog.providerSettingsDialog.visible)
            dialog.close()
        }
    }
    function test_unconfigured_connect_opens_settings_without_network() {
        dialog.open(); choose("openai")
        click(findChild(dialog,"aiConnect"))
        tryCompare(dialog.providerSettingsDialog,"visible",true)
        compare(settingsControl("aiApiKey").echoMode,TextInput.Password)
        verify(!settingsControl("aiSaveSession").enabled)
        compare(service.tests,0); compare(service.submissions,0)
    }
    function test_file_picker_is_masked_local_only_and_ignores_late_result() {
        dialog.open(); configure("openai")
        const picker=settingsControl("aiKeyFilePicker")
        verify(settingsControl("aiOpenKeyFile").enabled)
        picker.accepted()
        compare(settingsControl("aiApiKey").text,"synthetic-file-key")
        compare(service.submissions,0)
        dialog.providerSettingsDialog.close()
        picker.accepted()
        compare(settingsControl("aiApiKey").text,"")
    }
    function test_provider_change_parent_close_and_demo_clear_temporary_key() {
        dialog.open(); configure("openai")
        const key=settingsControl("aiApiKey")
        key.text="synthetic-sentinel"
        choose("gemini")
        verify(!dialog.providerSettingsDialog.visible)
        compare(key.text,"")
        configure("gemini")
        compare(settingsControl("aiModel").text,"existing-model")
        key.text="synthetic-sentinel"
        dialog.close(); compare(key.text,"")
        dialog.open(); configure("openai")
        key.text="synthetic-sentinel"; service.demo_mode=true
        verify(!dialog.providerSettingsDialog.visible); compare(key.text,"")
    }
    function test_save_keeps_main_popup_open_and_never_connects() {
        dialog.open(); configure("grok")
        const key=settingsControl("aiApiKey")
        settingsControl("aiModel").text="grok-model"; key.text="synthetic-sentinel"
        click(settingsControl("aiSaveSession"))
        compare(service.submissions,1); compare(service.receivedProvider,"grok")
        compare(service.receivedModel,"grok-model"); verify(service.receivedKey)
        compare(key.text,"")
        verify(dialog.visible); verify(!dialog.providerSettingsDialog.visible)
        compare(service.tests,0); compare(service.signIns,0)
    }
    function test_rejection_clears_key_and_preserves_settings() {
        dialog.open(); configure("openai")
        const key=settingsControl("aiApiKey")
        settingsControl("aiModel").text="model"; key.text="synthetic-sentinel"
        service.error_message="Invalid configuration"
        click(settingsControl("aiSaveSession"))
        compare(key.text,"")
        verify(dialog.providerSettingsDialog.visible)
        verify(settingsControl("aiProviderSettingsError").visible)
    }
    function test_openai_connect_uses_retained_key_for_explicit_metadata_check() {
        service.state_json=JSON.stringify({selected:"openai",enabled:false,providers:[{id:"openai",label:"OpenAI",model:"saved-model",configured:true}]})
        dialog.open()
        compare(service.tests,0)
        click(findChild(dialog,"aiConnect"))
        compare(service.receivedProvider,"openai"); compare(service.receivedModel,"saved-model")
        verify(!service.receivedKey)
        compare(service.tests,1)
    }
    function test_setup_only_providers_have_disabled_connect() {
        dialog.open()
        for (let id of ["gemini","claude","grok"]) {
            choose(id)
            verify(!findChild(dialog,"aiConnect").enabled)
            verify(findChild(dialog,"aiConfigureProvider").enabled)
            verify(findChild(dialog,"aiConfigurationStatus").text.indexOf("Setup only")>=0)
            dialog.connectProvider()
        }
        compare(service.tests,0); compare(service.signIns,0); compare(service.submissions,0)
    }
    function test_busy_disables_actions_and_close_cancels() {
        dialog.open(); service.busy=true
        verify(!findChild(dialog,"aiConnect").enabled)
        verify(!findChild(dialog,"aiConfigureProvider").enabled)
        verify(!findChild(dialog,"aiProvider").enabled)
        dialog.close(); compare(service.cancellations,1)
    }
    function test_remember_key_is_explicit_and_default_off() {
        dialog.open(); configure("openai")
        const remember=settingsControl("aiRememberKey")
        verify(!remember.checked)
        remember.checked=true
        settingsControl("aiModel").text="model"; settingsControl("aiApiKey").text="synthetic-sentinel"
        click(settingsControl("aiSaveSession"))
        compare(service.rememberCalls,1); verify(service.remembered)
        compare(service.tests,0)
    }
    function test_connect_restores_saved_credentials_without_signing_in() {
        service.saved_json=JSON.stringify({providers:{chatgpt:"account-model"},preferred:"chatgpt"})
        dialog.open()
        click(findChild(dialog,"aiConnect"))
        compare(service.restores,1); compare(service.signIns,0)
        service.reauth_provider="chatgpt"
        click(findChild(dialog,"aiConnect"))
        compare(service.signIns,1)
    }
    function test_rejected_authentication_chip_requests_reconnect() {
        service.state_json=JSON.stringify({enabled:false,selected:"openai",authentication_required:true})
        compare(chip.text,"AI Assistance · Reconnect")
    }
    function test_configured_chip_does_not_claim_verified() {
        service.state_json=JSON.stringify({selected:"openai",enabled:true,providers:[]})
        compare(chip.text,"AI Assistance · Configured")
        verify(chip.configured)
    }
}
