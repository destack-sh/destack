// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::Edit;

/// Source input used to open a live session.
#[derive(Debug)]
#[napi(object)]
pub struct Source {
    /// Payload variant label.
    pub kind: String,
    /// Source root or child path.
    pub path: Option<String>,
    /// Source root path used for repository identity.
    pub root: Option<String>,
    /// Edits used to seed the memory filesystem.
    pub edits: Option<Vec<Edit>>,
}

impl Source {
    /// Convert this NAPI payload enum into one bridge enum.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::Source> {
        match self.kind.as_str() {
            "fileSystem" => {
                if self.root.is_some() {
                    return Err(unexpected_payload("root"));
                }
                if self.edits.is_some() {
                    return Err(unexpected_payload("edits"));
                }
                let Some(value) = self.path else {
                    return Err(missing_payload("path"));
                };
                let path = value;
                Ok(bridge::Source::FileSystem { path })
            }
            "memory" => {
                if self.path.is_some() {
                    return Err(unexpected_payload("path"));
                }
                let Some(value) = self.root else {
                    return Err(missing_payload("root"));
                };
                let root = value;
                let Some(value) = self.edits else {
                    return Err(missing_payload("edits"));
                };
                let edits = value
                    .into_iter()
                    .map(|item| Ok::<_, napi::Error>(item.into_bridge()?))
                    .collect::<napi::Result<Vec<_>>>()?;
                Ok(bridge::Source::Memory { root, edits })
            }
            _ => Err(napi::Error::from_reason(format!(
                "unknown {}: {}",
                stringify!(Source),
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
