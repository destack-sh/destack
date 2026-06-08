// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::SourceFile;

/// Source truth used to open a live session.
#[derive(Debug)]
#[napi(object)]
pub struct SourceSnapshot {
    /// Files visible to the session source root.
    pub files: Vec<SourceFile>,
}

impl SourceSnapshot {
    /// Convert this NAPI value into one bridge value.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::SourceSnapshot> {
        Ok(bridge::SourceSnapshot {
            files: self
                .files
                .into_iter()
                .map(|item| Ok::<_, napi::Error>(item.into_bridge()?))
                .collect::<napi::Result<Vec<_>>>()?,
        })
    }
}
