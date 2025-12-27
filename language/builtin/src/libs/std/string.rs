use super::super::source::BuiltinLibSource;

pub const STD_STRING_INDEX_DS: BuiltinLibSource = BuiltinLibSource::new(
    "std",
    "string",
    "index.ds",
    include_str!(concat!("../../../std/string/index.ds")),
);
