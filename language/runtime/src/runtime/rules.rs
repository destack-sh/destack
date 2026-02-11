use crate::platform::bindings::{
    BindingBlocking, BindingDescriptor, BindingScope, EffectClass, ExecutionMode, PolicyEngine,
    ReplayPolicy,
};
use destack_source::matches as glob_matches;
use destack_workspace::{
    RuntimeFilter, RuntimeFilterBlocking, RuntimeFilterEffect, RuntimeFilterEngine,
    RuntimeFilterScope,
};

/// Match one binding descriptor against one runtime filter.
pub(crate) fn matches_runtime_filter(
    filter: &RuntimeFilter,
    descriptor: BindingDescriptor,
    mode: ExecutionMode,
    engine: Option<PolicyEngine>,
) -> bool {
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
        let matches_mode = modes
            .iter()
            .copied()
            .any(|rule_mode| rule_mode == mode.into());
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

    true
}

/// Match a runtime filter engine against one call engine.
fn matches_engine(rule_engine: RuntimeFilterEngine, engine: PolicyEngine) -> bool {
    matches!(
        (rule_engine, engine),
        (RuntimeFilterEngine::Vm, PolicyEngine::Vm)
            | (RuntimeFilterEngine::Native, PolicyEngine::Native)
    )
}

/// Match a runtime filter scope against one binding scope.
fn matches_scope(rule_scope: RuntimeFilterScope, scope: BindingScope) -> bool {
    matches!(
        (rule_scope, scope),
        (RuntimeFilterScope::Os, BindingScope::Os)
            | (RuntimeFilterScope::Runtime, BindingScope::Runtime)
            | (RuntimeFilterScope::Hybrid, BindingScope::Hybrid)
    )
}

/// Match a runtime filter blocking class against one binding class.
fn matches_blocking(rule_blocking: RuntimeFilterBlocking, blocking: BindingBlocking) -> bool {
    matches!(
        (rule_blocking, blocking),
        (RuntimeFilterBlocking::Always, BindingBlocking::Always)
            | (RuntimeFilterBlocking::Never, BindingBlocking::Never)
            | (RuntimeFilterBlocking::Sometimes, BindingBlocking::Sometimes)
    )
}

/// Match a runtime filter effect class against one binding effect.
fn matches_effect(rule_effect: RuntimeFilterEffect, effect_class: EffectClass) -> bool {
    matches!(
        (rule_effect, effect_class),
        (RuntimeFilterEffect::Pure, EffectClass::Pure)
            | (
                RuntimeFilterEffect::Deterministic,
                EffectClass::Deterministic
            )
            | (
                RuntimeFilterEffect::ExternalRecordable,
                EffectClass::External {
                    replay: ReplayPolicy::Recordable,
                },
            )
            | (
                RuntimeFilterEffect::ExternalNonRecordable,
                EffectClass::External {
                    replay: ReplayPolicy::NonRecordable,
                },
            )
    )
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
        return "ios";
    }

    #[cfg(target_os = "windows")]
    {
        return "windows";
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
        return "unknown";
    }
}
