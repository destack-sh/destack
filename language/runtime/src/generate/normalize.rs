use crate::model::{BindingCatalog, BindingType};

/// Normalize one binding catalog into one deterministic canonical shape.
pub(crate) fn normalize_binding_catalog(mut catalog: BindingCatalog) -> BindingCatalog {
    for bindings in catalog.values_mut() {
        for entry in bindings.values_mut() {
            // normalize parameter types
            for parameter in &mut entry.parameters {
                normalize_binding_type(&mut parameter.binding_type);
            }

            // normalize return type
            normalize_binding_type(&mut entry.return_binding);
        }
    }

    catalog
}

/// Normalize one binding type recursively.
fn normalize_binding_type(binding_type: &mut BindingType) {
    match binding_type {
        // normalize wrapped values recursively
        BindingType::Optional(inner) => {
            normalize_binding_type(inner);
            normalize_optional_binding_type(inner);
        }

        // normalize wrapped collection and nominal values recursively
        BindingType::Slice(inner)
        | BindingType::Array(inner)
        | BindingType::Newtype { inner, .. } => {
            normalize_binding_type(inner);
        }

        // normalize each struct field type
        BindingType::Struct { fields, .. } => {
            for field in fields {
                normalize_binding_type(&mut field.binding_type);
            }
        }

        // normalize and deterministically sort tagged union variants
        BindingType::TaggedUnion { variants, .. } => {
            for variant in variants.iter_mut() {
                normalize_binding_type(&mut variant.binding_type);
            }

            variants.sort_by(|left, right| left.name.cmp(&right.name));
        }

        _ => {}
    }
}

/// Normalize one optional type wrapper into a canonical shape.
fn normalize_optional_binding_type(inner: &mut Box<BindingType>) {
    loop {
        let BindingType::Optional(_) = inner.as_ref() else {
            break;
        };

        let nested = std::mem::replace(inner, Box::new(BindingType::Void));
        let BindingType::Optional(nested_inner) = *nested else {
            break;
        };
        *inner = nested_inner;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::model::{
        BindingCatalog, BindingEntry, BindingField, BindingParameter, BindingTaggedUnionVariant,
        BindingType, CatalogBindingBlocking, CatalogBindingReplayKind, CatalogBindingScope,
        CatalogEffectClass, CatalogReplayPayload,
    };

    use super::normalize_binding_catalog;

    /// Keep tagged union variant ordering deterministic by variant name.
    #[test]
    fn test_normalize_binding_catalog_sorts_tagged_union_variants() {
        let mut catalog: BindingCatalog = BTreeMap::new();
        let mut domain = BTreeMap::new();
        domain.insert(
            "destack.test.binding".to_string(),
            BindingEntry {
                implementation_name: "binding".to_string(),
                documentation: None,
                signature: "function binding()".to_string(),
                parameters: vec![BindingParameter {
                    name: "value".to_string(),
                    type_text: None,
                    binding_type: BindingType::TaggedUnion {
                        name: "Example".to_string(),
                        domain: "test".to_string(),
                        variants: vec![
                            BindingTaggedUnionVariant {
                                name: "B".to_string(),
                                binding_type: BindingType::Struct {
                                    name: "B".to_string(),
                                    domain: "test".to_string(),
                                    fields: vec![BindingField {
                                        name: "kind".to_string(),
                                        documentation: None,
                                        binding_type: BindingType::String,
                                    }],
                                },
                            },
                            BindingTaggedUnionVariant {
                                name: "A".to_string(),
                                binding_type: BindingType::Struct {
                                    name: "A".to_string(),
                                    domain: "test".to_string(),
                                    fields: vec![BindingField {
                                        name: "kind".to_string(),
                                        documentation: None,
                                        binding_type: BindingType::String,
                                    }],
                                },
                            },
                        ],
                    },
                }],
                return_binding: BindingType::Void,
                return_is_result: true,
                effect_class: CatalogEffectClass::Deterministic,
                replay_kind: CatalogBindingReplayKind::BindingCall,
                replay_payload: CatalogReplayPayload::ResultsOnly,
                requires: vec!["test.read".to_string()],
                host_platforms: vec!["linux".to_string()],
                scope: CatalogBindingScope::Runtime,
                blocking: CatalogBindingBlocking::Never,
            },
        );
        catalog.insert("test".to_string(), domain);

        let catalog = normalize_binding_catalog(catalog);
        let binding = catalog
            .get("test")
            .and_then(|domain| domain.get("destack.test.binding"))
            .expect("binding should exist");
        let BindingType::TaggedUnion { variants, .. } = &binding.parameters[0].binding_type else {
            panic!("expected tagged union parameter");
        };

        assert_eq!(variants[0].name, "A");
        assert_eq!(variants[1].name, "B");
    }

    /// Collapse nested optional wrappers into one canonical optional layer.
    #[test]
    fn test_normalize_binding_catalog_collapses_nested_optional() {
        let mut catalog: BindingCatalog = BTreeMap::new();
        let mut domain = BTreeMap::new();
        domain.insert(
            "destack.test.binding".to_string(),
            BindingEntry {
                implementation_name: "binding".to_string(),
                documentation: None,
                signature: "function binding(value: Test)".to_string(),
                parameters: vec![BindingParameter {
                    name: "value".to_string(),
                    type_text: Some("Test".to_string()),
                    binding_type: BindingType::Optional(Box::new(BindingType::Optional(Box::new(
                        BindingType::String,
                    )))),
                }],
                return_binding: BindingType::Void,
                return_is_result: true,
                effect_class: CatalogEffectClass::Deterministic,
                replay_kind: CatalogBindingReplayKind::BindingCall,
                replay_payload: CatalogReplayPayload::ResultsOnly,
                requires: vec!["test.read".to_string()],
                host_platforms: vec!["linux".to_string()],
                scope: CatalogBindingScope::Runtime,
                blocking: CatalogBindingBlocking::Never,
            },
        );
        catalog.insert("test".to_string(), domain);

        let catalog = normalize_binding_catalog(catalog);
        let binding = catalog
            .get("test")
            .and_then(|domain| domain.get("destack.test.binding"))
            .expect("binding should exist");
        let binding_type = &binding.parameters[0].binding_type;

        assert_eq!(
            binding_type,
            &BindingType::Optional(Box::new(BindingType::String))
        );
    }
}
