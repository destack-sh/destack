use std::collections::BTreeMap;

use crate::runtime::bindings::{
    BindingAffinity, BindingBlocking, BindingDescriptor, BindingEffect, BindingEffectClass,
    BindingEngine, BindingScope,
};
use destack_source::matches as glob_matches;
use destack_workspace as workspace;
use destack_workspace::{ExecutionMode, RuntimeLabelOperator, RuntimeSelector};

/// Return true when one selector matches one runtime call selector context.
pub(crate) fn selector_matches(
    selector: &RuntimeSelector,
    runtime_name: &str,
    runtime_labels: &BTreeMap<String, String>,
    worker_name: &str,
    worker_labels: &BTreeMap<String, String>,
    descriptor: Option<BindingDescriptor>,
    mode: ExecutionMode,
    engine: Option<BindingEngine>,
) -> bool {
    // descriptor-aware selectors require one descriptor
    if descriptor.is_none() && selector_has_binding_clauses(selector) {
        return false;
    }

    // match runtime identity selector
    if let Some(runtime_selector) = &selector.runtime
        && !matches_identity_selector(runtime_selector, runtime_name, runtime_labels)
    {
        return false;
    }

    // match worker identity selector
    if let Some(worker_selector) = &selector.worker
        && !matches_identity_selector(worker_selector, worker_name, worker_labels)
    {
        return false;
    }

    // match engine selector
    if let Some(rule_engine) = selector.engine {
        let Some(engine) = engine else {
            return false;
        };
        if !matches_engine(rule_engine, engine) {
            return false;
        }
    }

    // match execution mode selector
    if let Some(modes) = &selector.execution_modes {
        let matches_mode = modes.iter().copied().any(|rule_mode| rule_mode == mode);
        if !matches_mode {
            return false;
        }
    }

    // match platform selector
    if let Some(platforms) = &selector.platforms {
        let current_platform = runtime_platform_name();
        let matches_platform = platforms
            .iter()
            .any(|pattern| glob_match(pattern, current_platform));
        if !matches_platform {
            return false;
        }
    }

    // match descriptor-aware selectors
    if let Some(descriptor) = descriptor {
        if let Some(pattern) = &selector.binding
            && !glob_match(pattern, descriptor.name)
        {
            return false;
        }

        if selector.function.is_some() {
            return false;
        }

        if let Some(pattern) = &selector.module {
            let module_name = descriptor
                .name
                .strip_prefix("destack.")
                .unwrap_or(descriptor.name);
            if !glob_match(pattern, module_name) {
                return false;
            }
        }

        if let Some(pattern) = &selector.component {
            let component_name = binding_component_name(descriptor.name);
            if !glob_match(pattern, component_name) {
                return false;
            }
        }

        if let Some(pattern) = &selector.capability {
            let has_match = descriptor
                .requires()
                .iter()
                .any(|capability| glob_match(pattern, capability));
            if !has_match {
                return false;
            }
        }

        if let Some(scope) = selector.scope
            && !matches_scope(scope, descriptor.scope())
        {
            return false;
        }

        if let Some(blocking) = selector.blocking
            && !matches_blocking(blocking, descriptor.blocking())
        {
            return false;
        }

        if let Some(affinity) = selector.affinity
            && !matches_affinity(affinity, descriptor.affinity())
        {
            return false;
        }

        if let Some(effect) = selector.effect
            && !matches_effect(effect, descriptor.effect_class)
        {
            return false;
        }
    }

    true
}

/// Return true when one selector depends on descriptor fields.
fn selector_has_binding_clauses(selector: &RuntimeSelector) -> bool {
    selector.binding.is_some()
        || selector.function.is_some()
        || selector.capability.is_some()
        || selector.component.is_some()
        || selector.module.is_some()
        || selector.scope.is_some()
        || selector.blocking.is_some()
        || selector.affinity.is_some()
        || selector.effect.is_some()
}

/// Return true when one identity selector matches name and labels.
fn matches_identity_selector(
    selector: &workspace::RuntimeIdentitySelector,
    name: &str,
    labels: &BTreeMap<String, String>,
) -> bool {
    if let Some(name_pattern) = &selector.name
        && !glob_match(name_pattern, name)
    {
        return false;
    }

    if let Some(label_selector) = &selector.labels
        && !matches_label_selector(label_selector, labels)
    {
        return false;
    }

    true
}

/// Return true when one label selector matches labels.
fn matches_label_selector(
    selector: &workspace::RuntimeLabelSelector,
    labels: &BTreeMap<String, String>,
) -> bool {
    for (key, expected_value) in &selector.match_labels {
        let Some(actual_value) = labels.get(key) else {
            return false;
        };
        if actual_value != expected_value {
            return false;
        }
    }

    for requirement in &selector.match_expressions {
        if !matches_label_requirement(requirement, labels) {
            return false;
        }
    }

    true
}

/// Return true when one label requirement matches labels.
fn matches_label_requirement(
    requirement: &workspace::RuntimeLabelRequirement,
    labels: &BTreeMap<String, String>,
) -> bool {
    match requirement.operator {
        RuntimeLabelOperator::In => {
            let Some(actual_value) = labels.get(&requirement.key) else {
                return false;
            };
            requirement.values.iter().any(|value| value == actual_value)
        }
        RuntimeLabelOperator::NotIn => {
            let Some(actual_value) = labels.get(&requirement.key) else {
                return false;
            };
            !requirement.values.iter().any(|value| value == actual_value)
        }
        RuntimeLabelOperator::Exists => labels.contains_key(&requirement.key),
        RuntimeLabelOperator::DoesNotExist => !labels.contains_key(&requirement.key),
    }
}

/// Return true when one engine selector matches the call engine.
fn matches_engine(rule_engine: BindingEngine, engine: BindingEngine) -> bool {
    rule_engine == engine
}

/// Return true when one scope selector matches the binding scope.
fn matches_scope(rule_scope: BindingScope, scope: BindingScope) -> bool {
    rule_scope == scope
}

/// Return true when one blocking selector matches the binding class.
fn matches_blocking(rule_blocking: BindingBlocking, blocking: BindingBlocking) -> bool {
    rule_blocking == blocking
}

/// Return true when one affinity selector matches the binding class.
fn matches_affinity(rule_affinity: BindingAffinity, affinity: BindingAffinity) -> bool {
    rule_affinity == affinity
}

/// Return true when one effect selector matches the binding effect class.
fn matches_effect(rule_effect: BindingEffect, effect_class: BindingEffectClass) -> bool {
    rule_effect == effect_class.binding_effect()
}

/// Match one text value against one glob pattern.
fn glob_match(pattern: &str, text: &str) -> bool {
    glob_matches(pattern.as_bytes(), 0, text.as_bytes(), 0)
}

/// Return the component segment for one binding id.
fn binding_component_name(binding_name: &str) -> &str {
    let stripped_name = binding_name
        .strip_prefix("destack.")
        .unwrap_or(binding_name);
    let mut component_segments = stripped_name.splitn(2, '.');
    component_segments.next().unwrap_or("unknown")
}

/// Return the current compile target platform name.
fn runtime_platform_name() -> &'static str {
    #[cfg(target_os = "linux")]
    {
        "linux"
    }

    #[cfg(target_os = "macos")]
    {
        "macos"
    }

    #[cfg(target_os = "ios")]
    {
        "ios"
    }

    #[cfg(target_os = "windows")]
    {
        "windows"
    }

    #[cfg(target_os = "wasi")]
    {
        return "wasi";
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "ios",
        target_os = "windows",
        target_os = "wasi"
    )))]
    {
        "unknown"
    }
}
