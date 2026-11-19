/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Explicit OFDM AI operations; retained credentials remain in Rust session state.
#[cxx_qt::bridge]
mod bridge {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        include!("cxx-qt-lib/qurl.h");
        type QUrl = cxx_qt_lib::QUrl;
        type QString = cxx_qt_lib::QString;
    }
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, state_json)]
        #[qproperty(QString, saved_json)]
        #[qproperty(QString, credential_status)]
        #[qproperty(QString, reauth_provider)]
        #[qproperty(QString, error_message)]
        #[qproperty(QString, preview_json)]
        #[qproperty(QString, report_json)]
        #[qproperty(QString, findings_text)]
        #[qproperty(QString, operation_status)]
        #[qproperty(QString, conversation_json)]
        #[qproperty(bool, demo_mode)]
        #[qproperty(bool, demo_locked)]
        #[qproperty(bool, busy)]
        #[qproperty(bool, request_ready)]
        type AiBackend = super::AiBackendState;
        /// Stores a masked form submission for this session only; no API call.
        #[qinvokable]
        fn configure(
            self: Pin<&mut AiBackend>,
            provider: &QString,
            model: &QString,
            key: &QString,
        ) -> bool;
        /// Disables assistance or reenables a configured provider without key removal.
        #[qinvokable]
        fn set_enabled(self: Pin<&mut AiBackend>, enabled: bool) -> bool;
        /// Removes one provider's session configuration; no remote revocation.
        #[qinvokable]
        fn forget(self: Pin<&mut AiBackend>, provider: &QString) -> bool;
        /// Reads an explicitly selected local key file into temporary masked input.
        #[qinvokable]
        fn read_key_file(self: Pin<&mut AiBackend>, url: &QUrl) -> QString;
        /// Updates immutable run identity, cancelling old work and invalidating approval.
        #[qinvokable]
        fn context_changed(self: Pin<&mut AiBackend>, source: &QString);
        /// Builds an offline exact request preview for the current OFDM evidence.
        #[qinvokable]
        fn prepare_analysis(self: Pin<&mut AiBackend>, evidence: &QString, question: &QString);
        /// Dispatches only the currently displayed single-use request digest.
        #[qinvokable]
        fn analyze(self: Pin<&mut AiBackend>, digest: &QString);
        /// Invalidates approval when question, settings or popup state changes.
        #[qinvokable]
        fn invalidate(self: Pin<&mut AiBackend>);
        /// Tests selected model metadata access only, with no PNM or inference body.
        #[qinvokable]
        fn test_connection(self: Pin<&mut AiBackend>);
        /// Requests cooperative cancellation; busy remains true until acknowledged.
        #[qinvokable]
        fn cancel(self: Pin<&mut AiBackend>);
        /// Exports the current bounded conversation explicitly to a new local file.
        #[qinvokable]
        fn export_conversation(self: Pin<&mut AiBackend>, url: &QUrl);
        /// Selects local demo mode; --ai-demo launch sessions cannot enable live AI.
        #[qinvokable]
        fn choose_demo_mode(self: Pin<&mut AiBackend>, enabled: bool);
        /// Starts explicit ChatGPT browser sign-in and returns its authorization URL.
        #[qinvokable]
        fn sign_in_chatgpt(self: Pin<&mut AiBackend>) -> QString;
        /// Selects a verified ChatGPT account model and invalidates existing approval.
        #[qinvokable]
        fn select_chatgpt_model(self: Pin<&mut AiBackend>, model: &QString) -> bool;
        /// Clears local ChatGPT tokens and attempts remote session revocation.
        #[qinvokable]
        fn sign_out_chatgpt(self: Pin<&mut AiBackend>);
        /// Saves or removes the active credential through an explicit OS-vault operation.
        #[qinvokable]
        fn remember_connection(self: Pin<&mut AiBackend>, provider: &QString, remember: bool);
        /// Restores one remembered connection without sending analysis evidence.
        #[qinvokable]
        fn restore_connection(self: Pin<&mut AiBackend>, provider: &QString);
        /// Forgets a saved provider and clears its active in-memory configuration.
        #[qinvokable]
        fn forget_connection(self: Pin<&mut AiBackend>, provider: &QString);
        /// Records explicit consent to remember the next successful ChatGPT sign-in.
        #[qinvokable]
        fn remember_next_signin(self: Pin<&mut AiBackend>, remember: bool);
        /// Updates known sign-in expiry locally without a provider request.
        #[qinvokable]
        fn check_credential_status(self: Pin<&mut AiBackend>);
        /// Clears a form diagnostic without changing credentials.
        #[qinvokable]
        fn clear_error(self: Pin<&mut AiBackend>);
    }
    impl cxx_qt::Threading for AiBackend {}
}

#[path = "ai/credentials.rs"]
mod credentials;
#[path = "ai/session.rs"]
mod session;
use session::AiBackendState;
use std::pin::Pin;
