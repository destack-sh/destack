//! Structural type inference for JSON/TOML/YAML data.
//!
//! Converts `serde_json::Value` to DIR `Type` for type-safe data imports.

use destack_base::StringPool;
use destack_dir::{
    LocalNodeIdAny, LocalTypeId, PrimitiveType, StaticKey, Type, TypeField, TypeLiteral, TypeTable,
};
use serde_json::Value;

/// Convert a serde_json::Value to a DIR Type.
///
/// This is used to infer structural types for data module imports (JSON, TOML, YAML).
///
/// Type mappings:
/// - `null` → `null`
/// - `true`/`false` → `boolean`
/// - Numbers → `number` (always float64, JSON doesn't distinguish int/float)
/// - Strings → `string`
/// - Arrays → `T[]` or `(T | U | V)[]` for mixed element types
/// - Objects → `{ key1: T1, key2: T2, ... }`
pub(crate) fn json_value_to_type(
    value: &Value,
    source_id: LocalNodeIdAny,
    types: &mut TypeTable,
    strings: &StringPool,
) -> LocalTypeId {
    match value {
        Value::Null => types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Null,
            },
            source_id,
        ),

        Value::Bool(_) => types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            },
            source_id,
        ),

        Value::Number(_) => types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number),
            },
            source_id,
        ),

        Value::String(_) => types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            },
            source_id,
        ),

        Value::Array(items) => {
            let element_type = infer_array_element_type(items, source_id, types, strings);
            types.insert_type_from_any(
                Type::Array {
                    element: Some(element_type),
                },
                source_id,
            )
        }

        Value::Object(map) => {
            let fields = map
                .iter()
                .map(|(key, val)| {
                    let key_id = strings.intern(key);
                    let val_type = json_value_to_type(val, source_id, types, strings);
                    TypeField {
                        key: StaticKey::Name(key_id),
                        ty: val_type,
                        is_optional: false,
                        is_readonly: true, // data imports are immutable
                    }
                })
                .collect();

            types.insert_type_from_any(
                Type::Object {
                    fields,
                    call_signatures: vec![],
                    construct_signatures: vec![],
                    index_signatures: vec![],
                },
                source_id,
            )
        }
    }
}

/// Infer the element type for a JSON array, creating a union for mixed types.
///
/// Empty arrays get `unknown` element type.
/// Homogeneous arrays get the element type directly.
/// Mixed arrays get a union type: `(T | U | V)[]`.
fn infer_array_element_type(
    items: &[Value],
    source_id: LocalNodeIdAny,
    types: &mut TypeTable,
    strings: &StringPool,
) -> LocalTypeId {
    if items.is_empty() {
        // empty array → unknown element type
        return types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
            source_id,
        );
    }

    // collect element types, deduplicating by JSON value kind
    let mut seen_kinds = Vec::new();
    let mut element_types = Vec::new();

    for item in items {
        let kind = json_kind(item);
        if !seen_kinds.contains(&kind) {
            seen_kinds.push(kind);
            element_types.push(json_value_to_type(item, source_id, types, strings));
        }
    }

    if element_types.len() == 1 {
        element_types.into_iter().next().unwrap()
    } else {
        types.insert_type_from_any(
            Type::Union {
                elements: element_types,
            },
            source_id,
        )
    }
}

/// Get a discriminant for JSON value kinds for deduplication.
/// We use this to avoid creating duplicate types for values of the same kind.
fn json_kind(value: &Value) -> JsonKind {
    match value {
        Value::Null => JsonKind::Null,
        Value::Bool(_) => JsonKind::Boolean,
        Value::Number(_) => JsonKind::Number,
        Value::String(_) => JsonKind::String,
        Value::Array(_) => JsonKind::Array,
        Value::Object(_) => JsonKind::Object,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JsonKind {
    Null,
    Boolean,
    Number,
    String,
    Array,
    Object,
}

#[cfg(test)]
mod tests {
    use super::*;
    use destack_dir::NodeType;
    use destack_source::ModuleId;

    fn test_types() -> TypeTable {
        TypeTable::new(ModuleId::EPHEMERAL)
    }

    fn test_strings() -> StringPool {
        StringPool::new()
    }

    fn test_node_id() -> LocalNodeIdAny {
        LocalNodeIdAny::new(0, NodeType::Expression)
    }

    #[test]
    fn test_json_null() {
        let mut types = test_types();
        let strings = test_strings();
        let ty_id = json_value_to_type(&Value::Null, test_node_id(), &mut types, &strings);
        assert!(matches!(
            types.get_type(ty_id),
            Type::TypeLiteral {
                value: TypeLiteral::Null
            }
        ));
    }

    #[test]
    fn test_json_boolean() {
        let mut types = test_types();
        let strings = test_strings();
        let ty_id = json_value_to_type(&Value::Bool(true), test_node_id(), &mut types, &strings);
        assert!(matches!(
            types.get_type(ty_id),
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean)
            }
        ));
    }

    #[test]
    fn test_json_number() {
        let mut types = test_types();
        let strings = test_strings();
        let value = serde_json::json!(42);
        let ty_id = json_value_to_type(&value, test_node_id(), &mut types, &strings);
        assert!(matches!(
            types.get_type(ty_id),
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number)
            }
        ));
    }

    #[test]
    fn test_json_string() {
        let mut types = test_types();
        let strings = test_strings();
        let value = serde_json::json!("hello");
        let ty_id = json_value_to_type(&value, test_node_id(), &mut types, &strings);
        assert!(matches!(
            types.get_type(ty_id),
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String)
            }
        ));
    }

    #[test]
    fn test_json_array_homogeneous() {
        let mut types = test_types();
        let strings = test_strings();
        let value = serde_json::json!([1, 2, 3]);
        let ty_id = json_value_to_type(&value, test_node_id(), &mut types, &strings);
        let Type::Array { element: Some(elem_id) } = types.get_type(ty_id) else {
            panic!("expected array type");
        };
        assert!(matches!(
            types.get_type(*elem_id),
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number)
            }
        ));
    }

    #[test]
    fn test_json_array_mixed() {
        let mut types = test_types();
        let strings = test_strings();
        let value = serde_json::json!([1, "hello", true]);
        let ty_id = json_value_to_type(&value, test_node_id(), &mut types, &strings);
        let Type::Array { element: Some(elem_id) } = types.get_type(ty_id) else {
            panic!("expected array type");
        };
        let Type::Union { elements } = types.get_type(*elem_id) else {
            panic!("expected union type for mixed array");
        };
        assert_eq!(elements.len(), 3);
    }

    #[test]
    fn test_json_object() {
        let mut types = test_types();
        let strings = test_strings();
        let value = serde_json::json!({"name": "Alice", "age": 30});
        let ty_id = json_value_to_type(&value, test_node_id(), &mut types, &strings);
        let Type::Object { fields, .. } = types.get_type(ty_id) else {
            panic!("expected object type");
        };
        assert_eq!(fields.len(), 2);
        // all fields should be readonly
        assert!(fields.iter().all(|f| f.is_readonly));
    }
}
