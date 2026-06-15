use serde::{Deserialize, Serialize};

/// Emitted artifact family for a build target.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum EmitFormat {
    /// JavaScript (.js).
    #[default]
    Js,
    /// TypeScript (.ts).
    Ts,
    /// WebAssembly (.wasm).
    Wasm,
    /// Native binary.
    Native,
}

impl EmitFormat {
    /// The canonical lowercase tag for this format.
    pub fn canonical_tag(&self) -> &'static str {
        match self {
            Self::Js => "js",
            Self::Ts => "ts",
            Self::Wasm => "wasm",
            Self::Native => "native",
        }
    }

    /// Whether this family produces JavaScript output.
    pub fn is_js(&self) -> bool {
        matches!(self, Self::Js)
    }

    /// Whether this family produces TypeScript output.
    pub fn is_ts(&self) -> bool {
        matches!(self, Self::Ts)
    }

    /// Whether this family produces WebAssembly output.
    pub fn is_wasm(&self) -> bool {
        matches!(self, Self::Wasm)
    }

    /// Whether this family produces native output.
    pub fn is_native(&self) -> bool {
        matches!(self, Self::Native)
    }

    /// Whether this family uses the JavaScript emit pipeline.
    pub fn is_js_family(&self) -> bool {
        matches!(self, Self::Js | Self::Ts)
    }

    /// Whether this family uses the native emit pipeline.
    pub fn is_native_family(&self) -> bool {
        matches!(self, Self::Wasm | Self::Native)
    }

    /// Whether this family typically produces a single output file.
    pub fn is_single_file(&self) -> bool {
        matches!(self, Self::Wasm | Self::Native)
    }
}
