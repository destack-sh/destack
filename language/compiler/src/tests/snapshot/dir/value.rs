use std::fmt::Debug;

use destack_dir as dir;

use super::DirSnapshotBuilder;

/// Return one optional local type id label.
pub(super) fn optional_type_label(value: Option<dir::LocalTypeId>) -> Option<String> {
    value.map(type_label)
}

/// Return one optional instantiation id label.
pub(super) fn optional_instantiation_label(
    value: Option<dir::LocalInstantiationId>,
) -> Option<String> {
    value.map(|value| value.to_string())
}

/// Return one debug label as lower snake case.
pub(super) fn debug_label<T>(value: T) -> String
where
    T: Debug,
{
    let debug = format!("{value:?}");

    lower_snake(&debug)
}

/// Return one optional debug label.
pub(super) fn optional_debug_label<T>(value: Option<T>) -> Option<String>
where
    T: Debug,
{
    value.map(debug_label)
}

/// Return one local type id label.
pub(super) fn type_label(type_id: dir::LocalTypeId) -> String {
    format!("type{}", type_id.0)
}

/// Return one local symbol id label.
pub(super) fn local_symbol_label(symbol_id: dir::LocalSymbolId) -> String {
    format!("symbol{}", symbol_id.id)
}

/// Return one resolution label.
pub(super) fn resolution_label(
    builder: &DirSnapshotBuilder<'_>,
    resolution: &dir::Resolution,
) -> String {
    match resolution {
        dir::Resolution::Symbol(symbol_id) => builder.symbol_label(*symbol_id),
        dir::Resolution::Dependency(dependency) => format!("dependency:{dependency:?}"),
        dir::Resolution::Label(label) => format!("label:{}", debug_label(label)),
        dir::Resolution::Dispatch(dispatch) => format!("dispatch:{dispatch:?}"),
    }
}

/// Return one dependency target field.
pub(super) fn dependency_target_field(
    builder: &DirSnapshotBuilder<'_>,
    target: dir::DependencyTarget,
) -> (&'static str, String) {
    match target {
        dir::DependencyTarget::Module(module_id) => ("module", builder.module_path(module_id)),
        dir::DependencyTarget::External(specifier) => {
            ("external", builder.strings.get(specifier).to_string())
        }
        dir::DependencyTarget::Unresolved => ("target", "<unresolved>".to_string()),
    }
}

/// Return one export key label.
pub(super) fn export_key_label(builder: &DirSnapshotBuilder<'_>, key: dir::ExportKey) -> String {
    match key {
        dir::ExportKey::Default => "<default>".to_string(),
        dir::ExportKey::Named(name) => builder.static_key(name),
    }
}

/// Return one export selector label.
pub(super) fn export_selector_label(
    builder: &DirSnapshotBuilder<'_>,
    selector: dir::ExportSelector,
) -> String {
    match selector {
        dir::ExportSelector::Default => "<default>".to_string(),
        dir::ExportSelector::Named(name) => builder.static_key(name),
        dir::ExportSelector::Namespace => "<namespace>".to_string(),
    }
}

/// Return one captured binding label.
pub(super) fn optional_capture_binding_label(
    builder: &DirSnapshotBuilder<'_>,
    binding: Option<dir::CapturedBinding>,
) -> Option<String> {
    binding.map(|binding| {
        let symbol = builder.symbol_label(binding.symbol);
        let mode = debug_label(binding.mode);
        format!("{symbol}:{mode}")
    })
}

/// Return one macro trigger label.
pub(super) fn macro_trigger_label(
    builder: &DirSnapshotBuilder<'_>,
    trigger: &dir::MacroTrigger,
) -> String {
    match trigger {
        dir::MacroTrigger::Decorator(node_id) => {
            let node_id = node_id.clone().into_any();
            format!("decorator:{}", builder.node_label(node_id))
        }
        dir::MacroTrigger::AutoDerive => "auto_derive".to_string(),
    }
}

/// Convert one CamelCase-ish debug string to lower snake case.
fn lower_snake(value: &str) -> String {
    let mut result = String::new();

    for character in value.chars() {
        if character.is_ascii_uppercase() {
            if !result.is_empty() {
                result.push('_');
            }
            result.push(character.to_ascii_lowercase());
        } else {
            result.push(character);
        }
    }

    result
}
