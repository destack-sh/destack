use crate::runtime::bindings::{
    BindingBlocking, BindingDescriptor, BindingEffect, BindingEffectClass, BindingEngine,
    BindingScope,
};
use crate::runtime::policy::Selector;
use destack_source::matches as glob_matches;
use destack_workspace::ExecutionMode;

/// Match one binding descriptor against one runtime filter.
pub(crate) fn matches_selector(
    filter: &Selector,
    descriptor: Option<BindingDescriptor>,
    mode: ExecutionMode,
    engine: Option<BindingEngine>,
    agent_id: Option<u64>,
) -> bool {
    // descriptor-aware filters require a binding descriptor
    if descriptor.is_none() && filter_has_binding_clauses(filter) {
        return false;
    }

    // match agent selector when present
    if let Some(agent_ids) = &filter.agent_ids {
        let Some(agent_id) = agent_id else {
            return false;
        };
        if !agent_ids.contains(&agent_id) {
            return false;
        }
    }

    // match engine selector when present
    if let Some(rule_engine) = filter.engine {
        let Some(engine) = engine else {
            return false;
        };
        if !matches_engine(rule_engine, engine) {
            return false;
        }
    }

    // match execution mode selector when present
    if let Some(modes) = &filter.execution_modes {
        let matches_mode = modes.iter().copied().any(|rule_mode| rule_mode == mode);
        if !matches_mode {
            return false;
        }
    }

    // match platform selector when present
    if let Some(platforms) = &filter.platforms {
        let current_platform = runtime_platform_name();
        let matches_platform = platforms
            .iter()
            .any(|pattern| glob_match(pattern, current_platform));
        if !matches_platform {
            return false;
        }
    }

    // match descriptor-aware selectors when descriptor is available
    if let Some(descriptor) = descriptor {
        // match binding name glob
        if let Some(pattern) = &filter.binding
            && !glob_match(pattern, descriptor.name)
        {
            return false;
        }

        // match module name glob
        if let Some(pattern) = &filter.module {
            let module_name = descriptor
                .name
                .strip_prefix("destack.")
                .unwrap_or(descriptor.name);
            if !glob_match(pattern, module_name) {
                return false;
            }
        }

        // match component name glob
        if let Some(pattern) = &filter.component {
            let component_name = binding_component_name(descriptor.name);
            if !glob_match(pattern, component_name) {
                return false;
            }
        }

        // match any required capability
        if let Some(pattern) = &filter.capability {
            let has_match = descriptor
                .requires()
                .iter()
                .any(|capability| glob_match(pattern, capability));
            if !has_match {
                return false;
            }
        }

        // match binding scope selector when present
        if let Some(scope) = filter.scope
            && !matches_scope(scope, descriptor.scope())
        {
            return false;
        }

        // match blocking selector when present
        if let Some(blocking) = filter.blocking
            && !matches_blocking(blocking, descriptor.blocking())
        {
            return false;
        }

        // match effect selector when present
        if let Some(effect) = filter.effect
            && !matches_effect(effect, descriptor.effect_class)
        {
            return false;
        }
    }

    true
}

/// Return true when a filter depends on descriptor fields.
fn filter_has_binding_clauses(filter: &Selector) -> bool {
    filter.binding.is_some()
        || filter.capability.is_some()
        || filter.component.is_some()
        || filter.module.is_some()
        || filter.scope.is_some()
        || filter.blocking.is_some()
        || filter.effect.is_some()
}

/// Match a runtime filter engine against one call engine.
fn matches_engine(rule_engine: BindingEngine, engine: BindingEngine) -> bool {
    rule_engine == engine
}

/// Match a runtime filter scope against one binding scope.
fn matches_scope(rule_scope: BindingScope, scope: BindingScope) -> bool {
    rule_scope == scope
}

/// Match a runtime filter blocking class against one binding class.
fn matches_blocking(rule_blocking: BindingBlocking, blocking: BindingBlocking) -> bool {
    rule_blocking == blocking
}

/// Match a runtime filter effect class against one binding effect.
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
        return "linux";
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
