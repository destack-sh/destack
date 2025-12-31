use super::super::super::source::{BuiltinLib, BuiltinLibSource};
use super::ES_CANONICAL_EXPORTS;

macro_rules! lib_source {
    ($name:ident, $file:literal) => {
        const $name: BuiltinLibSource = BuiltinLibSource::new(
            "lib",
            "es/es2023",
            $file,
            include_str!(concat!("../../../../lib/es/es2023/", $file)),
        );
    };
}

lib_source!(LIB_ES_ES2023_ARRAY_D_DS, "array.d.ds");
lib_source!(LIB_ES_ES2023_COLLECTION_D_DS, "collection.d.ds");
lib_source!(LIB_ES_ES2023_FULL_D_DS, "full.d.ds");
lib_source!(LIB_ES_ES2023_INDEX_D_DS, "index.d.ds");
lib_source!(LIB_ES_ES2023_INTL_D_DS, "intl.d.ds");

pub const LIB_ES2023: BuiltinLib = BuiltinLib::ambient(
    "es2023",
    &[
        LIB_ES_ES2023_ARRAY_D_DS,
        LIB_ES_ES2023_COLLECTION_D_DS,
        LIB_ES_ES2023_INDEX_D_DS,
        LIB_ES_ES2023_INTL_D_DS,
    ],
    &["es2022"],
)
.with_canonical_exports(ES_CANONICAL_EXPORTS);

pub const LIB_ES2023_ARRAY: BuiltinLib =
    BuiltinLib::ambient("es2023.array", &[LIB_ES_ES2023_ARRAY_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2023_COLLECTION: BuiltinLib =
    BuiltinLib::ambient("es2023.collection", &[LIB_ES_ES2023_COLLECTION_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2023_FULL: BuiltinLib = BuiltinLib::ambient(
    "es2023.full",
    &[LIB_ES_ES2023_FULL_D_DS],
    &[
        "es2023",
        "dom",
        "worker.importscripts",
        "scripthost",
        "dom.iterable",
        "dom.asynciterable",
    ],
)
.with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2023_INTL: BuiltinLib =
    BuiltinLib::ambient("es2023.intl", &[LIB_ES_ES2023_INTL_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
