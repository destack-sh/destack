use super::super::source::BuiltinLibSource;

pub const STD_TIME_INDEX_DS: BuiltinLibSource = BuiltinLibSource::new(
    "std",
    "time",
    "index.ds",
    include_str!(concat!("../../../std/time/index.ds")),
);
