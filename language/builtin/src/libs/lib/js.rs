use super::super::source::{BuiltinLib, BuiltinLibSource};

pub(crate) const JS_DECLARED_SYMBOLS: &[&str] = &["Slice"];

// root
const LIB_JS_INDEX_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "js",
    "index.ds",
    include_str!("../../../lib/js/index.ds"),
);

// collections/
const LIB_JS_COLLECTIONS_INDEX_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "js/collections",
    "index.ds",
    include_str!("../../../lib/js/collections/index.ds"),
);

const LIB_JS_COLLECTIONS_SLICE_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "js/collections",
    "slice.ds",
    include_str!("../../../lib/js/collections/slice.ds"),
);

pub const LIB_JS: BuiltinLib = BuiltinLib::ambient_lib(
    "js",
    &[
        // root
        LIB_JS_INDEX_DS,
        // collections/
        LIB_JS_COLLECTIONS_INDEX_DS,
        LIB_JS_COLLECTIONS_SLICE_DS,
    ],
    &["es5"],
)
.with_declared_symbols(JS_DECLARED_SYMBOLS);
