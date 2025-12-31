use super::super::super::source::{BuiltinLib, BuiltinLibSource};
use super::ES_CANONICAL_EXPORTS;

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
lib_source!(LIB_ES_ES2022_FULL_D_DS, "full.d.ds");
lib_source!(LIB_ES_ES2022_INDEX_D_DS, "index.d.ds");
lib_source!(LIB_ES_ES2022_INTL_D_DS, "intl.d.ds");
lib_source!(LIB_ES_ES2022_OBJECT_D_DS, "object.d.ds");
lib_source!(LIB_ES_ES2022_REGEXP_D_DS, "regexp.d.ds");
lib_source!(LIB_ES_ES2022_STRING_D_DS, "string.d.ds");

pub const LIB_ES2022: BuiltinLib = BuiltinLib::ambient(
    "es2022",
    &[
        LIB_ES_ES2022_ARRAY_D_DS,
        LIB_ES_ES2022_ERROR_D_DS,
        LIB_ES_ES2022_INDEX_D_DS,
        LIB_ES_ES2022_INTL_D_DS,
        LIB_ES_ES2022_OBJECT_D_DS,
        LIB_ES_ES2022_REGEXP_D_DS,
        LIB_ES_ES2022_STRING_D_DS,
    ],
    &["es2021"],
)
.with_canonical_exports(ES_CANONICAL_EXPORTS);

pub const LIB_ES2022_ARRAY: BuiltinLib =
    BuiltinLib::ambient("es2022.array", &[LIB_ES_ES2022_ARRAY_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2022_ERROR: BuiltinLib =
    BuiltinLib::ambient("es2022.error", &[LIB_ES_ES2022_ERROR_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2022_FULL: BuiltinLib = BuiltinLib::ambient(
    "es2022.full",
    &[LIB_ES_ES2022_FULL_D_DS],
    &[
        "es2022",
        "dom",
        "worker.importscripts",
        "scripthost",
        "dom.iterable",
        "dom.asynciterable",
    ],
)
.with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2022_INTL: BuiltinLib =
    BuiltinLib::ambient("es2022.intl", &[LIB_ES_ES2022_INTL_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2022_OBJECT: BuiltinLib =
    BuiltinLib::ambient("es2022.object", &[LIB_ES_ES2022_OBJECT_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2022_REGEXP: BuiltinLib =
    BuiltinLib::ambient("es2022.regexp", &[LIB_ES_ES2022_REGEXP_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2022_STRING: BuiltinLib =
    BuiltinLib::ambient("es2022.string", &[LIB_ES_ES2022_STRING_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
