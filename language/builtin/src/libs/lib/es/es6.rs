use super::super::super::source::{BuiltinLib, BuiltinLibSource};

const LIB_ES_ES6_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "es/es6",
    "index.d.ds",
    include_str!(concat!("../../../../lib/es/es6/index.d.ds")),
);

pub const LIB_ES6: BuiltinLib = BuiltinLib::ambient(
    "es6",
    &[LIB_ES_ES6_INDEX_D_DS],
    &[
        "es2015",
        "dom",
        "worker.importscripts",
        "scripthost",
        "dom.iterable",
    ],
);
