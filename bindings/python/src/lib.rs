/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: Apache-2.0
 */
//! Optional thin PyO3 boundary; all parsing and numeric semantics live in pnm.
use pnm::{
    diagnostic::{ErrorCode, ParseError},
    pnm::{
        analysis::cmts_upstream_rxmer::statistics,
        parser::{
            cmts_us_ofdma_rxmer::{CmtsUsOfdmaRxMer, HEADER_BYTES},
            parse_limits::ParseLimits,
        },
    },
};
use pyo3::{
    exceptions::{PyException, PyOSError, PyValueError},
    prelude::*,
    types::{PyBytes, PyModule},
};
use std::{fs::File, io::Read, path::PathBuf};
const MAX_FILE_BYTES: usize = HEADER_BYTES + 65_536;
pyo3::create_exception!(
    _native,
    ParseFailure,
    PyException,
    "Rust parser failure with code, field and byte offset."
);
fn failure(py: Python<'_>, error: ParseError) -> PyErr {
    let exception = ParseFailure::new_err(error.to_string());
    let value = exception.value(py);
    let _ = value.setattr("code", format!("{:?}", error.code));
    let _ = value.setattr("field", error.field);
    let _ = value.setattr("offset", error.offset);
    exception
}
#[pyclass(frozen)]
struct NativeCapture {
    bytes: Vec<u8>,
    json: String,
    summary: String,
}
#[pymethods]
impl NativeCapture {
    /// Validates borrowed Python bytes before retaining one owned Rust snapshot.
    /// # Arguments
    /// * `py`, `bytes` - Active interpreter and complete original binary bytes.
    /// # Returns
    /// Immutable native capture; no desktop or network connection is required.
    /// # Errors
    /// Structured parser errors or serialization errors.
    #[staticmethod]
    fn from_bytes(py: Python<'_>, bytes: &Bound<'_, PyBytes>) -> PyResult<Self> {
        Self::decode(py, bytes.as_bytes())
    }
    /// Loads one explicitly selected regular file with a bounded reader.
    /// # Arguments
    /// * `py`, `path` - Interpreter and user-supplied filesystem path.
    /// # Returns
    /// An owned immutable original/decoded snapshot.
    /// # Errors
    /// OSError for I/O; structured failures for invalid or oversized captures.
    #[staticmethod]
    fn from_file(py: Python<'_>, path: PathBuf) -> PyResult<Self> {
        let file = File::open(&path).map_err(|error| PyOSError::new_err(error.to_string()))?;
        if !file
            .metadata()
            .map_err(|error| PyOSError::new_err(error.to_string()))?
            .is_file()
        {
            return Err(PyOSError::new_err("Select a regular file"));
        }
        let mut bytes = Vec::new();
        file.take((MAX_FILE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| PyOSError::new_err(error.to_string()))?;
        Self::decode(py, &bytes)
    }
    /// Returns canonical Rust serialization without reparsing or recomputing data.
    /// # Returns
    /// JSON shared with Rust and the CLI, including null readings and provenance fields.
    fn to_json(&self) -> &str {
        &self.json
    }
    /// Returns descriptive Rust summary JSON, not a separately calculated Python result.
    /// # Returns
    /// Counts and nullable arithmetic dB statistics.
    fn statistics_json(&self) -> &str {
        &self.summary
    }
    /// Returns exact encoded sample bytes; one Python allocation is explicit.
    /// # Arguments
    /// * `py` - Active interpreter.
    /// # Returns
    /// Original sample octets, including0xff markers.
    fn raw_values<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.bytes[HEADER_BYTES..])
    }
    /// Returns the original unmodified capture, including padding.
    /// # Arguments
    /// * `py` - Active interpreter.
    /// # Returns
    /// A separate Python bytes object; native ownership remains intact.
    fn original_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.bytes)
    }
}
impl NativeCapture {
    fn decode(py: Python<'_>, bytes: &[u8]) -> PyResult<Self> {
        if bytes.len() > MAX_FILE_BYTES {
            return Err(failure(
                py,
                ParseError {
                    code: ErrorCode::LimitExceeded,
                    offset: 0,
                    field: "input_bytes",
                },
            ));
        }
        let capture = CmtsUsOfdmaRxMer::parse(bytes, ParseLimits::default())
            .map_err(|error| failure(py, error))?;
        let summary = statistics(&capture);
        let json = capture
            .to_json(32 * 1024 * 1024)
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
        // Summary serialization uses the same bounded core serializer.
        let summary = pnm::common::json::to_json(&summary, 4096)
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
        Ok(Self {
            bytes: bytes.to_vec(),
            json,
            summary,
        })
    }
}
/// Registers the optional decoding boundary without any runtime initialization I/O.
/// # Arguments
/// * `module` - Python's native extension module.
/// # Returns
/// Registration success.
/// # Errors
/// Propagates Python registration failures.
#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<NativeCapture>()?;
    module.add("ParseFailure", module.py().get_type::<ParseFailure>())?;
    Ok(())
}
