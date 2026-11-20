/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Generation-guarded channel-estimation collection; no DSP in QML.
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
        #[qproperty(QString, catalog_json)]
        #[qproperty(QString, view_json)]
        #[qproperty(QString, error_message)]
        #[qproperty(bool, busy)]
        type ChannelEstimationCollectionBackend = super::ChannelEstimationState;
        /// Starts bounded local file/folder/archive import with stale-result rejection.
        #[qinvokable]
        fn import_collection(self: Pin<&mut ChannelEstimationCollectionBackend>, url: &QUrl);
        /// Cancels imports/selection and preserves the previous usable session.
        #[qinvokable]
        fn cancel(self: Pin<&mut ChannelEstimationCollectionBackend>);
        /// Averages one exact device/channel/grid group on a worker.
        #[qinvokable]
        fn select_group(self: Pin<&mut ChannelEstimationCollectionBackend>, group: u32);
        /// Displays one capture in chronological order, preserving frequency zoom.
        #[qinvokable]
        fn show_frame(self: Pin<&mut ChannelEstimationCollectionBackend>, group: u32, frame: u32);
        /// Changes the displayed metric while preserving the selected frequency range.
        #[qinvokable]
        fn select_metric(self: Pin<&mut ChannelEstimationCollectionBackend>, delay: bool);
        /// Applies a source-carrier zoom range, with full-range reset supported.
        #[qinvokable]
        fn show_range(self: Pin<&mut ChannelEstimationCollectionBackend>, start: u32, count: u32);
        /// Writes full-resolution derived analysis to a new user-chosen file.
        #[qinvokable]
        fn export_json(self: Pin<&mut ChannelEstimationCollectionBackend>, url: &QUrl);
        /// Creates a local report and prepared question without contacting a provider.
        #[qinvokable]
        fn engineering_report(self: Pin<&mut ChannelEstimationCollectionBackend>) -> QString;
    }
    impl cxx_qt::Threading for ChannelEstimationCollectionBackend {}
}
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QUrl};
use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use viewer::measurements::channel_estimation::collection::ChannelEstimateAggregate;
use viewer::{
    capture::{collection::write_json_new, Diagnostic},
    measurements::channel_estimation::collection::{self, Collection, ImportLimits},
};
/// Independent collection ownership; only bounded serialized plots cross to Qt.
pub struct ChannelEstimationState {
    catalog_json: QString,
    view_json: QString,
    error_message: QString,
    busy: bool,
    generation: u64,
    cancel: Arc<AtomicBool>,
    collection: Option<Arc<Collection>>,
    result: Option<Arc<ChannelEstimateAggregate>>,
    delay: bool,
    frame: Option<(usize, usize)>,
    playback_magnitude_bounds: Option<(usize, (f64, f64))>,
    start: usize,
    count: usize,
}
impl Default for ChannelEstimationState {
    /// Creates empty state without I/O, worker threads or network operations.
    fn default() -> Self {
        Self {
            catalog_json: "{}".into(),
            view_json: "{}".into(),
            error_message: QString::default(),
            busy: false,
            generation: 0,
            cancel: Arc::new(AtomicBool::new(false)),
            collection: None,
            result: None,
            delay: false,
            frame: None,
            playback_magnitude_bounds: None,
            start: 0,
            count: 0,
        }
    }
}
impl Drop for ChannelEstimationState {
    /// Stops archive traversal when the QObject is destroyed.
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
impl bridge::ChannelEstimationCollectionBackend {
    /// Invalidates callbacks from superseded workers and releases busy state.
    pub fn cancel(mut self: Pin<&mut Self>) {
        self.as_mut()
            .rust_mut()
            .cancel
            .store(true, Ordering::Relaxed);
        let generation = self.rust().generation.wrapping_add(1);
        self.as_mut().rust_mut().generation = generation;
        self.set_busy(false);
    }
    /// Starts safe import, retaining the previous session on failure.
    /// # Arguments
    /// * `url` - Native user-selected local capture, directory or archive.
    pub fn import_collection(mut self: Pin<&mut Self>, url: &QUrl) {
        let path = match crate::backend::local_path(url) {
            Ok(path) => path,
            Err(error) => {
                self.set_error_message(error.message.into());
                return;
            }
        };
        self.as_mut().cancel();
        self.as_mut().rust_mut().cancel = Arc::new(AtomicBool::new(false));
        let cancel = self.rust().cancel.clone();
        let generation = self.rust().generation;
        let thread = self.qt_thread();
        self.as_mut().set_busy(true);
        self.as_mut().set_error_message(QString::default());
        let spawn = std::thread::Builder::new()
            .name("channel-estimation-import".into())
            .spawn(move || {
                let result = Collection::scan(&path, ImportLimits::default(), &cancel).and_then(
                    |collection| {
                        serde_json::to_string(&collection.catalog)
                            .map(|json| (Arc::new(collection), json))
                            .map_err(|error| Diagnostic::new("Json", error))
                    },
                );
                let _ = thread.queue(move |mut backend| {
                    if backend.rust().generation != generation {
                        return;
                    }
                    backend.as_mut().set_busy(false);
                    match result {
                        Ok((collection, json)) => {
                            backend.as_mut().rust_mut().collection = Some(collection);
                            backend.as_mut().rust_mut().result = None;
                            backend.as_mut().rust_mut().frame = None;
                            backend.as_mut().rust_mut().playback_magnitude_bounds = None;
                            backend.as_mut().set_view_json("{}".into());
                            backend.set_catalog_json(json.into());
                        }
                        Err(error) => backend.set_error_message(error.message.into()),
                    }
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.set_error_message(error.to_string().into());
        }
    }
    /// Calculates one compatible group's complex average and delay off the UI thread.
    /// # Arguments
    /// * `group` - Exact catalog group ID from this collection.
    pub fn select_group(mut self: Pin<&mut Self>, group: u32) {
        let Some(collection) = self.rust().collection.clone() else {
            return;
        };
        self.as_mut().cancel();
        let generation = self.rust().generation;
        let thread = self.qt_thread();
        let delay = self.rust().delay;
        self.as_mut().set_busy(true);
        self.as_mut().set_error_message(QString::default());
        let spawn = std::thread::Builder::new()
            .name("channel-estimation-average".into())
            .spawn(move || {
                let result = collection.aggregate(group as usize).and_then(|result| {
                    collection::viewport(&result, delay, 0, result.frequencies_hz.len())
                        .map(|view| (Arc::new(result), view.to_string()))
                });
                let _ = thread.queue(move |mut backend| {
                    if backend.rust().generation != generation {
                        return;
                    }
                    backend.as_mut().set_busy(false);
                    match result {
                        Ok((result, json)) => {
                            backend.as_mut().rust_mut().frame = None;
                            backend.as_mut().rust_mut().count = result.frequencies_hz.len();
                            backend.as_mut().rust_mut().start = 0;
                            backend.as_mut().rust_mut().result = Some(result);
                            backend.set_view_json(json.into());
                        }
                        Err(error) => backend.set_error_message(error.message.into()),
                    }
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.set_error_message(error.to_string().into());
        }
    }
    /// Displays a bounded chronological frame using the shared Rust analysis.
    /// # Arguments
    /// * `group`, `frame` - Current catalog group and zero-based capture index.
    pub fn show_frame(mut self: Pin<&mut Self>, group: u32, frame: u32) {
        if *self.busy() {
            return;
        }
        let Some(collection) = self.rust().collection.clone() else {
            return;
        };
        if self
            .rust()
            .playback_magnitude_bounds
            .is_none_or(|(id, _)| id != group as usize)
        {
            match collection.playback_magnitude_bounds(group as usize) {
                Ok(bounds) => {
                    self.as_mut().rust_mut().playback_magnitude_bounds =
                        Some((group as usize, bounds));
                }
                Err(error) => {
                    self.set_error_message(error.message.into());
                    return;
                }
            }
        }
        match collection.frame(group as usize, frame as usize) {
            Ok(result) => {
                let same_grid = self
                    .rust()
                    .result
                    .as_ref()
                    .is_some_and(|previous| previous.group == result.group);
                let count = if same_grid {
                    self.rust().count
                } else {
                    result.frequencies_hz.len()
                };
                let start = if same_grid { self.rust().start } else { 0 };
                self.as_mut().rust_mut().frame = Some((group as usize, frame as usize));
                self.as_mut().rust_mut().result = Some(Arc::new(result));
                self.show_range(start as u32, count as u32);
            }
            Err(error) => self.set_error_message(error.message.into()),
        }
    }
    /// Switches magnitude/delay using existing full-resolution results.
    /// # Arguments
    /// * `delay` - True for microseconds; false for magnitude in dB.
    pub fn select_metric(mut self: Pin<&mut Self>, delay: bool) {
        if *self.busy() {
            return;
        }
        self.as_mut().rust_mut().delay = delay;
        let start = self.rust().start as u32;
        let count = self.rust().count as u32;
        self.show_range(start, count);
    }
    /// Publishes a validated bounded source range without changing exported results.
    /// # Arguments
    /// * `start`, `count` - Original carrier range from the shared zoom control.
    pub fn show_range(mut self: Pin<&mut Self>, start: u32, count: u32) {
        if *self.busy() {
            return;
        }
        let Some(result) = self.rust().result.clone() else {
            return;
        };
        match collection::viewport(&result, self.rust().delay, start as usize, count as usize) {
            Ok(mut view) => {
                if let Some((group, frame)) = self.rust().frame {
                    if let Some(catalog) = self
                        .rust()
                        .collection
                        .as_ref()
                        .and_then(|collection| collection.catalog.groups.get(group))
                    {
                        view["frame_index"] = serde_json::json!(frame);
                        view["frame_count"] = serde_json::json!(catalog.capture_count);
                        view["capture_time"] =
                            serde_json::json!(catalog.sources[frame].capture_time);
                        view["average_method"] = serde_json::json!("single_capture");
                        if !self.rust().delay {
                            if let Some((id, (minimum, maximum))) =
                                self.rust().playback_magnitude_bounds
                            {
                                if id == group {
                                    view["minimum"] = serde_json::json!(minimum);
                                    view["maximum"] = serde_json::json!(maximum);
                                    view["magnitude_axis"] =
                                        serde_json::json!("locked_full_group_aggregate");
                                }
                            }
                        }
                        view.as_object_mut()
                            .unwrap()
                            .remove("channel_estimation_statistics");
                    }
                }
                self.as_mut().rust_mut().start = start as usize;
                self.as_mut().rust_mut().count = count as usize;
                self.as_mut().set_error_message(QString::default());
                self.set_view_json(view.to_string().into());
            }
            Err(error) => self.set_error_message(error.message.into()),
        }
    }
    /// Exports all carriers and provenance; refuses to overwrite an existing file.
    /// # Arguments
    /// * `url` - Native user-selected new JSON destination.
    pub fn export_json(mut self: Pin<&mut Self>, url: &QUrl) {
        let Some(result) = self.rust().result.clone() else {
            return;
        };
        let path = match crate::backend::local_path(url) {
            Ok(path) => path,
            Err(error) => {
                self.set_error_message(error.message.into());
                return;
            }
        };
        self.as_mut().cancel();
        let generation = self.rust().generation;
        let thread = self.qt_thread();
        self.as_mut().set_busy(true);
        self.as_mut().set_error_message(QString::default());
        let spawn = std::thread::Builder::new()
            .name("channel-estimation-export".into())
            .spawn(move || {
                let outcome = write_json_new(&*result, &path);
                let _ = thread.queue(move |mut backend| {
                    if backend.rust().generation != generation {
                        return;
                    }
                    backend.as_mut().set_busy(false);
                    if let Err(error) = outcome {
                        backend.set_error_message(error.message.into());
                    }
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.set_error_message(error.to_string().into());
        }
    }
    /// Creates a generation-bound local report and a prepared analysis question.
    /// # Returns
    /// Trusted local results, never a live provider request or new calculation.
    pub fn engineering_report(self: Pin<&mut Self>) -> QString {
        let mut snapshot =
            serde_json::from_str::<serde_json::Value>(&self.rust().view_json.to_string())
                .unwrap_or(serde_json::Value::Null);
        // Reports summarize scalar provenance; decimated trace arrays are not
        // independent measurements and must not crowd out the capture metadata.
        if let Some(fields) = snapshot.as_object_mut() {
            fields.remove("points");
            fields.remove("channel_estimation_statistics");
            if let Some(minimum) = fields.remove("minimum") {
                fields.insert("plot_axis_minimum".into(), minimum);
            }
            if let Some(maximum) = fields.remove("maximum") {
                fields.insert("plot_axis_maximum".into(), maximum);
            }
        }
        let report = if self.rust().result.is_none() {
            Err(Diagnostic::new(
                "ChannelEstimationReport",
                "Select a channel-estimation group first.",
            ))
        } else {
            viewer::reports::adapter::from_snapshot(
                "Multi Channel Estimation",
                "channel-estimation-collection",
                self.rust().generation,
                &snapshot,
                viewer::reports::context::Classification::Derived,
            )
        };
        let result = report.map(|mut report| {
            report["prepared_question"] = serde_json::json!("Explain the selected device/channel's complex-averaged channel estimate and its group delay. Identify single-capture playback versus temporal aggregation from average_method. Distinguish temporal magnitude extrema from the magnitude and phase-derived delay of mean I/Q. Preserve dB, microsecond display and second export units, missing-data gaps and capture counts. This local summary contains provenance and display bounds, not full trace arrays; do not treat axis bounds as measured extrema, infer PLC locations or infer a physical cause without evidence.");
            report["prepared_question_version"] = serde_json::json!("1.1");
            report
        }).unwrap_or_else(|error| serde_json::json!({"error":error.message}));
        result.to_string().into()
    }
}
