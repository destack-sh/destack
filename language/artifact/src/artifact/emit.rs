use serde::{Deserialize, Serialize};

use destack_builtin::BuiltinOutputFormat;

/// Emitted artifact family for a build target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum EmitFormat {
    /// JavaScript (.js).
    #[default]
    Js,
    /// TypeScript (.ts).
    Ts,
    /// HTML document output.
    Html,
    /// WebAssembly (.wasm).
    Wasm,
    /// Native binary.
    Native,
}

impl EmitFormat {
    /// Whether this family produces JavaScript output.
    pub fn is_js(&self) -> bool {
        matches!(self, Self::Js)
    }

    /// Whether this family produces TypeScript output.
    pub fn is_ts(&self) -> bool {
        matches!(self, Self::Ts)
    }

    /// Whether this family produces HTML document output.
    pub fn is_html(&self) -> bool {
        matches!(self, Self::Html)
    }

    /// Whether this family produces WebAssembly output.
    pub fn is_wasm(&self) -> bool {
        matches!(self, Self::Wasm)
    }

    /// Whether this family produces native binary output.
    pub fn is_native(&self) -> bool {
        matches!(self, Self::Native)
    }

    /// Whether this family uses the JavaScript generation pipeline.
    pub fn is_js_family(&self) -> bool {
        matches!(self, Self::Js | Self::Ts | Self::Html)
    }

    /// Whether this family uses the native generation pipeline.
    pub fn is_native_family(&self) -> bool {
        matches!(self, Self::Wasm | Self::Native)
    }

    /// Whether this family typically produces a single output file.
    pub fn is_single_file(&self) -> bool {
        matches!(self, Self::Html | Self::Wasm | Self::Native)
    }

    /// Convert this emit family to the builtin output family.
    pub fn builtin_output_format(&self) -> BuiltinOutputFormat {
        match self {
            Self::Js | Self::Html => BuiltinOutputFormat::Js,
            Self::Ts => BuiltinOutputFormat::Ts,
            Self::Wasm => BuiltinOutputFormat::Wasm,
            Self::Native => BuiltinOutputFormat::Native,
        }
    }
}
