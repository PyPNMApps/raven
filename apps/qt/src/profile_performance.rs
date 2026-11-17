/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Worker-owned OFDM preparation and simulation; no numerical work on Qt's thread.
#[cxx_qt::bridge]
mod bridge {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qurl.h");
        type QUrl = cxx_qt_lib::QUrl;
    }
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, inputs_json)]
        #[qproperty(QString, chart_json)]
        #[qproperty(QString, result_json)]
        #[qproperty(QString, evidence_json)]
        #[qproperty(QString, source_id)]
        #[qproperty(QString, error_message)]
        #[qproperty(bool, busy)]
        #[qproperty(bool, selection_available)]
        #[qproperty(bool, prepared)]
        #[qproperty(bool, running)]
        type ProfilePerformanceBackend = super::State;
        /// Reads a bounded binary capture into its explicitly chosen input role.
        #[qinvokable]
        fn choose_file(self: Pin<&mut ProfilePerformanceBackend>, role: &QString, url: &QUrl);
        /// Identifies a bounded multi-file selection and validates compatibility.
        #[qinvokable]
        fn choose_files(self: Pin<&mut ProfilePerformanceBackend>, urls: &QString, tolerance: u32);
        /// Validates captured identity/grid and explicit settings on a worker.
        #[qinvokable]
        fn prepare(
            self: Pin<&mut ProfilePerformanceBackend>,
            profile: u32,
            mce: bool,
            tolerance: u32,
        );
        /// Preflights several captured IDs selected with checkboxes.
        #[qinvokable]
        fn prepare_many(
            self: Pin<&mut ProfilePerformanceBackend>,
            profiles: &QString,
            mce: bool,
            tolerance: u32,
        );
        /// Preflights profiles with an optional per-run channel bypass.
        #[qinvokable]
        fn prepare_selection(
            self: Pin<&mut ProfilePerformanceBackend>,
            profiles: &QString,
            mce: bool,
            ignore_channel: bool,
            tolerance: u32,
        );
        /// Selects the prepared cases to display and run; empty disables Run.
        #[qinvokable]
        fn select_profiles(self: Pin<&mut ProfilePerformanceBackend>, profiles: &QString);
        /// Starts actual decoder execution using the last successful preparation.
        #[qinvokable]
        fn run(self: Pin<&mut ProfilePerformanceBackend>);
        /// Requests cooperative cancellation; running remains true until acknowledged.
        #[qinvokable]
        fn cancel(self: Pin<&mut ProfilePerformanceBackend>);
        /// Requests a bounded source-index plot range without rerunning FEC.
        #[qinvokable]
        fn show_range(self: Pin<&mut ProfilePerformanceBackend>, start: u32, count: u32);
    }
    impl cxx_qt::Threading for ProfilePerformanceBackend {}
}
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QUrl};
use std::pin::Pin;
use viewer::simulation::fec::{batch::BatchSession, engine, workspace::Inputs};

/// Bound the tiny checkbox-selection message before JSON allocation.
const MAX_PROFILE_SELECTION_BYTES: usize = 128;

/// Bound the native multi-file URL message before deserialization.
const MAX_CAPTURE_SELECTION_BYTES: usize = 16_384;

