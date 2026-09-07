use destack_source::SourceMap;

use crate::{CompilerError, CompilerResult};

const BASE64_VLQ_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// One mapped or unmapped source map position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceMapMarker {
    /// Start a mapped generated range.
    Mapped {
        /// The emitted byte offset.
        emitted_byte: u32,
        /// The mapped source index.
        source_index: usize,
        /// The mapped source line.
        original_line: u32,
        /// The mapped source UTF-16 column.
        original_column: u32,
        /// The authored identifier name index when one exists.
        name: Option<usize>,
    },
    /// Start an unmapped generated range.
    Unmapped {
        /// The emitted byte offset.
        emitted_byte: u32,
    },
}

impl SourceMapMarker {
    /// Return the emitted byte offset.
    const fn emitted_byte(self) -> u32 {
        match self {
            Self::Mapped { emitted_byte, .. } | Self::Unmapped { emitted_byte } => emitted_byte,
        }
    }

    /// Return the mutable emitted byte offset.
    fn emitted_byte_mut(&mut self) -> &mut u32 {
        match self {
            Self::Mapped { emitted_byte, .. } | Self::Unmapped { emitted_byte } => emitted_byte,
        }
    }
}

/// One source map builder before VLQ encoding.
#[derive(Debug, Clone)]
pub(crate) struct SourceMapBuilder {
    /// The mapped source paths.
    sources: Vec<String>,
    /// The embedded source contents when requested.
    sources_content: Option<Vec<Option<String>>>,
    /// The precise source markers.
    markers: Vec<SourceMapMarker>,
    /// Authored identifier names referenced by markers.
    names: Vec<String>,
}

impl SourceMapBuilder {
    /// Build one invalid source map error.
    fn error(message: impl Into<String>) -> CompilerError {
        CompilerError::Internal {
            message: message.into(),
        }
    }

    /// Create one source map builder with authored identifier names.
    pub(crate) fn new(
        sources: Vec<String>,
        sources_content: Option<Vec<Option<String>>>,
        names: Vec<String>,
        markers: Vec<SourceMapMarker>,
    ) -> Self {
        Self {
            sources,
            sources_content,
            markers,
            names,
        }
    }

    /// Shift all emitted positions by one byte prefix length.
    pub(crate) fn shift(&mut self, byte_count: u32) {
        if byte_count == 0 {
            return;
        }

        for marker in &mut self.markers {
            *marker.emitted_byte_mut() += byte_count;
        }
    }

    /// Build one encoded source map for one emitted text payload.
    pub(crate) fn build(
        &self,
        emitted_code: &str,
        trailing_unmapped_line_count: usize,
    ) -> CompilerResult<SourceMap> {
        let mappings =
            Self::encode_mappings(emitted_code, trailing_unmapped_line_count, &self.markers)?;

        Ok(SourceMap {
            file: None,
            source_root: None,
            sources: self.sources.iter().cloned().map(Some).collect(),
            sources_content: self.sources_content.clone(),
            names: self.names.clone(),
            mappings,
            ignore_list: Vec::new(),
            debug_id: None,
        })
    }

    /// Return the number of visible lines in one text payload.
    fn line_count(text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }

        let newline_count = text.bytes().filter(|byte| *byte == b'\n').count();

        if text.ends_with('\n') {
            return newline_count;
        }

