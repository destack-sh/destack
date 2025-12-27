use super::super::super::source::{BuiltinLib, BuiltinLibSource};

macro_rules! lib_source {
    ($name:ident, $file:literal) => {
        const $name: BuiltinLibSource = BuiltinLibSource::new(
            "lib",
            "es/es2022",
            $file,
            include_str!(concat!("../../../../lib/es/es2022/", $file)),
        );
    };
}

lib_source!(LIB_ES_ES2022_ARRAY_D_DS, "array.d.ds");
lib_source!(LIB_ES_ES2022_ERROR_D_DS, "error.d.ds");
lib_source!(LIB_ES_ES2022_INTL_D_DS, "intl.d.ds");
lib_source!(LIB_ES_ES2022_OBJECT_D_DS, "object.d.ds");
lib_source!(LIB_ES_ES2022_REGEXP_D_DS, "regexp.d.ds");
lib_source!(LIB_ES_ES2022_SHAREDMEMORY_D_DS, "sharedmemory.d.ds");
lib_source!(LIB_ES_ES2022_STRING_D_DS, "string.d.ds");

pub const LIB_ES2022: BuiltinLib = BuiltinLib::ambient(
    "es2022",
    &[
        LIB_ES_ES2022_ARRAY_D_DS,
        LIB_ES_ES2022_ERROR_D_DS,
        LIB_ES_ES2022_INTL_D_DS,
        LIB_ES_ES2022_OBJECT_D_DS,
        LIB_ES_ES2022_REGEXP_D_DS,
        LIB_ES_ES2022_SHAREDMEMORY_D_DS,
        LIB_ES_ES2022_STRING_D_DS,
    ],
    &["es2021"],
);

pub const LIB_ES2022_ARRAY: BuiltinLib =
    BuiltinLib::ambient("es2022.array", &[LIB_ES_ES2022_ARRAY_D_DS], &[]);
pub const LIB_ES2022_ERROR: BuiltinLib =
    BuiltinLib::ambient("es2022.error", &[LIB_ES_ES2022_ERROR_D_DS], &[]);
pub const LIB_ES2022_INTL: BuiltinLib =
    BuiltinLib::ambient("es2022.intl", &[LIB_ES_ES2022_INTL_D_DS], &[]);
pub const LIB_ES2022_OBJECT: BuiltinLib =
    BuiltinLib::ambient("es2022.object", &[LIB_ES_ES2022_OBJECT_D_DS], &[]);
pub const LIB_ES2022_REGEXP: BuiltinLib =
    BuiltinLib::ambient("es2022.regexp", &[LIB_ES_ES2022_REGEXP_D_DS], &[]);
pub const LIB_ES2022_SHAREDMEMORY: BuiltinLib = BuiltinLib::ambient(
    "es2022.sharedmemory",
    &[LIB_ES_ES2022_SHAREDMEMORY_D_DS],
    &[],
);
pub const LIB_ES2022_STRING: BuiltinLib =
    BuiltinLib::ambient("es2022.string", &[LIB_ES_ES2022_STRING_D_DS], &[]);
