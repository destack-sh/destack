use super::super::source::BuiltinLibSource;

pub const STD_ARRAY_INDEX_DS: BuiltinLibSource = BuiltinLibSource::new(
    "std",
    "array",
    "index.ds",
    include_str!(concat!("../../../std/array/index.ds")),
);
