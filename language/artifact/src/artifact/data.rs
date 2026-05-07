use serde::{Deserialize, Serialize};

use crate::{Css, Html};

/// One parsed non-code module payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Data {
    /// One parsed JSON-like module value.
    Json(serde_json::Value),
    /// One parsed HTML module payload.
    Html(Box<Html>),
    /// One parsed CSS module payload.
    Css(Box<Css>),
}
