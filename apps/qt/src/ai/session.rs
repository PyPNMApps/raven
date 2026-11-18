/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Session settings, explicit AI workers and read-only presentation behind CXX-Qt.
use super::bridge;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QUrl};
use std::{
    io::Write,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use viewer::ai::Session;

/// Nonsecret Qt presentation with private Rust-owned ephemeral provider state.
pub struct AiBackendState {
    pub(super) state_json: QString,
    pub(super) error_message: QString,
    pub(super) preview_json: QString,
    pub(super) report_json: QString,
    pub(super) findings_text: QString,
    pub(super) operation_status: QString,
    pub(super) conversation_json: QString,
    pub(super) demo_mode: bool,
    pub(super) demo_locked: bool,
    forced_demo: bool,
    pub(super) busy: bool,
    pub(super) request_ready: bool,
    source: String,
    generation: u64,
    control: Arc<AtomicBool>,
    turns: Vec<serde_json::Value>,
    session: Session,
}
impl Default for AiBackendState {
    /// Creates an unconfigured service without network, disk or environment access.
    fn default() -> Self {
        let session = Session::default();
        let forced_demo = std::env::args_os().any(|argument| argument == "--ai-demo");
        Self {
            state_json: QString::from(session.catalog().to_string()),
            error_message: QString::default(),
            preview_json: QString::from("{}"),
            report_json: QString::from("{}"),
            findings_text: QString::default(),
            operation_status: QString::default(),
            conversation_json: QString::from("[]"),
            demo_mode: forced_demo,
            demo_locked: forced_demo,
            forced_demo,
            busy: false,
            request_ready: false,
            source: String::new(),
            generation: 0,
            control: Arc::new(AtomicBool::new(false)),
            turns: Vec::new(),
            session,
        }
    }
}
impl bridge::AiBackend {
    /// Validates a form submission and immediately returns a nonsecret result.
    /// # Arguments
    /// * `provider`, `model` - Explicit bounded configuration IDs.
    /// * `key` - Transient Qt password text; the UI clears its copy after this call.
    /// # Returns
    /// True when configured, false with a sanitized local diagnostic otherwise.
    pub fn configure(
        mut self: Pin<&mut Self>,
        provider: &QString,
        model: &QString,
        key: &QString,
    ) -> bool {
        self.as_mut().invalidate();
        let result = self.as_mut().rust_mut().session.configure(
            &provider.to_string(),
            &model.to_string(),
            key.to_string(),
        );
        self.publish(result)
    }
    /// Changes local availability without making requests or forgetting keys.
    /// # Arguments
    /// * `enabled` - Desired local setup state.
    /// # Returns
    /// Success, or false when no selected configuration exists.
    pub fn set_enabled(mut self: Pin<&mut Self>, enabled: bool) -> bool {
        self.as_mut().invalidate();
        let result = self.as_mut().rust_mut().session.set_enabled(enabled);
        self.publish(result)
    }
    /// Forgets a provider's ephemeral key; never contacts the provider.
    /// # Arguments
    /// * `provider` - Stable supported provider ID.
    /// # Returns
    /// True on local removal; false for an unsupported ID.
    pub fn forget(mut self: Pin<&mut Self>, provider: &QString) -> bool {
        self.as_mut().invalidate();
        let result = self
            .as_mut()
            .rust_mut()
            .session
            .forget(&provider.to_string());
        self.publish(result)
    }
    /// Reads a bounded key file only after an explicit native file selection.
    /// # Arguments
    /// * `url` - User-selected local URL; remote URLs are rejected.
    /// # Returns
    /// Transient key text for the masked form, or empty with a generic diagnostic.
    /// Does not expose existing session keys, retain paths or change configuration.
    pub fn read_key_file(mut self: Pin<&mut Self>, url: &QUrl) -> QString {
        let result =
            crate::backend::local_path(url).and_then(|path| viewer::ai::read_key_file(&path));
        match result {
            Ok(key) => {
                self.as_mut().set_error_message(QString::default());
                QString::from(key.as_str())
            }
            Err(_) => {
                self.set_error_message(QString::from("Select a local plain-text file containing only one API key (up to 4096 bytes)."));
                QString::default()
            }
        }
    }
    /// Clears the current sanitized form error without changing setup state.
    pub fn clear_error(self: Pin<&mut Self>) {
        self.set_error_message(QString::default());
    }
    fn publish(
        mut self: Pin<&mut Self>,
        result: Result<serde_json::Value, viewer::Diagnostic>,
    ) -> bool {
        match result {
            Ok(catalog) => {
                self.as_mut()
                    .set_state_json(QString::from(catalog.to_string()));
                self.set_error_message(QString::default());
                true
            }
            Err(error) => {
                self.set_error_message(QString::from(error.message));
                false
            }
        }
    }
}

impl Drop for AiBackendState {
    fn drop(&mut self) {
        self.control.store(true, Ordering::Relaxed);
    }
}
impl bridge::AiBackend {
    /// Rejects stale run associations before any later approval or worker result.
    /// # Arguments
    /// * `source` - Native OFDM generation ID; never derived from JS numeric counters.
    pub fn context_changed(mut self: Pin<&mut Self>, source: &QString) {
        if self.rust().source == source.to_string() {
            return;
        }
        self.as_mut().invalidate();
        self.as_mut().rust_mut().source = source.to_string();
        self.as_mut().rust_mut().turns.clear();
        self.as_mut().set_conversation_json(QString::from("[]"));
        self.as_mut().set_report_json(QString::from("{}"));
        self.set_findings_text(QString::default());
    }
    /// Invalidates approval and cooperatively cancels an in-flight operation.
    pub fn invalidate(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().session.invalidate_analysis();
        self.rust().control.store(true, Ordering::Relaxed);
        self.as_mut().set_request_ready(false);
        if !self.rust().busy {
            self.as_mut().set_operation_status(QString::default());
        }
        self.set_preview_json(QString::from("{}"));
    }
    /// Freezes the complete request without any network operation.
    /// # Arguments
    /// * `evidence` - Strict native-run snapshot, passed as original JSON text.
    /// * `question` - Bounded editable question, no implicit transcript context.
    pub fn prepare_analysis(mut self: Pin<&mut Self>, evidence: &QString, question: &QString) {
        if self.rust().busy {
            return;
        }
        let source = self.rust().source.clone();
        let result = if self.rust().is_demo() {
            self.as_mut().rust_mut().session.prepare_demo(
                &evidence.to_string(),
                &question.to_string(),
                &source,
            )
        } else {
            self.as_mut().rust_mut().session.prepare_analysis(
                &evidence.to_string(),
                &question.to_string(),
                &source,
            )
        };
        self.as_mut().set_request_ready(false);
        match result {
            Ok(preview) => {
                self.as_mut()
                    .set_findings_text(QString::from(preview["findings"].as_str().unwrap_or("")));
                self.as_mut()
                    .set_preview_json(QString::from(preview.to_string()));
                self.as_mut().set_error_message(QString::default());
                self.set_request_ready(true);
            }
            Err(error) => {
                self.as_mut().set_preview_json(QString::from("{}"));
                self.set_error_message(QString::from(error.message));
            }
        }
    }
    /// Runs a no-capture model-metadata probe on the owned worker.
    pub fn test_connection(mut self: Pin<&mut Self>) {
        if self.rust().busy {
            return;
        }
        if self.rust().is_demo() {
            self.set_operation_status(QString::from(
                "Demo mode is local. No provider connection is tested.",
            ));
            return;
        }
        let probe = match self.rust().session.connection_probe() {
            Ok(probe) => probe,
            Err(error) => {
                self.set_error_message(QString::from(error.message));
                return;
            }
        };
        self.as_mut().invalidate();
        self.start("Checking model access…", move |cancel| {
            probe.execute(cancel)?;
            Ok(serde_json::json!({"connection":"API key accepted; selected model metadata is accessible. Inference has not been tested."}))
        });
    }
    /// Consumes the displayed approval and dispatches once on a worker.
    /// # Arguments
    /// * `digest` - Exact preview body digest, checked against current run/settings.
    pub fn analyze(mut self: Pin<&mut Self>, digest: &QString) {
        if self.rust().busy {
            return;
        }
        let source = self.rust().source.clone();
        if self.rust().is_demo() {
            let request = match self
                .as_mut()
                .rust_mut()
                .session
                .approve_demo(&digest.to_string(), &source)
            {
                Ok(request) => request,
                Err(error) => {
                    self.set_error_message(QString::from(error.message));
                    return;
                }
            };
            self.as_mut().set_request_ready(false);
            self.start("Creating local demo response…", move |cancel| {
                request.execute(cancel)
            });
            return;
        }
        let request = match self
            .as_mut()
            .rust_mut()
            .session
            .approve_analysis(&digest.to_string(), &source)
        {
            Ok(request) => request,
            Err(error) => {
                self.set_error_message(QString::from(error.message));
                return;
            }
        };
        self.as_mut().set_request_ready(false);
        self.start("Analyzing approved evidence…", move |cancel| {
            request.execute(&viewer::ai::transport::HttpsTransport, cancel)
        });
    }
    /// Changes mode while idle and invalidates any approval before switching.
    /// # Arguments
    /// * `enabled` - Desired mode; false is ignored in a --ai-demo launch session.
    pub fn choose_demo_mode(mut self: Pin<&mut Self>, enabled: bool) {
        if self.rust().busy {
            return;
        }
        self.as_mut().invalidate();
        let demo = self.rust().forced_demo || enabled;
        self.as_mut().set_demo_mode(demo);
        self.set_report_json(QString::from("{}"));
    }
    /// Requests stop; an already-sent provider request may still be billed.
    pub fn cancel(mut self: Pin<&mut Self>) {
        self.as_mut().invalidate();
        self.set_operation_status(QString::from("Cancelling…"));
    }
    /// Writes a user-approved transcript export with no overwrite or automatic save.
    /// # Arguments
    /// * `url` - Explicit local destination from the native Save dialog.
    pub fn export_conversation(self: Pin<&mut Self>, url: &QUrl) {
        let result = (|| {
            let path = crate::backend::local_path(url)?;
            let record = serde_json::json!({"schema_version":1,"source":self.rust().source,"turns":self.rust().turns});
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(|_| {
                    viewer::Diagnostic::new(
                        "AiExport",
                        "Could not create a new conversation file. Choose a different destination.",
                    )
                })?;
            file.write_all(record.to_string().as_bytes()).map_err(|_| {
                viewer::Diagnostic::new("AiExport", "Could not write the conversation file.")
            })?;
            Ok::<(), viewer::Diagnostic>(())
        })();
        match result {
            Ok(()) => self.set_operation_status(QString::from("Conversation exported.")),
            Err(error) => self.set_error_message(QString::from(error.message)),
        }
    }
    fn start(
        mut self: Pin<&mut Self>,
        message: &str,
        work: impl FnOnce(&AtomicBool) -> Result<serde_json::Value, viewer::Diagnostic> + Send + 'static,
    ) {
        let generation = self.rust().generation.wrapping_add(1);
        let control = Arc::new(AtomicBool::new(false));
        self.as_mut().rust_mut().generation = generation;
        self.as_mut().rust_mut().control = control.clone();
        self.as_mut().set_error_message(QString::default());
        self.as_mut().set_busy(true);
        self.as_mut().set_operation_status(QString::from(message));
        let source = self.rust().source.clone();
        let thread = self.qt_thread();
        let spawn = std::thread::Builder::new()
            .name("ai-request".into())
            .spawn(move || {
                let result = work(&control);
                let _ = thread.queue(move |mut state| {
                    if state.rust().generation != generation {
                        return;
                    }
                    state.as_mut().set_busy(false);
                    if !state.rust().accepts_result(generation, &source) {
                        state.set_operation_status(QString::from(
                            "Cancelled; an already-sent request may still be billed.",
                        ));
                        return;
                    }
                    match result {
                        Ok(report) => {
                            if let Some(message) = report["connection"].as_str() {
                                state.set_operation_status(QString::from(message));
                            } else {
                                state
                                    .as_mut()
                                    .set_report_json(QString::from(report.to_string()));
                                if state.rust().turns.len() == 20 {
                                    state.as_mut().rust_mut().turns.remove(0);
                                }
                                state.as_mut().rust_mut().turns.push(report);
                                let history = serde_json::to_string(&state.rust().turns)
                                    .unwrap_or_else(|_| "[]".into());
                                state.as_mut().set_conversation_json(QString::from(history));
                                let message = if state.rust().is_demo() {
                                    "Demo response complete · no provider used"
                                } else {
                                    "Analysis complete · advisory commentary"
                                };
                                state.set_operation_status(QString::from(message));
                            }
                        }
                        Err(error) => {
                            state
                                .as_mut()
                                .set_operation_status(QString::from("AI operation failed."));
                            state.set_error_message(QString::from(error.message));
                        }
                    }
                });
            });
        if spawn.is_err() {
            self.as_mut().set_busy(false);
            self.set_error_message(QString::from("Could not start the AI request worker."));
        }
    }
}

impl AiBackendState {
    fn is_demo(&self) -> bool {
        self.forced_demo || self.demo_mode
    }
    fn accepts_result(&self, generation: u64, source: &str) -> bool {
        self.generation == generation
            && self.source == source
            && !self.control.load(Ordering::Relaxed)
    }
}

impl bridge::AiBackend {
    /// Starts an isolated PKCE callback worker; the UI opens only the returned HTTPS URL.
    /// # Returns
    /// Authorization URL with no tokens, or empty on failure, demo lock or busy state.
    pub fn sign_in_chatgpt(mut self: Pin<&mut Self>) -> QString {
        if self.rust().busy || self.rust().is_demo() {
            return QString::default();
        }
        self.as_mut().invalidate();
        let result = (|| {
            let base = if cfg!(target_os = "windows") {
                std::env::var_os("APPDATA")
            } else {
                std::env::var_os("HOME")
            }
            .ok_or_else(|| {
                viewer::Diagnostic::new(
                    "AiRegistration",
                    "Could not locate the user configuration directory.",
                )
            })?;
            let base = std::path::PathBuf::from(base);
            let directory = if cfg!(target_os = "windows") {
                base.join("RAVEN")
            } else if cfg!(target_os = "macos") {
                base.join("Library/Application Support/RAVEN")
            } else {
                base.join(".config/raven")
            };
            viewer::ai::chatgpt::SignIn::begin(&directory.join("chatgpt-registration.json"))
        })();
        let attempt = match result {
            Ok(attempt) => attempt,
            Err(error) => {
                self.set_error_message(QString::from(error.message));
                return QString::default();
            }
        };
        let url = attempt.authorization_url();
        let generation = self.rust().generation.wrapping_add(1);
        let control = Arc::new(AtomicBool::new(false));
        self.as_mut().rust_mut().generation = generation;
        self.as_mut().rust_mut().control = control.clone();
        self.as_mut().set_busy(true);
        self.as_mut().set_error_message(QString::default());
        self.as_mut()
            .set_operation_status(QString::from("Complete ChatGPT sign-in in your browser…"));
        let thread = self.qt_thread();
        if std::thread::Builder::new().name("chatgpt-sign-in".into()).spawn(move||{
            let result=attempt.complete(&control);
            let _=thread.queue(move|mut state|{
                if state.rust().generation!=generation{return;}
                state.as_mut().set_busy(false);
                if state.rust().control.load(Ordering::Relaxed) || state.rust().is_demo(){state.set_operation_status(QString::from("ChatGPT sign-in cancelled."));return;}
                match result {
                    Ok(connection)=>{
                        let catalog=state.as_mut().rust_mut().session.connect_chatgpt(connection);
                        state.as_mut().set_state_json(QString::from(catalog.to_string()));
                        state.set_operation_status(QString::from("Signed in to ChatGPT · choose a model. Inference has not been tested."));
                    },
                    Err(error)=>{state.as_mut().set_operation_status(QString::from("ChatGPT sign-in failed."));state.set_error_message(QString::from(error.message));},
                }
            });
        }).is_err(){self.as_mut().set_busy(false);self.set_error_message(QString::from("Could not start ChatGPT sign-in."));return QString::default();}
        QString::from(url)
    }
    /// Selects one authenticated model without network access or inference.
    /// # Arguments
    /// * `model` - Exact catalog slug supplied by the ChatGPT model picker.
    /// # Returns
    /// True on selection; false on unknown/expired/busy/demo state.
    pub fn select_chatgpt_model(mut self: Pin<&mut Self>, model: &QString) -> bool {
        if self.rust().busy || self.rust().is_demo() {
            return false;
        }
        self.as_mut().invalidate();
        let result = self
            .as_mut()
            .rust_mut()
            .session
            .select_chatgpt_model(&model.to_string());
        self.publish(result)
    }
    /// Clears local tokens immediately; remote revocation runs on the owned worker.
    pub fn sign_out_chatgpt(mut self: Pin<&mut Self>) {
        if self.rust().busy {
            return;
        }
        self.as_mut().invalidate();
        let connection = self.as_mut().rust_mut().session.disconnect_chatgpt();
        let catalog = self.rust().session.catalog();
        self.as_mut()
            .set_state_json(QString::from(catalog.to_string()));
        if let Some(connection) = connection {
            if self.rust().is_demo() {
                return;
            }
            self.start("Signing out of ChatGPT…",move|cancel|{
                let message=if connection.revoke(cancel).is_ok(){"Signed out of ChatGPT."}else{"Signed out locally. Remote revocation was not confirmed; disconnect RAVEN in ChatGPT Settings if needed."};
                Ok(serde_json::json!({"connection":message}))
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launch_demo_lock_cannot_be_bypassed_by_presentation_state() {
        let mut state = AiBackendState::default();
        state.demo_mode = false;
        state.forced_demo = true;
        assert!(state.is_demo());
        state.forced_demo = false;
        assert!(!state.is_demo());
        state.demo_mode = true;
        assert!(state.is_demo());
    }
    #[test]
    fn cancelled_replaced_and_late_queued_results_cannot_attach() {
        let mut state = AiBackendState::default();
        state.generation = 7;
        state.source = "run-3".into();
        assert!(state.accepts_result(7, "run-3"));
        assert!(!state.accepts_result(6, "run-3"));
        assert!(!state.accepts_result(7, "run-2"));
        state.control.store(true, Ordering::Relaxed);
        assert!(!state.accepts_result(7, "run-3"));
        state.control = Arc::new(AtomicBool::new(false));
        state.generation = 8;
        assert!(!state.accepts_result(7, "run-3"));
        assert!(state.accepts_result(8, "run-3"));
    }
}
