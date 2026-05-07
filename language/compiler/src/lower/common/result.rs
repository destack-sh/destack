use std::collections::HashSet;

use destack_core::StringPool;
use destack_dir as dir;

/// Result union metadata resolved from a type id.
#[derive(Debug, Clone)]
pub(crate) struct ResultUnionInfo {
    /// Union type id for the Result.
    pub(crate) union_type: dir::LocalTypeId,
    /// Ok variant type id.
    pub(crate) ok_type: dir::LocalTypeId,
    /// Err variant type id.
    pub(crate) err_type: dir::LocalTypeId,
    /// Ok value type id.
    pub(crate) ok_value_type: dir::LocalTypeId,
    /// Err value type id.
    pub(crate) err_value_type: dir::LocalTypeId,
}

/// Resolve the Ok/Err union metadata for a Result-like type.
pub(crate) fn resolve_result_union(
    types: &dir::TypeTable,
    strings: &StringPool,
    type_id: dir::LocalTypeId,
) -> Option<ResultUnionInfo> {
    let union_type = resolve_union_type_id(types, type_id)?;
    let dir::Type::Union(union) = types.get_type(union_type) else {
        return None;
    };

    let mut ok_variant = None;
    let mut err_variant = None;
    for element in &union.elements {
        if let Some(variant) = result_variant_info(types, strings, *element) {
            match variant.kind {
                ResultVariantKind::Ok => ok_variant = Some(variant),
                ResultVariantKind::Err => err_variant = Some(variant),
            }
        }
    }

    let (Some(ok_variant), Some(err_variant)) = (ok_variant, err_variant) else {
        return None;
    };

    Some(ResultUnionInfo {
        union_type,
        ok_type: ok_variant.struct_type,
        err_type: err_variant.struct_type,
        ok_value_type: ok_variant.value_type,
        err_value_type: err_variant.value_type,
    })
}

/// Return true when a type id resolves to void.
pub(crate) fn is_void_type(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    let mut visited = HashSet::new();
    is_void_type_inner(types, type_id, &mut visited)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResultVariantKind {
    Ok,
    Err,
}

#[derive(Debug, Clone)]
struct ResultVariantInfo {
    struct_type: dir::LocalTypeId,
    value_type: dir::LocalTypeId,
    kind: ResultVariantKind,
}

fn resolve_union_type_id(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
) -> Option<dir::LocalTypeId> {
    let mut visited = HashSet::new();
    resolve_union_type_id_inner(types, type_id, &mut visited)
}

fn resolve_union_type_id_inner(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    visited: &mut HashSet<dir::LocalTypeId>,
) -> Option<dir::LocalTypeId> {
    if !visited.insert(type_id) {
        return None;
    }

    match types.get_type(type_id) {
        dir::Type::Union(_) => Some(type_id),
        dir::Type::Reference(reference) => {
            if let Some(target) = types.get_alias_target_type_id(reference.symbol) {
                return resolve_union_type_id_inner(types, target, visited);
            }

            if let Some(instance) = types.get_instance_type_id(reference.symbol) {
                return resolve_union_type_id_inner(types, instance, visited);
            }

            None
        }
        dir::Type::Value(value) => resolve_union_type_id_inner(types, value.value, visited),
        _ => None,
    }
}

fn is_void_type_inner(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    visited: &mut HashSet<dir::LocalTypeId>,
) -> bool {
    if !visited.insert(type_id) {
        return false;
    }

    match types.get_type(type_id) {
        dir::Type::Literal(dir::LiteralType {
            value: dir::TypeLiteral::Void,
        }) => true,
        dir::Type::Reference(reference) => {
            if let Some(target) = types.get_alias_target_type_id(reference.symbol) {
                return is_void_type_inner(types, target, visited);
            }
            if let Some(instance) = types.get_instance_type_id(reference.symbol) {
                return is_void_type_inner(types, instance, visited);
            }
            false
        }
        dir::Type::Value(value) => is_void_type_inner(types, value.value, visited),
        _ => false,
    }
}

fn result_variant_info(
    types: &dir::TypeTable,
    strings: &StringPool,
    type_id: dir::LocalTypeId,
) -> Option<ResultVariantInfo> {
    let fields = resolve_object_fields(types, type_id)?;

    let kind_key = dir::StaticKey::Name(strings.intern("kind"));
    let value_key = dir::StaticKey::Name(strings.intern("value"));
    let error_key = dir::StaticKey::Name(strings.intern("error"));
    let ok_literal = strings.intern("ok");
    let err_literal = strings.intern("err");

    let mut kind_literal = None;
    let mut value_type = None;
    let mut error_type = None;

    for field in &fields {
        if field.key == kind_key
            && let dir::Type::Literal(dir::LiteralType {
                value: dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::String(literal)),
            }) = types.get_type(field.ty)
        {
            kind_literal = Some(*literal);
        }
        if field.key == value_key {
            value_type = Some(field.ty);
        }
        if field.key == error_key {
            error_type = Some(field.ty);
        }
    }

    let kind_literal = kind_literal?;

    if kind_literal == ok_literal {
        let value_type = value_type?;
        Some(ResultVariantInfo {
            struct_type: type_id,
            value_type,
            kind: ResultVariantKind::Ok,
        })
    } else if kind_literal == err_literal {
        let value_type = error_type?;
        Some(ResultVariantInfo {
            struct_type: type_id,
            value_type,
            kind: ResultVariantKind::Err,
        })
    } else {
        None
    }
}

fn resolve_object_fields(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
) -> Option<Vec<dir::TypeField>> {
    let mut visited = HashSet::new();
    resolve_object_fields_inner(types, type_id, &mut visited)
}

fn resolve_object_fields_inner(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    visited: &mut HashSet<dir::LocalTypeId>,
) -> Option<Vec<dir::TypeField>> {
    if !visited.insert(type_id) {
        return None;
    }

    match types.get_type(type_id) {
        dir::Type::Object(object) => Some(object.fields.clone()),
        dir::Type::Reference(reference) => {
            if let Some(target) = types.get_alias_target_type_id(reference.symbol) {
                return resolve_object_fields_inner(types, target, visited);
            }
            if let Some(instance) = types.get_instance_type_id(reference.symbol) {
                return resolve_object_fields_inner(types, instance, visited);
            }
            None
        }
        dir::Type::Value(value) => resolve_object_fields_inner(types, value.value, visited),
        _ => None,
    }
}
