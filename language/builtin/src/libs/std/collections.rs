use super::super::source::BuiltinLibSource;

pub const STD_COLLECTIONS_INDEX_DS: BuiltinLibSource = BuiltinLibSource::new(
    "std",
    "collections",
    "index.ds",
    include_str!(concat!("../../../std/collections/index.ds")),
);
