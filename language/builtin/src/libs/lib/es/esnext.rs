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

lib_source!(LIB_ES_ESNEXT_ARRAY_D_DS, "array.d.ts");
lib_source!(LIB_ES_ESNEXT_COLLECTION_D_DS, "collection.d.ts");
lib_source!(LIB_ES_ESNEXT_DECORATORS_D_DS, "decorators.d.ts");
lib_source!(LIB_ES_ESNEXT_DISPOSABLE_D_DS, "disposable.d.ts");
lib_source!(LIB_ES_ESNEXT_ERROR_D_DS, "error.d.ts");
lib_source!(LIB_ES_ESNEXT_FLOAT16_D_DS, "float16.d.ts");
lib_source!(LIB_ES_ESNEXT_FULL_D_DS, "full.d.ts");
lib_source!(LIB_ES_ESNEXT_INDEX_D_DS, "index.d.ts");
lib_source!(LIB_ES_ESNEXT_INTL_D_DS, "intl.d.ts");
lib_source!(LIB_ES_ESNEXT_ITERATOR_D_DS, "iterator.d.ts");
lib_source!(LIB_ES_ESNEXT_PROMISE_D_DS, "promise.d.ts");
lib_source!(LIB_ES_ESNEXT_SHAREDMEMORY_D_DS, "sharedmemory.d.ts");

pub const LIB_ESNEXT: BuiltinLib = BuiltinLib::ambient(
    "esnext",
    &[
        LIB_ES_ESNEXT_ARRAY_D_DS,
        LIB_ES_ESNEXT_COLLECTION_D_DS,
        LIB_ES_ESNEXT_DECORATORS_D_DS,
        LIB_ES_ESNEXT_DISPOSABLE_D_DS,
        LIB_ES_ESNEXT_ERROR_D_DS,
        LIB_ES_ESNEXT_FLOAT16_D_DS,
        LIB_ES_ESNEXT_INDEX_D_DS,
        LIB_ES_ESNEXT_INTL_D_DS,
        LIB_ES_ESNEXT_ITERATOR_D_DS,
        LIB_ES_ESNEXT_PROMISE_D_DS,
        LIB_ES_ESNEXT_SHAREDMEMORY_D_DS,
    ],
    &["es2024", "decorators", "es2015.iterable", "es2015.symbol"],
);

pub const LIB_ESNEXT_ARRAY: BuiltinLib =
    BuiltinLib::ambient("esnext.array", &[LIB_ES_ESNEXT_ARRAY_D_DS], &[]);
pub const LIB_ESNEXT_COLLECTION: BuiltinLib =
    BuiltinLib::ambient("esnext.collection", &[LIB_ES_ESNEXT_COLLECTION_D_DS], &[]);
pub const LIB_ESNEXT_DECORATORS: BuiltinLib = BuiltinLib::ambient(
    "esnext.decorators",
    &[LIB_ES_ESNEXT_DECORATORS_D_DS],
    &["decorators", "es2015.symbol"],
);
pub const LIB_ESNEXT_DISPOSABLE: BuiltinLib =
    BuiltinLib::ambient("esnext.disposable", &[LIB_ES_ESNEXT_DISPOSABLE_D_DS], &[]);
pub const LIB_ESNEXT_ERROR: BuiltinLib =
    BuiltinLib::ambient("esnext.error", &[LIB_ES_ESNEXT_ERROR_D_DS], &[]);
pub const LIB_ESNEXT_FLOAT16: BuiltinLib = BuiltinLib::ambient(
    "esnext.float16",
    &[LIB_ES_ESNEXT_FLOAT16_D_DS],
    &["es2015.symbol", "es2015.iterable"],
);
pub const LIB_ESNEXT_FULL: BuiltinLib = BuiltinLib::ambient(
    "esnext.full",
    &[LIB_ES_ESNEXT_FULL_D_DS],
    &[
        "esnext",
        "dom",
        "worker.importscripts",
        "scripthost",
        "dom.iterable",
        "dom.asynciterable",
    ],
);
pub const LIB_ESNEXT_INTL: BuiltinLib =
    BuiltinLib::ambient("esnext.intl", &[LIB_ES_ESNEXT_INTL_D_DS], &[]);
pub const LIB_ESNEXT_ITERATOR: BuiltinLib =
    BuiltinLib::ambient("esnext.iterator", &[LIB_ES_ESNEXT_ITERATOR_D_DS], &[]);
pub const LIB_ESNEXT_PROMISE: BuiltinLib =
    BuiltinLib::ambient("esnext.promise", &[LIB_ES_ESNEXT_PROMISE_D_DS], &[]);
pub const LIB_ESNEXT_SHAREDMEMORY: BuiltinLib = BuiltinLib::ambient(
    "esnext.sharedmemory",
    &[LIB_ES_ESNEXT_SHAREDMEMORY_D_DS],
    &[],
);
