use crate::error::GeneratorValidationIssue;
use crate::platform::model::{BindingCatalog, BindingType, CatalogBindingProvider};

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

/// Validate one normalized binding catalog and fail on invalid invariants.
pub(crate) fn validate_binding_catalog(catalog: &BindingCatalog) {
    let issues = collect_validation_issues(catalog);
    if issues.is_empty() {
        return;
    }

    // emit all collected issues before failing
    for issue in &issues {
        if let Some(binding) = issue.binding.as_deref() {
            eprintln!(
                "generator validation error [{}] {}::{}: {}",
                issue.code, issue.domain, binding, issue.message
            );
        } else {
            eprintln!(
                "generator validation error [{}] {}: {}",
                issue.code, issue.domain, issue.message
            );
        }
    }

    panic!("binding generation failed due to validation issues");
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

/// Collect all catalog validation issues.
fn collect_validation_issues(catalog: &BindingCatalog) -> Vec<GeneratorValidationIssue> {
    let mut issues = Vec::new();

    for (domain, bindings) in catalog {
        // enforce one provider family per module
        let has_host_provider = bindings
            .values()
            .any(|binding| binding.provider != CatalogBindingProvider::Runtime);
        let has_runtime_provider = bindings
            .values()
            .any(|binding| binding.provider == CatalogBindingProvider::Runtime);
        if has_host_provider && has_runtime_provider {
            issues.push(GeneratorValidationIssue::domain(
                "mixed_provider",
                domain,
                "module mixes host and runtime providers",
            ));
        }

        // validate each binding shape
        for (extern_name, binding) in bindings {
            if !extern_name.starts_with("destack.") {
                issues.push(GeneratorValidationIssue::binding(
                    "extern_prefix",
                    domain,
                    extern_name,
                    "binding id must start with `destack.`",
                ));
            }

            if binding.implementation_name.trim().is_empty() {
                issues.push(GeneratorValidationIssue::binding(
                    "empty_impl",
                    domain,
                    extern_name,
                    "implementation name is empty",
                ));
            }

            validate_binding_type(domain, extern_name, &binding.return_binding, &mut issues);
            for parameter in &binding.parameters {
                validate_binding_type(domain, extern_name, &parameter.binding_type, &mut issues);
            }
        }
    }

    issues
}

/// Validate one binding type recursively.
fn validate_binding_type(
    domain: &str,
    extern_name: &str,
    binding_type: &BindingType,
    issues: &mut Vec<GeneratorValidationIssue>,
) {
    match binding_type {
        BindingType::TaggedUnion { name, variants, .. } => {
            if variants.is_empty() {
                issues.push(GeneratorValidationIssue::binding(
                    "empty_tagged_union",
                    domain,
                    extern_name,
                    format!("tagged union `{name}` has no variants"),
                ));
            }

            let mut seen = std::collections::BTreeSet::<&str>::new();
            for variant in variants {
                if !seen.insert(variant.name.as_str()) {
                    issues.push(GeneratorValidationIssue::binding(
                        "duplicate_variant",
                        domain,
                        extern_name,
                        format!(
                            "tagged union `{name}` contains duplicate variant `{}`",
                            variant.name
                        ),
                    ));
                }

                validate_binding_type(domain, extern_name, &variant.binding_type, issues);
            }
        }
        BindingType::Optional(inner)
        | BindingType::Slice(inner)
        | BindingType::Array(inner)
        | BindingType::Newtype { inner, .. } => {
            validate_binding_type(domain, extern_name, inner, issues);
        }
        BindingType::Struct { fields, .. } => {
            for field in fields {
                validate_binding_type(domain, extern_name, &field.binding_type, issues);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::error::GeneratorValidationIssue;
    use crate::platform::model::{
        BindingCatalog, BindingEntry, BindingField, BindingParameter, BindingTaggedUnionVariant,
        BindingType, CatalogBindingAffinity, CatalogBindingProvider, CatalogBindingReplayKind,
        CatalogBindingSimulation, CatalogEffect, CatalogReplayPayload,
    };

    use super::{collect_validation_issues, normalize_binding_catalog};

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
                effect: CatalogEffect::Deterministic,
                replay_kind: CatalogBindingReplayKind::BindingCall,
                replay_payload: CatalogReplayPayload::ResultsOnly,
                requires: vec!["test.read".to_string()],
                platforms: vec!["linux".to_string()],
                hosts: Vec::new(),
                provider: CatalogBindingProvider::Runtime,
                affinity: CatalogBindingAffinity::None,
                simulation: CatalogBindingSimulation::Unsupported,
            },
        );
        catalog.insert("test".to_string(), domain);

        let catalog = normalize_binding_catalog(catalog);
        let binding = catalog
            .get("test")
            .and_then(|domain| domain.get("destack.test.binding"))
            .unwrap_or_else(|| panic!("binding should exist"));
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
                effect: CatalogEffect::Deterministic,
                replay_kind: CatalogBindingReplayKind::BindingCall,
                replay_payload: CatalogReplayPayload::ResultsOnly,
                requires: vec!["test.read".to_string()],
                platforms: vec!["linux".to_string()],
                hosts: Vec::new(),
                provider: CatalogBindingProvider::Runtime,
                affinity: CatalogBindingAffinity::None,
                simulation: CatalogBindingSimulation::Unsupported,
            },
        );
        catalog.insert("test".to_string(), domain);

        let catalog = normalize_binding_catalog(catalog);
        let binding = catalog
            .get("test")
            .and_then(|domain| domain.get("destack.test.binding"))
            .unwrap_or_else(|| panic!("binding should exist"));
        let binding_type = &binding.parameters[0].binding_type;

        assert_eq!(
            binding_type,
            &BindingType::Optional(Box::new(BindingType::String))
        );
    }

    /// Reject modules that mix host and runtime providers.
    #[test]
    fn test_collect_validation_issues_reports_mixed_provider() {
        let mut catalog: BindingCatalog = BTreeMap::new();
        let mut domain = BTreeMap::new();
        domain.insert(
            "destack.test.host".to_string(),
            minimal_binding(CatalogBindingProvider::Host),
        );
        domain.insert(
            "destack.test.runtime".to_string(),
            minimal_binding(CatalogBindingProvider::Runtime),
        );
        catalog.insert("test".to_string(), domain);

        let issues = collect_validation_issues(&catalog);
        assert!(issues.iter().any(|issue| issue.code == "mixed_provider"));
    }

    /// Reject tagged unions that contain no variants.
    #[test]
    fn test_collect_validation_issues_reports_empty_tagged_union() {
        let mut catalog: BindingCatalog = BTreeMap::new();
        let mut domain = BTreeMap::new();
        let mut binding = minimal_binding(CatalogBindingProvider::Runtime);
        binding.return_binding = BindingType::TaggedUnion {
            name: "Empty".to_string(),
            domain: "test".to_string(),
            variants: vec![],
        };
        domain.insert("destack.test.runtime".to_string(), binding);
        catalog.insert("test".to_string(), domain);

        let issues = collect_validation_issues(&catalog);
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "empty_tagged_union")
        );
    }

    /// Reject tagged unions that contain duplicate variants.
    #[test]
    fn test_collect_validation_issues_reports_duplicate_variants() {
        let mut catalog: BindingCatalog = BTreeMap::new();
        let mut domain = BTreeMap::new();
        let mut binding = minimal_binding(CatalogBindingProvider::Runtime);
        binding.return_binding = BindingType::TaggedUnion {
            name: "Duplicate".to_string(),
            domain: "test".to_string(),
            variants: vec![
                BindingTaggedUnionVariant {
                    name: "Variant".to_string(),
                    binding_type: BindingType::Void,
                },
                BindingTaggedUnionVariant {
                    name: "Variant".to_string(),
                    binding_type: BindingType::Void,
                },
            ],
        };
        domain.insert("destack.test.runtime".to_string(), binding);
        catalog.insert("test".to_string(), domain);

        let issues = collect_validation_issues(&catalog);
        assert!(issues.iter().any(|issue| issue.code == "duplicate_variant"));
    }

    fn minimal_binding(provider: CatalogBindingProvider) -> BindingEntry {
        BindingEntry {
            implementation_name: "binding".to_string(),
            documentation: None,
            signature: "function binding()".to_string(),
            parameters: vec![],
            return_binding: BindingType::TaggedUnion {
                name: "Example".to_string(),
                domain: "test".to_string(),
                variants: vec![BindingTaggedUnionVariant {
                    name: "Variant".to_string(),
                    binding_type: BindingType::Void,
                }],
            },
            return_is_result: true,
            effect: CatalogEffect::Deterministic,
            replay_kind: CatalogBindingReplayKind::BindingCall,
            replay_payload: CatalogReplayPayload::ResultsOnly,
            requires: vec!["test.read".to_string()],
            platforms: vec!["linux".to_string()],
            hosts: Vec::new(),
            provider,
            affinity: CatalogBindingAffinity::None,
            simulation: CatalogBindingSimulation::Unsupported,
        }
    }

    #[allow(dead_code)]
    fn _assert_issue_type(issue: &GeneratorValidationIssue) -> &str {
        issue.code
    }
}
