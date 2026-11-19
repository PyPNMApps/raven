/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Independent offline report gallery. No provider, credential or network dependency.
#[cxx_qt::bridge]
mod bridge {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        include!("cxx-qt-lib/qurl.h");
        type QString = cxx_qt_lib::QString;
        type QUrl = cxx_qt_lib::QUrl;
    }
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, model_json)]
        #[qproperty(QString, error_message)]
        #[qproperty(QString, fixture_catalog)]
        type ReportBackend = super::ReportState;
        /// Loads one finite offline example through the normal report validator.
        #[qinvokable]
        fn load_fixture(self: Pin<&mut ReportBackend>, id: &QString);
        /// Disposes the current report when its owner/view closes or is replaced.
        #[qinvokable]
        fn clear(self: Pin<&mut ReportBackend>);
        /// Writes the approved example presentation to a newly chosen local file.
        #[qinvokable]
        fn export_report(self: Pin<&mut ReportBackend>, url: &QUrl);
    }
}
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QUrl};
use std::{io::Write, pin::Pin};
pub struct ReportState {
    model_json: QString,
    error_message: QString,
    fixture_catalog: QString,
}
impl Default for ReportState {
    /// Creates a gallery catalog without network, credential or filesystem access.
    ///
    /// # Returns
    /// Empty presentation/diagnostics and a finite bundled-example catalog.
    fn default() -> Self {
        Self {
            model_json: QString::from("{}"),
            error_message: QString::default(),
            fixture_catalog: QString::from(viewer::reports::fixtures::catalog().to_string()),
        }
    }
}
impl bridge::ReportBackend {
    /// Replaces the current report; invalid definitions never retain the old model.
    /// # Arguments
    /// * `id` - One finite bundled example ID.
    pub fn load_fixture(mut self: Pin<&mut Self>, id: &QString) {
        self.as_mut().clear();
        match viewer::reports::fixtures::render(&id.to_string()) {
            Ok(model) => self.set_model_json(QString::from(model.to_string())),
            Err(error) => self.set_error_message(QString::from(error.message)),
        }
    }
    /// Clears presentation and diagnostics without any external side effects.
    ///
    /// Updates both Qt properties so a rejected or closed report cannot retain content.
    pub fn clear(mut self: Pin<&mut Self>) {
        self.as_mut().set_model_json(QString::from("{}"));
        self.set_error_message(QString::default());
    }
    /// Exports only an accepted fixture presentation, never overwriting a destination.
    /// # Arguments
    /// * `url` - User-selected native Save destination, not a report-authored path.
    ///
    /// Failures populate the diagnostic property without overwriting an existing file.
    pub fn export_report(self: Pin<&mut Self>, url: &QUrl) {
        let result = (|| {
            if self.rust().model_json.to_string() == "{}" {
                return Err("Load a valid example before exporting.");
            }
            let path =
                crate::backend::local_path(url).map_err(|_| "Choose a local output file.")?;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(|_| {
                    "Could not create a new report file; choose a different destination."
                })?;
            file.write_all(self.rust().model_json.to_string().as_bytes())
                .map_err(|_| "Could not write report presentation.")
        })();
        if let Err(error) = result {
            self.set_error_message(QString::from(error));
        }
    }
}
