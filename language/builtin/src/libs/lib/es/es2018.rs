use super::super::super::source::{BuiltinLib, BuiltinLibSource};

macro_rules! lib_source {
    ($name:ident, $file:literal) => {
        const $name: BuiltinLibSource = BuiltinLibSource::new(
            "lib",
            "es/es2018",
            $file,
            include_str!(concat!("../../../../lib/es/es2018/", $file)),
        );
    };
}

lib_source!(LIB_ES_ES2018_ASYNCGENERATOR_D_DS, "asyncgenerator.d.ds");
lib_source!(LIB_ES_ES2018_ASYNCITERABLE_D_DS, "asynciterable.d.ds");
lib_source!(LIB_ES_ES2018_FULL_D_DS, "full.d.ds");
lib_source!(LIB_ES_ES2018_INDEX_D_DS, "index.d.ds");
lib_source!(LIB_ES_ES2018_INTL_D_DS, "intl.d.ds");
lib_source!(LIB_ES_ES2018_PROMISE_D_DS, "promise.d.ds");
lib_source!(LIB_ES_ES2018_REGEXP_D_DS, "regexp.d.ds");

pub const LIB_ES2018: BuiltinLib = BuiltinLib::ambient(
    "es2018",
    &[
        LIB_ES_ES2018_ASYNCGENERATOR_D_DS,
        LIB_ES_ES2018_ASYNCITERABLE_D_DS,
        LIB_ES_ES2018_INDEX_D_DS,
        LIB_ES_ES2018_INTL_D_DS,
        LIB_ES_ES2018_PROMISE_D_DS,
        LIB_ES_ES2018_REGEXP_D_DS,
    ],
    &["es2017"],
);

pub const LIB_ES2018_ASYNCGENERATOR: BuiltinLib = BuiltinLib::ambient(
    "es2018.asyncgenerator",
    &[LIB_ES_ES2018_ASYNCGENERATOR_D_DS],
    &[],
);
pub const LIB_ES2018_ASYNCITERABLE: BuiltinLib = BuiltinLib::ambient(
    "es2018.asynciterable",
    &[LIB_ES_ES2018_ASYNCITERABLE_D_DS],
    &[],
);
pub const LIB_ES2018_FULL: BuiltinLib = BuiltinLib::ambient(
    "es2018.full",
    &[LIB_ES_ES2018_FULL_D_DS],
    &[
        "es2018",
        "dom",
        "worker.importscripts",
        "scripthost",
        "dom.iterable",
        "dom.asynciterable",
    ],
);
pub const LIB_ES2018_INTL: BuiltinLib =
    BuiltinLib::ambient("es2018.intl", &[LIB_ES_ES2018_INTL_D_DS], &[]);
pub const LIB_ES2018_PROMISE: BuiltinLib =
    BuiltinLib::ambient("es2018.promise", &[LIB_ES_ES2018_PROMISE_D_DS], &[]);
pub const LIB_ES2018_REGEXP: BuiltinLib =
    BuiltinLib::ambient("es2018.regexp", &[LIB_ES_ES2018_REGEXP_D_DS], &[]);
