use crate::error::GeneratorValidationIssue;
use crate::model::{BindingCatalog, BindingType, CatalogBindingScope};

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

/// Collect all catalog validation issues.
fn collect_validation_issues(catalog: &BindingCatalog) -> Vec<GeneratorValidationIssue> {
    let mut issues = Vec::new();

    for (domain, bindings) in catalog {
        // enforce one scope family per module
        let has_host_scope = bindings
            .values()
            .any(|binding| binding.scope != CatalogBindingScope::Runtime);
        let has_runtime_scope = bindings
            .values()
            .any(|binding| binding.scope == CatalogBindingScope::Runtime);
        if has_host_scope && has_runtime_scope {
            issues.push(GeneratorValidationIssue::domain(
                "mixed_scope",
                domain,
                "module mixes host and runtime scopes",
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
    use crate::model::{
        BindingCatalog, BindingEntry, BindingTaggedUnionVariant, BindingType,
        CatalogBindingBlocking, CatalogBindingReplayKind, CatalogBindingScope, CatalogEffectClass,
        CatalogReplayPayload,
    };

    use super::collect_validation_issues;

    /// Reject modules that mix host and runtime scopes.
    #[test]
    fn test_collect_validation_issues_reports_mixed_scope() {
        let mut catalog: BindingCatalog = BTreeMap::new();
        let mut domain = BTreeMap::new();
        domain.insert(
            "destack.test.host".to_string(),
            minimal_binding(CatalogBindingScope::Host),
        );
        domain.insert(
            "destack.test.runtime".to_string(),
            minimal_binding(CatalogBindingScope::Runtime),
        );
        catalog.insert("test".to_string(), domain);

        let issues = collect_validation_issues(&catalog);
        assert!(issues.iter().any(|issue| issue.code == "mixed_scope"));
    }

    /// Reject tagged unions that contain no variants.
    #[test]
    fn test_collect_validation_issues_reports_empty_tagged_union() {
        let mut catalog: BindingCatalog = BTreeMap::new();
        let mut domain = BTreeMap::new();
        let mut binding = minimal_binding(CatalogBindingScope::Runtime);
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
        let mut binding = minimal_binding(CatalogBindingScope::Runtime);
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

    fn minimal_binding(scope: CatalogBindingScope) -> BindingEntry {
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
            effect_class: CatalogEffectClass::Deterministic,
            replay_kind: CatalogBindingReplayKind::Regular,
            replay_payload: CatalogReplayPayload::ResultsOnly,
            requires: vec!["test.read".to_string()],
            host_platforms: vec!["linux".to_string()],
            scope,
            blocking: CatalogBindingBlocking::Never,
        }
    }

    #[allow(dead_code)]
    fn _assert_issue_type(issue: &GeneratorValidationIssue) -> &str {
        issue.code
    }
}
