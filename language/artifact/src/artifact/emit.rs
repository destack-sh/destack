use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Emitted artifact family for a build target.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    Reflect,
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum EmitFormat {
    /// JavaScript (.js).
    #[default]
    Js,
    /// TypeScript (.ts).
    Ts,
    /// Destack bytecode program.
    Bytecode,
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
            Self::Bytecode => "bytecode",
            Self::Wasm => "wasm",
            Self::Native => "native",
        }
    }

    /// Whether this family produces a script artifact.
    pub fn is_script(&self) -> bool {
        matches!(self, Self::Js | Self::Ts)
    }

    /// Whether this family produces a linked Program artifact.
    pub fn is_program(&self) -> bool {
        matches!(self, Self::Bytecode | Self::Wasm | Self::Native)
    }

    /// Whether this family typically produces a single output file.
    pub fn is_single_file(&self) -> bool {
        self.is_program()
    }
}
