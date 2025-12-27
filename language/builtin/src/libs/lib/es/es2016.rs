use super::super::super::source::{BuiltinLib, BuiltinLibSource};

macro_rules! lib_source {
    ($name:ident, $file:literal) => {
        const $name: BuiltinLibSource = BuiltinLibSource::new(
            "lib",
            "es/es2016",
            $file,
            include_str!(concat!("../../../../lib/es/es2016/", $file)),
        );
    };
}

lib_source!(LIB_ES_ES2016_ARRAY_INCLUDE_D_DS, "array.include.d.ds");
lib_source!(LIB_ES_ES2016_INTL_D_DS, "intl.d.ds");

pub const LIB_ES2016: BuiltinLib = BuiltinLib::ambient(
    "es2016",
    &[LIB_ES_ES2016_ARRAY_INCLUDE_D_DS, LIB_ES_ES2016_INTL_D_DS],
    &["es2015"],
);

pub const LIB_ES2016_ARRAY_INCLUDE: BuiltinLib = BuiltinLib::ambient(
    "es2016.array.include",
    &[LIB_ES_ES2016_ARRAY_INCLUDE_D_DS],
    &[],
);
pub const LIB_ES2016_INTL: BuiltinLib =
    BuiltinLib::ambient("es2016.intl", &[LIB_ES_ES2016_INTL_D_DS], &[]);
