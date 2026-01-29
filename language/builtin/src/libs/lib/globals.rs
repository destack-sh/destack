use super::super::source::{BuiltinLib, BuiltinLibSource};

const LIB_GLOBALS_INDEX_D_TS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "globals",
    "index.d.ts",
    include_str!("../../../lib/globals/index.d.ts"),
);

pub const LIB_GLOBALS: BuiltinLib =
    BuiltinLib::ambient_lib("globals", &[LIB_GLOBALS_INDEX_D_TS], &[]);
