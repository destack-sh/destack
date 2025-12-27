use super::super::source::BuiltinLibSource;

pub const STD_INDEX_DS: BuiltinLibSource = BuiltinLibSource::new(
    "std",
    "",
    "index.ds",
    include_str!(concat!("../../../std/index.ds")),
);
