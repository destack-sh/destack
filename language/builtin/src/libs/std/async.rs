use super::super::source::BuiltinLibSource;

pub const STD_ASYNC_INDEX_DS: BuiltinLibSource = BuiltinLibSource::new(
    "std",
    "async",
    "index.ds",
    include_str!(concat!("../../../std/async/index.ds")),
);
