// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::Content;

/// One stable sidecar label crossing bridge boundaries.
#[derive(Debug)]
#[napi(object, js_name = "ArtifactSidecarLabel")]
pub struct ArtifactSidecarLabel {
    /// Label key.
    pub key: String,
    /// Label value.
    pub value: String,
}

impl ArtifactSidecarLabel {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::ArtifactSidecarLabel) -> Self {
        Self {
            key: value.key,
            value: value.value,
        }
    }
}

/// One named artifact sidecar crossing bridge boundaries.
#[derive(Debug)]
#[napi(object, js_name = "ArtifactSidecar")]
pub struct ArtifactSidecar {
    /// Sidecar name.
    pub name: String,
    /// Stable labels describing this sidecar.
    pub labels: Vec<ArtifactSidecarLabel>,
    /// Sidecar content.
    pub content: Content,
}

impl ArtifactSidecar {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::ArtifactSidecar) -> Self {
        Self {
            name: value.name,
            labels: value
                .labels
                .into_iter()
                .map(|item| ArtifactSidecarLabel::from_bridge(item))
                .collect(),
            content: Content::from_bridge(value.content),
        }
    }
}
