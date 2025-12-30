use super::super::super::source::{BuiltinLib, BuiltinLibSource};

macro_rules! lib_source {
    ($name:ident, $file:literal) => {
        const $name: BuiltinLibSource = BuiltinLibSource::new(
            "lib",
            "es/es2017",
            $file,
            include_str!(concat!("../../../../lib/es/es2017/", $file)),
        );
    };
}

lib_source!(LIB_ES_ES2017_DATE_D_DS, "date.d.ds");
lib_source!(LIB_ES_ES2017_ARRAYBUFFER_D_DS, "arraybuffer.d.ds");
lib_source!(LIB_ES_ES2017_FULL_D_DS, "full.d.ds");
lib_source!(LIB_ES_ES2017_INDEX_D_DS, "index.d.ds");
lib_source!(LIB_ES_ES2017_INTL_D_DS, "intl.d.ds");
lib_source!(LIB_ES_ES2017_OBJECT_D_DS, "object.d.ds");
lib_source!(LIB_ES_ES2017_SHAREDMEMORY_D_DS, "sharedmemory.d.ds");
lib_source!(LIB_ES_ES2017_STRING_D_DS, "string.d.ds");
lib_source!(LIB_ES_ES2017_TYPEDARRAYS_D_DS, "typedarrays.d.ds");

pub const LIB_ES2017: BuiltinLib = BuiltinLib::ambient(
    "es2017",
    &[
        LIB_ES_ES2017_ARRAYBUFFER_D_DS,
        LIB_ES_ES2017_DATE_D_DS,
        LIB_ES_ES2017_INDEX_D_DS,
        LIB_ES_ES2017_INTL_D_DS,
        LIB_ES_ES2017_OBJECT_D_DS,
        LIB_ES_ES2017_SHAREDMEMORY_D_DS,
        LIB_ES_ES2017_STRING_D_DS,
        LIB_ES_ES2017_TYPEDARRAYS_D_DS,
    ],
    &["es2016"],
);

pub const LIB_ES2017_DATE: BuiltinLib =
    BuiltinLib::ambient("es2017.date", &[LIB_ES_ES2017_DATE_D_DS], &[]);
pub const LIB_ES2017_ARRAYBUFFER: BuiltinLib =
    BuiltinLib::ambient("es2017.arraybuffer", &[LIB_ES_ES2017_ARRAYBUFFER_D_DS], &[]);
pub const LIB_ES2017_FULL: BuiltinLib = BuiltinLib::ambient(
    "es2017.full",
    &[LIB_ES_ES2017_FULL_D_DS],
    &[
        "es2017",
        "dom",
        "worker.importscripts",
        "scripthost",
        "dom.iterable",
    ],
);
pub const LIB_ES2017_INTL: BuiltinLib =
    BuiltinLib::ambient("es2017.intl", &[LIB_ES_ES2017_INTL_D_DS], &[]);
pub const LIB_ES2017_OBJECT: BuiltinLib =
    BuiltinLib::ambient("es2017.object", &[LIB_ES_ES2017_OBJECT_D_DS], &[]);
pub const LIB_ES2017_SHAREDMEMORY: BuiltinLib = BuiltinLib::ambient(
    "es2017.sharedmemory",
    &[LIB_ES_ES2017_SHAREDMEMORY_D_DS],
    &[],
);
pub const LIB_ES2017_STRING: BuiltinLib =
    BuiltinLib::ambient("es2017.string", &[LIB_ES_ES2017_STRING_D_DS], &[]);
pub const LIB_ES2017_TYPEDARRAYS: BuiltinLib =
    BuiltinLib::ambient("es2017.typedarrays", &[LIB_ES_ES2017_TYPEDARRAYS_D_DS], &[]);