        newline_count + 1
    }

    /// Encode one source map marker list into one VLQ mappings payload.
    fn encode_mappings(
        emitted_code: &str,
        trailing_unmapped_line_count: usize,
        markers: &[SourceMapMarker],
    ) -> CompilerResult<String> {
        let mut mappings = String::new();
        let mut emitted_byte = 0usize;
        let mut emitted_line = 0u32;
        let mut emitted_column = 0u32;
        let mut previous_emitted_line = 0u32;
        let mut previous_emitted_column = 0i64;
        let mut previous_source_index = 0i64;
        let mut previous_original_line = 0i64;
        let mut previous_original_column = 0i64;
        let mut previous_name_index = 0i64;
        let total_line_count =
            (Self::line_count(emitted_code) + trailing_unmapped_line_count) as u32;
        let mut is_first_segment_on_line = true;

        for (marker_index, marker) in markers.iter().enumerate() {
            let marker_byte = marker.emitted_byte() as usize;
            if marker_byte < emitted_byte {
                return Err(SourceMapBuilder::error(format!(
                    "source map marker byte {marker_byte} precedes byte {emitted_byte}"
                )));
            }

            // omit one terminal unmapped position when no final text follows it
            let is_terminal_unmapped = matches!(marker, SourceMapMarker::Unmapped { .. })
                && marker_index + 1 == markers.len()
                && marker_byte == emitted_code.len()
                && trailing_unmapped_line_count == 0;
            if is_terminal_unmapped {
                break;
            }

            // advance through each emitted scalar once
            let text = emitted_code.get(emitted_byte..marker_byte).ok_or_else(|| {
                SourceMapBuilder::error(format!(
                    "source map marker byte {marker_byte} is outside emitted text"
                ))
            })?;
            for character in text.chars() {
                if character == '\n' {
                    emitted_line += 1;
                    emitted_column = 0;
                } else {
                    emitted_column += character.len_utf16() as u32;
                }
            }
            emitted_byte = marker_byte;

            while previous_emitted_line < emitted_line {
                mappings.push(';');
                previous_emitted_line += 1;
                previous_emitted_column = 0;
                is_first_segment_on_line = true;
            }

            if !is_first_segment_on_line {
                mappings.push(',');
            }

            let mapped_emitted_column = i64::from(emitted_column);
            Self::encode_vlq(
                mapped_emitted_column - previous_emitted_column,
                &mut mappings,
            )?;

            if let SourceMapMarker::Mapped {
                source_index,
                original_line,
                original_column,
                name,
                ..
            } = *marker
            {
                let source_index = source_index as i64;
                let original_line = i64::from(original_line);
                let original_column = i64::from(original_column);

                Self::encode_vlq(source_index - previous_source_index, &mut mappings)?;
                Self::encode_vlq(original_line - previous_original_line, &mut mappings)?;
                Self::encode_vlq(original_column - previous_original_column, &mut mappings)?;
                if let Some(name) = name {
                    let name = name as i64;
                    Self::encode_vlq(name - previous_name_index, &mut mappings)?;
                    previous_name_index = name;
                }

                previous_source_index = source_index;
                previous_original_line = original_line;
                previous_original_column = original_column;
            }

            previous_emitted_column = mapped_emitted_column;
            is_first_segment_on_line = false;
        }

        // keep trailing unmapped output lines visible in the final map
        let last_emitted_line = total_line_count.saturating_sub(1);
        while previous_emitted_line < last_emitted_line {
            mappings.push(';');
            previous_emitted_line += 1;
        }

        Ok(mappings)
    }

    /// Encode one signed integer into one base64 VLQ segment.
    fn encode_vlq(value: i64, output: &mut String) -> CompilerResult<()> {
        if i32::try_from(value).is_err() {
            return Err(Self::error("source map VLQ value exceeds 32 bits"));
        }
        let mut value = Self::encode_signed(value);

        loop {
            let mut digit = value & 0b1_1111;
            value >>= 5;

            if value != 0 {
                digit |= 0b10_0000;
            }

            output.push(BASE64_VLQ_ALPHABET[digit as usize] as char);

            if value == 0 {
                break;
            }
        }

        Ok(())
    }

    /// Convert one signed integer into one VLQ signed payload.
    fn encode_signed(value: i64) -> u64 {
        let magnitude = value.unsigned_abs();

        if value < 0 {
            (magnitude << 1) | 1
        } else {
            magnitude << 1
        }
    }
}
