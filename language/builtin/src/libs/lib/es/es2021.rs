use super::super::super::source::{BuiltinLib, BuiltinLibSource};

macro_rules! lib_source {
    ($name:ident, $file:literal) => {
        const $name: BuiltinLibSource = BuiltinLibSource::new(
            "lib",
            "es/es2021",
            $file,
            include_str!(concat!("../../../../lib/es/es2021/", $file)),
        );
    };
}

lib_source!(LIB_ES_ES2021_INTL_D_DS, "intl.d.ds");
lib_source!(LIB_ES_ES2021_FULL_D_DS, "full.d.ds");
lib_source!(LIB_ES_ES2021_INDEX_D_DS, "index.d.ds");
lib_source!(LIB_ES_ES2021_PROMISE_D_DS, "promise.d.ds");
lib_source!(LIB_ES_ES2021_STRING_D_DS, "string.d.ds");
lib_source!(LIB_ES_ES2021_WEAKREF_D_DS, "weakref.d.ds");

pub const LIB_ES2021: BuiltinLib = BuiltinLib::ambient(
    "es2021",
    &[
        LIB_ES_ES2021_INDEX_D_DS,
        LIB_ES_ES2021_INTL_D_DS,
        LIB_ES_ES2021_PROMISE_D_DS,
        LIB_ES_ES2021_STRING_D_DS,
        LIB_ES_ES2021_WEAKREF_D_DS,
    ],
    &["es2020"],
);

pub const LIB_ES2021_INTL: BuiltinLib =
    BuiltinLib::ambient("es2021.intl", &[LIB_ES_ES2021_INTL_D_DS], &[]);
pub const LIB_ES2021_FULL: BuiltinLib = BuiltinLib::ambient(
    "es2021.full",
    &[LIB_ES_ES2021_FULL_D_DS],
    &[
        "es2021",
        "dom",
        "worker.importscripts",
        "scripthost",
        "dom.iterable",
        "dom.asynciterable",
    ],
);
pub const LIB_ES2021_PROMISE: BuiltinLib =
    BuiltinLib::ambient("es2021.promise", &[LIB_ES_ES2021_PROMISE_D_DS], &[]);
pub const LIB_ES2021_STRING: BuiltinLib =
    BuiltinLib::ambient("es2021.string", &[LIB_ES_ES2021_STRING_D_DS], &[]);
pub const LIB_ES2021_WEAKREF: BuiltinLib =
    BuiltinLib::ambient("es2021.weakref", &[LIB_ES_ES2021_WEAKREF_D_DS], &[]);
