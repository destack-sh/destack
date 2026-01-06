use super::super::super::source::{BuiltinLib, BuiltinLibSource};
use super::es2022::ES2022_DECLARED_SYMBOLS;

pub(crate) const ES2023_DECLARED_SYMBOLS: &[&str] = ES2022_DECLARED_SYMBOLS;
const ES2023_EMPTY_DECLARED_SYMBOLS: &[&str] = &[];

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

lib_source!(LIB_ES_ES2023_ARRAY_D_DS, "array.d.ts");
lib_source!(LIB_ES_ES2023_COLLECTION_D_DS, "collection.d.ts");
lib_source!(LIB_ES_ES2023_FULL_D_DS, "full.d.ts");
lib_source!(LIB_ES_ES2023_INDEX_D_DS, "index.d.ts");
lib_source!(LIB_ES_ES2023_INTL_D_DS, "intl.d.ts");

pub const LIB_ES2023: BuiltinLib = BuiltinLib::ambient_lib(
    "es2023",
    &[
        LIB_ES_ES2023_ARRAY_D_DS,
        LIB_ES_ES2023_COLLECTION_D_DS,
        LIB_ES_ES2023_INDEX_D_DS,
        LIB_ES_ES2023_INTL_D_DS,
    ],
    &["es2022"],
)
.with_declared_symbols(ES2023_DECLARED_SYMBOLS);

pub const LIB_ES2023_ARRAY: BuiltinLib =
    BuiltinLib::ambient_lib("es2023.array", &[LIB_ES_ES2023_ARRAY_D_DS], &[])
        .with_declared_symbols(ES2023_EMPTY_DECLARED_SYMBOLS);
pub const LIB_ES2023_COLLECTION: BuiltinLib =
    BuiltinLib::ambient_lib("es2023.collection", &[LIB_ES_ES2023_COLLECTION_D_DS], &[])
        .with_declared_symbols(ES2023_EMPTY_DECLARED_SYMBOLS);
pub const LIB_ES2023_FULL: BuiltinLib = BuiltinLib::ambient_lib(
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
.with_declared_symbols(ES2023_DECLARED_SYMBOLS);
pub const LIB_ES2023_INTL: BuiltinLib =
    BuiltinLib::ambient_lib("es2023.intl", &[LIB_ES_ES2023_INTL_D_DS], &[])
        .with_declared_symbols(ES2023_EMPTY_DECLARED_SYMBOLS);
