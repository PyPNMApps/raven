/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: Apache-2.0
 */
//! CCAP upstream RxMER: 3.1 I18 Table410 / 4.0 I02 Table411, major2 only.
//! See docs/reference/cmts-upstream-rxmer/format.md for evidence and limits.
use super::{
    parse_limits::ParseLimits,
    pnm_file_type::PnmDecoderKind,
    pnm_header::{HasPnmHeader, PnmHeader},
};
use crate::{
    common::reader::Reader,
    diagnostic::{ErrorCode, ParseError},
    pnm::data_type::{FrequencyHz, MacAddress, RxMerDb, SubcarrierIdx},
};
use serde::{ser::SerializeStruct, Serialize, Serializer};

/// Ordered header field widths from Table410; no downstream header reuse.
pub const HEADER_BYTES: usize = 4 + 1 + 1 + 4 + 4 + 256 + 6 + 1 + 6 + 2 + 1 + 4 + 2 + 1 + 4;
const CCAP_ID_OFFSET: usize = 14;
const FREQUENCY_OFFSET: usize = 286;
const FIRST_INDEX_OFFSET: usize = 290;
const SPACING_OFFSET: usize = 292;
const LENGTH_OFFSET: usize = 293;

/// What one octet establishes; excluded and unmeasured are not distinguishable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SampleStatus {
    Valid,
    Unavailable,
}
/// One physical carrier, preserving its original position and raw representation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RxMerSample {
    pub subcarrier_index: SubcarrierIdx,
    pub frequency_hz: FrequencyHz,
    pub raw: u8,
    pub rxmer_db: Option<RxMerDb>,
    pub valid: bool,
    pub status: SampleStatus,
}
/// Borrowed upstream identities and grid, without narrowing interface indices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmtsRxMerMetadata<'capture> {
    pub if_index: u32,
    pub ccap_id: &'capture str,
    pub ccap_id_raw: &'capture [u8],
    pub rpd_id: MacAddress,
    pub rpd_port: u8,
    pub cm_mac_address: MacAddress,
    pub number_of_averages: u16,
    /// Original wire octet; no unverified Boolean encoding is imposed.
    pub pre_eq_raw: u8,
    pub subcarrier_zero_frequency_hz: FrequencyHz,
    pub first_active_subcarrier_index: SubcarrierIdx,
    pub subcarrier_spacing_khz: u8,
}
/// Immutable borrowed capture; identities and raw padding remain lossless.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmtsUsOfdmaRxMer<'input> {
    header: PnmHeader<'input>,
    if_index: u32,
    ccap_id_raw: &'input [u8],
    ccap_id: &'input str,
    rpd_id: MacAddress,
    rpd_port: u8,
    cm_mac_address: MacAddress,
    number_of_averages: u16,
    pre_eq_raw: u8,
    zero_frequency_hz: FrequencyHz,
    first_index: SubcarrierIdx,
    spacing_khz: u8,
    raw_values: &'input [u8],
}
impl HasPnmHeader for CmtsUsOfdmaRxMer<'_> {
    /// Borrows the existing common header without copying capture bytes.
    /// # Returns
    /// Metadata whose format version is separate from DOCSIS generation.
    fn pnm_header(&self) -> &PnmHeader<'_> {
        &self.header
    }
}
impl<'input> CmtsUsOfdmaRxMer<'input> {
    /// Borrows typed metadata while preserving every identity and raw CCAP padding.
    /// # Returns
    /// A small immutable view; sample data is available separately through iterators.
    pub fn metadata(&self) -> CmtsRxMerMetadata<'_> {
        CmtsRxMerMetadata {
            if_index: self.if_index,
            ccap_id: self.ccap_id,
            ccap_id_raw: self.ccap_id_raw,
            rpd_id: self.rpd_id,
            rpd_port: self.rpd_port,
            cm_mac_address: self.cm_mac_address,
            number_of_averages: self.number_of_averages,
            pre_eq_raw: self.pre_eq_raw,
            subcarrier_zero_frequency_hz: self.zero_frequency_hz,
            first_active_subcarrier_index: self.first_index,
            subcarrier_spacing_khz: self.spacing_khz,
        }
    }
    /// Decodes the complete major2 file, borrowing raw samples and CCAP bytes.
    /// # Arguments
    /// * `bytes` - Original file, retained for the capture lifetime.
    /// * `limits` - Maximum accepted sample count, not a protocol maximum.
    /// # Returns
    /// Immutable validated capture; no sample-vector allocation or I/O.
    /// # Errors
    /// Structured type/version, truncation, encoding, field, length and overflow errors.
    pub fn parse(bytes: &'input [u8], limits: ParseLimits) -> Result<Self, ParseError> {
        Self::from_header(PnmHeader::from_bytes(bytes)?, limits)
    }
    /// Decodes upstream fields from the single existing common-header owner.
    /// # Arguments
    /// * `header` - Already recognized upstream major2 header.
    /// * `limits` - Caller-selected sample budget.
    /// # Returns
    /// A capture borrowing the original field and payload bytes.
    /// # Errors
    /// Rejects another decoder, bad strings/grids, lengths and arithmetic overflow.
    #[tracing::instrument(level = "debug", skip_all, err)]
    pub fn from_header(header: PnmHeader<'input>, limits: ParseLimits) -> Result<Self, ParseError> {
        if header.descriptor().decoder != PnmDecoderKind::CmtsUpstreamRxMer {
            return Err(error(ErrorCode::UnsupportedType, 0, "file_type"));
        }
        let mut reader = Reader::with_origin(header.pnm_data(), header.payload_offset())?;
        let if_index = reader.read_u32_be("if_index")?;
        let ccap_start = reader.position();
        reader.read_bytes::<256>("ccap_id")?;
        let ccap_id_raw = &header.pnm_data()
            [ccap_start - header.payload_offset()..reader.position() - header.payload_offset()];
        let terminator = ccap_id_raw
            .iter()
            .position(|byte| *byte == 0)
            .ok_or_else(|| {
                error(
                    ErrorCode::InvalidField,
                    CCAP_ID_OFFSET,
                    "ccap_id_terminator",
                )
            })?;
        let ccap_id = std::str::from_utf8(&ccap_id_raw[..terminator])
            .map_err(|_| error(ErrorCode::InvalidField, CCAP_ID_OFFSET, "ccap_id_encoding"))?;
        if ccap_id.chars().any(char::is_control) {
            return Err(error(
                ErrorCode::InvalidField,
                CCAP_ID_OFFSET,
                "ccap_id_encoding",
            ));
        }
        let rpd_id = reader.read_bytes("rpd_id")?;
        let rpd_port = reader.read_u8("rpd_port")?;
        let cm_mac_address = reader.read_bytes("cm_mac_address")?;
        let number_of_averages = reader.read_u16_be("number_of_averages")?;
        let pre_eq_raw = reader.read_u8("pre_eq_raw")?;
        let zero_frequency_hz = reader.read_u32_be("subcarrier_zero_frequency_hz")?;
        let first_index = reader.read_u16_be("first_active_subcarrier_index")?;
        let spacing_khz = reader.read_u8("subcarrier_spacing_khz")?;
        if ![25, 50].contains(&spacing_khz) {
            return Err(error(
                ErrorCode::InvalidField,
                SPACING_OFFSET,
                "subcarrier_spacing_khz",
            ));
        }
        let length = reader.read_u32_be("data_length")?;
        let count = usize::try_from(length)
            .map_err(|_| error(ErrorCode::LimitExceeded, LENGTH_OFFSET, "data_length"))?;
        if count > limits.max_samples {
            return Err(error(
                ErrorCode::LimitExceeded,
                LENGTH_OFFSET,
                "data_length",
            ));
        }
        let raw_values = reader.remaining();
        if raw_values.len() < count {
            return Err(error(
                ErrorCode::Truncated,
                reader.position() + raw_values.len(),
                "rxmer_payload",
            ));
        }
        if raw_values.len() != count {
            return Err(error(
                ErrorCode::LengthMismatch,
                LENGTH_OFFSET,
                "data_length",
            ));
        }
        if count > 0 {
            let last = u32::from(first_index)
                .checked_add(length - 1)
                .filter(|index| *index <= u32::from(u16::MAX))
                .ok_or_else(|| {
                    error(
                        ErrorCode::ArithmeticOverflow,
                        FIRST_INDEX_OFFSET,
                        "subcarrier_index",
                    )
                })?;
            last.checked_mul(u32::from(spacing_khz) * 1000)
                .and_then(|offset| zero_frequency_hz.checked_add(offset))
                .ok_or_else(|| {
                    error(
                        ErrorCode::ArithmeticOverflow,
                        FREQUENCY_OFFSET,
                        "sample_frequency_hz",
                    )
                })?;
        }
        Ok(Self {
            header,
            if_index,
            ccap_id_raw,
            ccap_id,
            rpd_id,
            rpd_port,
            cm_mac_address,
            number_of_averages,
            pre_eq_raw,
            zero_frequency_hz,
            first_index,
            spacing_khz,
            raw_values,
        })
    }
    /// Borrows raw octets including all unavailable markers.
    /// # Returns
    /// Original payload with the source lifetime, without copying.
    pub fn raw_values(&self) -> &'input [u8] {
        self.raw_values
    }
    /// Borrows the NUL-terminated CCAP identity prefix as decoded text.
    /// # Returns
    /// Original string prefix; raw padding remains available in JSON.
    pub fn ccap_id(&self) -> &'input str {
        self.ccap_id
    }
    /// Reads the upstream interface identity without narrowing it to a channel byte.
    /// # Returns
    /// Original four-byte ifIndex value.
    pub fn if_index(&self) -> u32 {
        self.if_index
    }
    /// Reads the target modem's encoded identity.
    /// # Returns
    /// Six original MAC octets.
    pub fn cm_mac_address(&self) -> MacAddress {
        self.cm_mac_address
    }
    /// Reads encoded grid spacing.
    /// # Returns
    /// Subcarrier spacing in kHz, either25 or50 for this supported profile.
    pub fn spacing_khz(&self) -> u8 {
        self.spacing_khz
    }
    /// Iterates decoded dB values with unavailable readings left absent.
    /// # Returns
    /// A lazy iterator; 0xff becomes None, every other byte is divided by4.
    pub fn values_db(&self) -> impl Iterator<Item = Option<RxMerDb>> + '_ {
        self.raw_values.iter().map(|raw| {
            if *raw == 255 {
                None
            } else {
                Some(f64::from(*raw) / 4.0)
            }
        })
    }
    /// Iterates the original contiguous physical frequency grid.
    /// # Returns
    /// Hz centers; arithmetic was checked during construction, including the last sample.
    pub fn frequencies_hz(&self) -> impl Iterator<Item = FrequencyHz> + '_ {
        (0..self.raw_values.len()).map(|index| {
            self.zero_frequency_hz
                + (u32::from(self.first_index) + index as u32) * u32::from(self.spacing_khz) * 1000
        })
    }
    /// Iterates lossless samples without compressing invalid positions.
    /// # Returns
    /// Typed raw/decoded/status/index/frequency records with no backing allocation.
    pub fn samples(&self) -> impl Iterator<Item = RxMerSample> + '_ {
        self.frequencies_hz()
            .zip(self.raw_values)
            .zip(self.values_db())
            .enumerate()
            .map(|(index, ((frequency_hz, raw), rxmer_db))| RxMerSample {
                subcarrier_index: self.first_index + index as u16,
                frequency_hz,
                raw: *raw,
                rxmer_db,
                valid: rxmer_db.is_some(),
                status: if rxmer_db.is_some() {
                    SampleStatus::Valid
                } else {
                    SampleStatus::Unavailable
                },
            })
    }
    /// Exports the canonical decoded representation through the bounded serializer.
    /// # Arguments
    /// * `max_bytes` - Maximum JSON output bytes.
    /// # Returns
    /// Complete JSON with explicit nulls, source identities and original octets.
    /// # Errors
    /// Propagates serialization errors or an exceeded output budget.
    pub fn to_json(&self, max_bytes: usize) -> serde_json::Result<String> {
        crate::common::json::to_json(self, max_bytes)
    }
}
fn error(code: ErrorCode, offset: usize, field: &'static str) -> ParseError {
    ParseError {
        code,
        offset,
        field,
    }
}
impl Serialize for CmtsUsOfdmaRxMer<'_> {
    /// Serializes typed metadata and lazy sample records using the shared contract.
    /// # Arguments
    /// * `serializer` - Destination; output budgets belong to to_json.
    /// # Returns
    /// Canonical upstream fields; no inferred DOCSIS generation or Boolean coding.
    /// # Errors
    /// Propagates destination errors without modifying input bytes.
    fn serialize<Output: Serializer>(
        &self,
        serializer: Output,
    ) -> Result<Output::Ok, Output::Error> {
        let mut output = serializer.serialize_struct("CmtsUsOfdmaRxMer", 20)?;
        output.serialize_field("pnm_header", &self.header.to_model().pnm_header)?;
        output.serialize_field("measurement_type", "cmts_upstream_ofdma_rxmer")?;
        output.serialize_field("device_role", "CMTS/CCAP")?;
        output.serialize_field("channel_direction", "upstream")?;
        output.serialize_field("docsis_version_context", &Option::<String>::None)?;
        output.serialize_field("file_type_identifier", "0x504E4E69")?;
        output.serialize_field("if_index", &self.if_index)?;
        output.serialize_field("ccap_id", &self.ccap_id)?;
        output.serialize_field("ccap_id_raw", &self.ccap_id_raw)?;
        output.serialize_field("rpd_id", &self.rpd_id)?;
        output.serialize_field("rpd_port", &self.rpd_port)?;
        output.serialize_field(
            "cm_mac_address",
            &crate::pnm::data_type::mac_text(&self.cm_mac_address),
        )?;
        output.serialize_field("number_of_averages", &self.number_of_averages)?;
        output.serialize_field("pre_eq_raw", &self.pre_eq_raw)?;
        output.serialize_field("subcarrier_zero_frequency_hz", &self.zero_frequency_hz)?;
        output.serialize_field("first_active_subcarrier_index", &self.first_index)?;
        output.serialize_field("subcarrier_spacing_khz", &self.spacing_khz)?;
        output.serialize_field("sample_count", &self.raw_values.len())?;
        output.serialize_field("value_units", "dB")?;
        output.serialize_field("samples", &Samples(self))?;
        output.end()
    }
}
struct Samples<'capture, 'input>(&'capture CmtsUsOfdmaRxMer<'input>);
impl Serialize for Samples<'_, '_> {
    fn serialize<Output: Serializer>(
        &self,
        serializer: Output,
    ) -> Result<Output::Ok, Output::Error> {
        serializer.collect_seq(self.0.samples())
    }
}
