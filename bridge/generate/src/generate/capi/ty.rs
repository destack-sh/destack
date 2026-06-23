use crate::generate::core::to_snake;
use crate::generate::schema::{ModulePath, Type};

use super::projection::Projection;

/// Return one generated C type name.
pub(super) fn c_type_name(name: &str) -> String {
    format!("Destack{name}")
}

/// Render one C declaration fragment.
pub(super) fn c_declaration(name: &str, ty: &Type, projection: &Projection<'_>) -> String {
    let ty = c_type(ty, projection);
    if ty.ends_with('*') {
        format!("{ty}{name}")
    } else {
        format!("{ty} {name}")
    }
}

/// Render one C type.
fn c_type(ty: &Type, projection: &Projection<'_>) -> String {
    match ty {
        Type::String => "char *".to_string(),
        Type::Bool => "bool".to_string(),
        Type::Char => "uint32_t".to_string(),
        Type::U8 => "uint8_t".to_string(),
        Type::U32 => "uint32_t".to_string(),
        Type::U64 => "uint64_t".to_string(),
        Type::U128 => "DestackU128".to_string(),
        Type::Signed(_) => "int32_t".to_string(),
        Type::Float(32) => "float".to_string(),
        Type::Float(_) => "double".to_string(),
        Type::Usize => "size_t".to_string(),
        Type::Vec(inner) if **inner == Type::U8 => "DestackByteArray".to_string(),
        Type::Vec(inner) if **inner == Type::String => "DestackStringArray".to_string(),
        Type::Vec(inner) => format!("Destack{}Array", named_type(inner)),
        Type::Option(inner) if **inner == Type::String => "DestackOptionalString".to_string(),
        Type::Option(inner) => format!("DestackOptional{}", named_type(inner)),
        Type::Named(name) if projection.is_handle(name) => format!("Destack{name} *"),
        Type::Named(name) => c_type_name(name),
        Type::Array(_, _) | Type::Tuple(_) | Type::Map(_, _) | Type::Json => {
            ty.unsupported_bridge_type()
        }
    }
}

/// Return one named type inside a collection.
pub(super) fn named_type(ty: &Type) -> &str {
    let Type::Named(name) = ty else {
        ty.unsupported_bridge_type();
    };

    name
}

/// Return one C enum variant name.
pub(super) fn c_enum_variant(ty: &str, variant: &str) -> String {
    let ty = to_snake(ty).to_ascii_uppercase();
    let variant = to_snake(variant).to_ascii_uppercase();

    format!("DESTACK_{ty}_{variant}")
}

/// Return one generated C header guard.
pub(super) fn header_guard(path: &ModulePath) -> String {
    let path = path
        .segments()
        .iter()
        .map(|segment| to_snake(segment).to_ascii_uppercase())
        .collect::<Vec<_>>()
        .join("_");

    format!("DESTACK_{path}_GENERATED_H")
}