/// Independent native session, with one active worker and stale-callback guards.
pub struct State {
    inputs_json: QString,
    chart_json: QString,
    result_json: QString,
    evidence_json: QString,
    source_id: QString,
    error_message: QString,
    busy: bool,
    prepared: bool,
    selection_available: bool,
    running: bool,
    inputs: Inputs,
    session: Option<BatchSession>,
    control: engine::RunControl,
    generation: u64,
}
impl Default for State {
    /// Creates empty state without file access or simulation work.
    fn default() -> Self {
        Self {
            inputs_json: QString::from("{}"),
            chart_json: QString::from("{}"),
            result_json: QString::from("{}"),
            evidence_json: QString::from("{}"),
            source_id: QString::from("0"),
            error_message: QString::default(),
            busy: false,
            prepared: false,
            selection_available: false,
            running: false,
            inputs: Inputs::default(),
            session: None,
            control: engine::RunControl::new(),
            generation: 0,
        }
    }
}
impl Drop for State {
    /// Stops owned computation when Qt destroys this session.
    fn drop(&mut self) {
        self.control.cancel();
    }
}
impl bridge::ProfilePerformanceBackend {
    fn begin(mut self: Pin<&mut Self>) -> u64 {
        let generation = self.rust().generation.wrapping_add(1);
        self.as_mut().rust_mut().generation = generation;
        self.as_mut()
            .set_source_id(QString::from(generation.to_string()));
        self.as_mut().set_evidence_json(QString::from("{}"));
        self.as_mut().set_error_message(QString::default());
        self.set_busy(true);
        generation
    }
    fn fail(mut self: Pin<&mut Self>, message: impl ToString) {
        self.as_mut()
            .set_error_message(QString::from(message.to_string()));
        self.as_mut().set_running(false);
        self.set_busy(false);
    }
    /// Reads the native selected file on a worker and preserves prior inputs on error.
    /// # Arguments
    /// * `role` - profile, rxmer or channel; wrong types are rejected.
    /// * `url` - Native local URL from the operating-system file picker.
    pub fn choose_file(mut self: Pin<&mut Self>, role: &QString, url: &QUrl) {
        if self.rust().busy {
            return;
        }
        let path = match crate::backend::local_path(url) {
            Ok(path) => path,
            Err(error) => {
                self.as_mut().fail(error.message);
                return;
            }
        };
        let role = role.to_string();
        self.update_inputs(move |inputs| inputs.select(&role, &path));
    }
    /// Imports two or three local URLs in arbitrary order on the owned worker.
    /// # Arguments
    /// * `urls` - JSON array of native file URLs; bounded to 16 KiB and three files.
    /// * `tolerance` - Maximum capture association separation in seconds.
    /// # Errors
    /// Publishes diagnostics and preserves inputs/chart on invalid selections.
    pub fn choose_files(self: Pin<&mut Self>, urls: &QString, tolerance: u32) {
        if self.rust().busy {
            return;
        }
        let urls = urls.to_string();
        let paths = (|| {
            if urls.len() > MAX_CAPTURE_SELECTION_BYTES {
                return Err("Capture selection is too large.".to_string());
            }
            let urls: Vec<String> =
                serde_json::from_str(&urls).map_err(|error| error.to_string())?;
            if !(2..=3).contains(&urls.len()) {
                return Err("Select 2 or 3 capture files.".to_string());
            }
            urls.iter()
                .map(|url| {
                    crate::backend::local_path(&QUrl::from(url.as_str()))
                        .map_err(|error| error.message)
                })
                .collect::<Result<Vec<_>, _>>()
        })();
        match paths {
            Ok(paths) => self.update_inputs(move |inputs| inputs.select_many(&paths, tolerance)),
            Err(error) => self.fail(error),
        }
    }
    fn update_inputs(
        mut self: Pin<&mut Self>,
        load: impl FnOnce(&mut Inputs) -> Result<serde_json::Value, viewer::Diagnostic> + Send + 'static,
    ) {
        let mut inputs = self.rust().inputs.clone();
        let generation = self.as_mut().begin();
        let thread = self.qt_thread();
        let spawn = std::thread::Builder::new()
            .name("ofdm-input".into())
            .spawn(move || {
                let result = load(&mut inputs);
                let _ = thread.queue(move |mut state| {
                    if state.rust().generation != generation {
                        return;
                    }
                    state.as_mut().set_busy(false);
                    match result {
                        Ok(catalog) => {
                            state.as_mut().rust_mut().inputs = inputs;
                            state
                                .as_mut()
                                .set_inputs_json(QString::from(catalog.to_string()));
                            state.as_mut().rust_mut().session = None;
                            state.as_mut().set_selection_available(false);
                            state.set_prepared(false);
                        }
                        Err(error) => state.fail(error.message),
                    }
                });
            });
        if let Err(error) = spawn {
            self.fail(error);
        }
    }
    /// Preflights a new session on a worker; failed preparation retains the prior chart.
    /// # Arguments
    /// * `profile` - Captured profile ID, not a positional index.
    /// * `mce` - Request received-pilot correction instead of supplied-H correction.
    /// * `tolerance` - Maximum source/profile epoch separation in seconds.
    pub fn prepare(mut self: Pin<&mut Self>, profile: u32, mce: bool, tolerance: u32) {
        if self.rust().busy {
            return;
        }
        self.as_mut()
            .prepare_many(&QString::from(format!("[{profile}]")), mce, tolerance);
    }
    /// Preflights the checkbox-selected profile IDs on a worker.
    /// # Arguments
    /// * `profiles` - JSON array of captured IDs, bounded and validated in Rust.
    /// * `mce`, `tolerance` - Shared receiver and source-association settings.
    pub fn prepare_many(self: Pin<&mut Self>, profiles: &QString, mce: bool, tolerance: u32) {
        self.prepare_selection(profiles, mce, false, tolerance);
    }
    /// Preflights chosen profiles, retaining any bypassed channel in the selection.
    /// # Arguments
    /// * `profiles` - Bounded JSON array of captured profile IDs.
    /// * `mce` - Request pilot-derived correction; requires an active channel.
    /// * `ignore_channel` - Use unity for this run without detaching the capture.
    /// * `tolerance` - Maximum source association separation in seconds.
    pub fn prepare_selection(
        mut self: Pin<&mut Self>,
        profiles: &QString,
        mce: bool,
        ignore_channel: bool,
        tolerance: u32,
    ) {
        if self.rust().busy {
            return;
        }
        let selection = profiles.to_string();
        if selection.len() > MAX_PROFILE_SELECTION_BYTES {
            self.fail("Too many profile IDs.");
            return;
        }
        let profiles = match serde_json::from_str::<Vec<u8>>(&selection) {
            Ok(profiles) => profiles,
            Err(_) => {
                self.fail("Select valid profile IDs.");
                return;
            }
        };
        let inputs = self.rust().inputs.clone();
        let generation = self.as_mut().begin();
        let thread = self.qt_thread();
        let spawn = std::thread::Builder::new()
            .name("ofdm-prepare".into())
            .spawn(move || {
                let result = BatchSession::prepare_selection(
                    inputs,
                    profiles,
                    mce,
                    tolerance,
                    ignore_channel,
                )
                .and_then(|session| session.chart(0, 0).map(|chart| (session, chart)));
                let _ = thread.queue(move |mut state| {
                    if state.rust().generation != generation {
                        return;
                    }
                    match result {
                        Ok((session, chart)) => {
                            state.as_mut().rust_mut().session = Some(session);
                            state
                                .as_mut()
                                .set_chart_json(QString::from(chart.to_string()));
                            state.as_mut().set_result_json(QString::from("{}"));
                            state.as_mut().set_selection_available(true);
                            state.as_mut().set_prepared(true);
                            state.set_busy(false);
                        }
                        Err(error) => state.fail(error.message),
                    }
                });
            });
        if let Err(error) = spawn {
            self.fail(error);
        }
    }
    /// Changes the prepared run/overlay subset and invalidates stale results.
    /// # Arguments
    /// * `profiles` - JSON array of unique prepared IDs; empty disables Run.
    pub fn select_profiles(mut self: Pin<&mut Self>, profiles: &QString) {
        if self.rust().busy {
            return;
        }
        let selection = profiles.to_string();
        if selection.len() > MAX_PROFILE_SELECTION_BYTES {
            self.fail("Too many profile IDs.");
            return;
        }
        let profiles = match serde_json::from_str::<Vec<u8>>(&selection) {
            Ok(profiles) => profiles,
            Err(_) => {
                self.fail("Select valid profile IDs.");
                return;
            }
        };
        let result = self.as_mut().rust_mut().session.as_mut().map(|session| {
            session
                .select(profiles)
                .and_then(|ready| session.chart(0, 0).map(|chart| (ready, chart)))
        });
        if let Some(result) = result {
            match result {
                Ok((ready, chart)) => {
                    self.as_mut()
                        .set_chart_json(QString::from(chart.to_string()));
                    let generation = self.rust().generation.wrapping_add(1);
                    self.as_mut().rust_mut().generation = generation;
                    self.as_mut()
                        .set_source_id(QString::from(generation.to_string()));
                    self.as_mut().set_evidence_json(QString::from("{}"));
                    self.as_mut().set_result_json(QString::from("{}"));
                    self.as_mut().set_prepared(ready);
                    self.set_error_message(QString::default());
                }
                Err(error) => self.fail(error.message),
            }
        }
    }
    /// Runs real MERLIN FEC on a worker and returns compact scoped counters.
    ///
    /// Does nothing while busy or before successful preparation; never blocks Qt.
    pub fn run(mut self: Pin<&mut Self>) {
        if self.rust().busy || !self.rust().prepared {
            return;
        }
        let Some(mut session) = self.as_mut().rust_mut().session.take() else {
            return;
        };
        let control = engine::RunControl::new();
        self.as_mut().rust_mut().control = control.clone();
        let generation = self.as_mut().begin();
        self.as_mut().set_result_json(QString::from("{}"));
        self.as_mut().set_running(true);
        let thread = self.qt_thread();
        let spawn = std::thread::Builder::new()
            .name("ofdm-fec".into())
            .spawn(move || {
                let result = session.run(&control);
                let _ = thread.queue(move |mut state| {
                    if state.rust().generation != generation {
                        return;
                    }
                    state.as_mut().rust_mut().session = Some(session);
                    state.as_mut().set_running(false);
                    state.as_mut().set_busy(false);
                    match result {
                        Ok(report) => {
                            state.as_mut().set_evidence_json(QString::from(
                                report["ai_evidence"].to_string(),
                            ));
                            state.set_result_json(QString::from(report.to_string()));
                        }
                        Err(error) => state.fail(error.message),
                    }
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_selection_available(false);
            self.as_mut().set_prepared(false);
            self.fail(error);
        }
    }
    /// Requests cancellation without reporting a running worker as already stopped.
    pub fn cancel(self: Pin<&mut Self>) {
        self.rust().control.cancel();
    }
    /// Computes only a bounded chart viewport; source points remain in Rust.
    /// # Arguments
    /// * `start`, `count` - Original sample range, zero count restores full capture.
    pub fn show_range(self: Pin<&mut Self>, start: u32, count: u32) {
        if self.rust().busy {
            return;
        }
        let result = self
            .rust()
            .session
            .as_ref()
            .map(|session| session.chart(start as usize, count as usize));
        if let Some(result) = result {
            match result {
                Ok(chart) => self.set_chart_json(QString::from(chart.to_string())),
                Err(error) => self.fail(error.message),
            }
        }
    }
}
