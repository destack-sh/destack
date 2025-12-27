use super::super::source::BuiltinLibSource;

pub const STD_IO_INDEX_DS: BuiltinLibSource = BuiltinLibSource::new(
    "std",
    "io",
    "index.ds",
    include_str!(concat!("../../../std/io/index.ds")),
);
