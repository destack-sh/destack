use std::io::{Error, ErrorKind};

use serde_json::{self, Value};
use tspp_source::{File, strip_json};

/// Build one file content error.
fn json_content_error(message: &str) -> serde_json::Error {
    serde_json::Error::io(Error::new(ErrorKind::InvalidData, message))
}

/// Parse one strict file text payload.
pub fn parse_json_text(content: &str) -> Result<Value, serde_json::Error> {
    let json_str = content.strip_prefix('\u{feff}').unwrap_or(content);

    serde_json::from_str(json_str)
}

/// Parse one fileC text payload.
pub fn parse_jsonc_text(content: &str) -> Result<Value, serde_json::Error> {
    let json_content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let json_str = strip_json(json_content).map_err(serde_json::Error::io)?;
    let json_str = if json_str.trim().is_empty() {
        "{}".to_string()
    } else {
        json_str
    };

    serde_json::from_str(&json_str)
}

/// Parse one file as strict file text.
pub fn parse_json_file(file: &File) -> Result<Value, serde_json::Error> {
    if !file.is_text() {
        return Err(json_content_error("file is not text"));
    }

    parse_json_text(file.text())
}

/// Parse one file as fileC text.
pub fn parse_jsonc_file(file: &File) -> Result<Value, serde_json::Error> {
    if !file.is_text() {
        return Err(json_content_error("file is not text"));
    }

    parse_jsonc_text(file.text())
}
