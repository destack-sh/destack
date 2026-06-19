// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::Module;

/// One document accepted by formatter operations.
#[derive(Debug)]
#[napi(object, js_name = "Document")]
pub struct Document {
    /// Payload variant label.
    pub kind: String,
    /// Loaded source module.
    pub module_module: Option<Module>,
    /// Display path used for parser language detection.
    pub path: Option<String>,
    /// Source text.
    pub text_text: Option<String>,
}

impl Document {
    /// Convert this NAPI payload enum into one bridge enum.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::Document> {
        match self.kind.as_str() {
            "module" => {
                if self.path.is_some() {
                    return Err(unexpected_payload("path"));
                }
                if self.text_text.is_some() {
                    return Err(unexpected_payload("text_text"));
                }
                let Some(value) = self.module_module else {
                    return Err(missing_payload("moduleModule"));
                };
                let module = value.into_bridge()?;
                Ok(bridge::Document::Module { module })
            }
            "text" => {
                if self.module_module.is_some() {
                    return Err(unexpected_payload("module_module"));
                }
                let Some(value) = self.path else {
                    return Err(missing_payload("path"));
                };
                let path = value;
                let Some(value) = self.text_text else {
                    return Err(missing_payload("textText"));
                };
                let text = value;
                Ok(bridge::Document::Text { path, text })
            }
            _ => Err(napi::Error::from_reason(format!(
                "unknown {}: {}",
                stringify!(Document),
                self.kind
            ))),
        }
    }
}

/// Return one missing payload error.
fn missing_payload(kind: &str) -> napi::Error {
    napi::Error::from_reason(format!("{kind} payload is missing"))
}

/// Return one unexpected payload error.
fn unexpected_payload(kind: &str) -> napi::Error {
    napi::Error::from_reason(format!("{kind} payload is unexpected"))
}

/// One formatter request.
#[derive(Debug)]
#[napi(object, js_name = "FormatRequest")]
pub struct FormatRequest {
    /// Document to format.
    pub document: Document,
}

impl FormatRequest {
    /// Convert this NAPI value into one bridge value.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::FormatRequest> {
        Ok(bridge::FormatRequest {
            document: self.document.into_bridge()?,
        })
    }
}

/// One formatter output.
#[derive(Debug)]
#[napi(object, js_name = "FormatOutput")]
pub struct FormatOutput {
    /// Formatted source text.
    pub text: String,
}

impl FormatOutput {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::FormatOutput) -> Self {
        Self { text: value.text }
    }
}
