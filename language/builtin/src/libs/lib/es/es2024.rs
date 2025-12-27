use super::super::super::source::{BuiltinLib, BuiltinLibSource};

macro_rules! lib_source {
    ($name:ident, $file:literal) => {
        const $name: BuiltinLibSource = BuiltinLibSource::new(
            "lib",
            "es/es2024",
            $file,
            include_str!(concat!("../../../../lib/es/es2024/", $file)),
        );
    };
}

lib_source!(LIB_ES_ES2024_ARRAYBUFFER_D_DS, "arraybuffer.d.ds");
lib_source!(LIB_ES_ES2024_COLLECTION_D_DS, "collection.d.ds");
lib_source!(LIB_ES_ES2024_OBJECT_D_DS, "object.d.ds");
lib_source!(LIB_ES_ES2024_PROMISE_D_DS, "promise.d.ds");
lib_source!(LIB_ES_ES2024_REGEXP_D_DS, "regexp.d.ds");
lib_source!(LIB_ES_ES2024_SHAREDMEMORY_D_DS, "sharedmemory.d.ds");
lib_source!(LIB_ES_ES2024_STRING_D_DS, "string.d.ds");

pub const LIB_ES2024: BuiltinLib = BuiltinLib::ambient(
    "es2024",
    &[
        LIB_ES_ES2024_ARRAYBUFFER_D_DS,
        LIB_ES_ES2024_COLLECTION_D_DS,
        LIB_ES_ES2024_OBJECT_D_DS,
        LIB_ES_ES2024_PROMISE_D_DS,
        LIB_ES_ES2024_REGEXP_D_DS,
        LIB_ES_ES2024_SHAREDMEMORY_D_DS,
        LIB_ES_ES2024_STRING_D_DS,
    ],
    &["es2023"],
);

pub const LIB_ES2024_ARRAYBUFFER: BuiltinLib =
    BuiltinLib::ambient("es2024.arraybuffer", &[LIB_ES_ES2024_ARRAYBUFFER_D_DS], &[]);
pub const LIB_ES2024_COLLECTION: BuiltinLib =
    BuiltinLib::ambient("es2024.collection", &[LIB_ES_ES2024_COLLECTION_D_DS], &[]);
pub const LIB_ES2024_OBJECT: BuiltinLib =
    BuiltinLib::ambient("es2024.object", &[LIB_ES_ES2024_OBJECT_D_DS], &[]);
pub const LIB_ES2024_PROMISE: BuiltinLib =
    BuiltinLib::ambient("es2024.promise", &[LIB_ES_ES2024_PROMISE_D_DS], &[]);
pub const LIB_ES2024_REGEXP: BuiltinLib =
    BuiltinLib::ambient("es2024.regexp", &[LIB_ES_ES2024_REGEXP_D_DS], &[]);
pub const LIB_ES2024_SHAREDMEMORY: BuiltinLib = BuiltinLib::ambient(
    "es2024.sharedmemory",
    &[LIB_ES_ES2024_SHAREDMEMORY_D_DS],
    &[],
);
pub const LIB_ES2024_STRING: BuiltinLib =
    BuiltinLib::ambient("es2024.string", &[LIB_ES_ES2024_STRING_D_DS], &[]);
