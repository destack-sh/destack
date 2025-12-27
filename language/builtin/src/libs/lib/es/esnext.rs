use super::super::super::source::{BuiltinLib, BuiltinLibSource};

macro_rules! lib_source {
    ($name:ident, $file:literal) => {
        const $name: BuiltinLibSource = BuiltinLibSource::new(
            "lib",
            "es/esnext",
            $file,
            include_str!(concat!("../../../../lib/es/esnext/", $file)),
        );
    };
}

lib_source!(LIB_ES_ESNEXT_ARRAY_D_DS, "array.d.ds");
lib_source!(LIB_ES_ESNEXT_COLLECTION_D_DS, "collection.d.ds");
lib_source!(LIB_ES_ESNEXT_DISPOSABLE_D_DS, "disposable.d.ds");
lib_source!(LIB_ES_ESNEXT_INTL_D_DS, "intl.d.ds");
lib_source!(LIB_ES_ESNEXT_ITERATOR_D_DS, "iterator.d.ds");
lib_source!(LIB_ES_ESNEXT_PROMISE_D_DS, "promise.d.ds");
lib_source!(LIB_ES_ESNEXT_REGEXP_D_DS, "regexp.d.ds");
lib_source!(LIB_ES_ESNEXT_STRING_D_DS, "string.d.ds");

pub const LIB_ESNEXT: BuiltinLib = BuiltinLib::ambient(
    "esnext",
    &[
        LIB_ES_ESNEXT_ARRAY_D_DS,
        LIB_ES_ESNEXT_COLLECTION_D_DS,
        LIB_ES_ESNEXT_DISPOSABLE_D_DS,
        LIB_ES_ESNEXT_INTL_D_DS,
        LIB_ES_ESNEXT_ITERATOR_D_DS,
        LIB_ES_ESNEXT_PROMISE_D_DS,
        LIB_ES_ESNEXT_REGEXP_D_DS,
        LIB_ES_ESNEXT_STRING_D_DS,
    ],
    &["es2024"],
);

pub const LIB_ESNEXT_ARRAY: BuiltinLib =
    BuiltinLib::ambient("esnext.array", &[LIB_ES_ESNEXT_ARRAY_D_DS], &[]);
pub const LIB_ESNEXT_COLLECTION: BuiltinLib =
    BuiltinLib::ambient("esnext.collection", &[LIB_ES_ESNEXT_COLLECTION_D_DS], &[]);
pub const LIB_ESNEXT_DISPOSABLE: BuiltinLib =
    BuiltinLib::ambient("esnext.disposable", &[LIB_ES_ESNEXT_DISPOSABLE_D_DS], &[]);
pub const LIB_ESNEXT_INTL: BuiltinLib =
    BuiltinLib::ambient("esnext.intl", &[LIB_ES_ESNEXT_INTL_D_DS], &[]);
pub const LIB_ESNEXT_ITERATOR: BuiltinLib =
    BuiltinLib::ambient("esnext.iterator", &[LIB_ES_ESNEXT_ITERATOR_D_DS], &[]);
pub const LIB_ESNEXT_PROMISE: BuiltinLib =
    BuiltinLib::ambient("esnext.promise", &[LIB_ES_ESNEXT_PROMISE_D_DS], &[]);
pub const LIB_ESNEXT_REGEXP: BuiltinLib =
    BuiltinLib::ambient("esnext.regexp", &[LIB_ES_ESNEXT_REGEXP_D_DS], &[]);
pub const LIB_ESNEXT_STRING: BuiltinLib =
    BuiltinLib::ambient("esnext.string", &[LIB_ES_ESNEXT_STRING_D_DS], &[]);
