use std::fmt::Debug;

use destack_dir as dir;

use super::DirSnapshotBuilder;
use crate::tests::snapshot::{SnapshotField, SnapshotRow};

/// Add an optional type id field when present.
pub(super) fn add_optional_type(
    row: &mut SnapshotRow,
    key: &'static str,
    value: Option<dir::LocalTypeId>,
) {
    if let Some(value) = value {
        row.fields.push(SnapshotField {
            key,
            value: type_id(value),
        });
    }
}

/// Add an optional instantiation id field when present.
pub(super) fn add_optional_instantiation(
    row: &mut SnapshotRow,
    key: &'static str,
    value: Option<dir::LocalInstantiationId>,
) {
    if let Some(value) = value {
        row.fields.push(SnapshotField {
            key,
            value: value.to_string(),
        });
    }
}

/// Render one debug name as lower snake case.
pub(super) fn debug<T>(value: T) -> String
where
    T: Debug,
{
    let debug = format!("{value:?}");

    lower_snake(&debug)
}

/// Render one optional debug value.
pub(super) fn optional_debug<T>(value: Option<T>) -> Option<String>
where
    T: Debug,
{
    value.map(debug)
}

/// Return one optional type id.
pub(super) fn optional_type_id(value: Option<dir::LocalTypeId>) -> Option<String> {
    value.map(type_id)
}

/// Return one local type id.
pub(super) fn type_id(type_id: dir::LocalTypeId) -> String {
    format!("type{}", type_id.0)
}

/// Return one local symbol id label.
pub(super) fn local_symbol(symbol_id: dir::LocalSymbolId) -> String {
    format!("symbol{}", symbol_id.id)
}

/// Return one resolution label.
pub(super) fn resolution(builder: &DirSnapshotBuilder<'_>, resolution: &dir::Resolution) -> String {
    match resolution {
        dir::Resolution::Symbol(symbol_id) => builder.symbol_label(*symbol_id),
        dir::Resolution::Dependency(dependency) => format!("dependency:{dependency:?}"),
        dir::Resolution::Label(label) => format!("label:{}", debug(label)),
        dir::Resolution::Dispatch(dispatch) => format!("dispatch:{dispatch:?}"),
    }
}

/// Add one dependency target field.
pub(super) fn add_dependency_target(
    row: SnapshotRow,
    builder: &DirSnapshotBuilder<'_>,
    target: dir::DependencyTarget,
) -> SnapshotRow {
    match target {
        dir::DependencyTarget::Module(module_id) => {
            row.field("module", builder.module_path(module_id))
        }
        dir::DependencyTarget::External(specifier) => {
            row.field("external", builder.strings.get(specifier))
        }
    }
}

/// Return one export name label.
pub(super) fn export_name(builder: &DirSnapshotBuilder<'_>, name: dir::ExportName) -> String {
    match name {
        dir::ExportName::Default => "default".to_string(),
        dir::ExportName::Named(name) => builder.static_key(name),
    }
}

/// Return one export selector label.
pub(super) fn export_selector(
    builder: &DirSnapshotBuilder<'_>,
    selector: dir::ExportSelector,
) -> String {
    match selector {
        dir::ExportSelector::Default => "default".to_string(),
        dir::ExportSelector::Named(name) => builder.static_key(name),
        dir::ExportSelector::Namespace => "namespace".to_string(),
    }
}

/// Return one captured binding label.
pub(super) fn optional_capture_binding(
    builder: &DirSnapshotBuilder<'_>,
    binding: Option<dir::CapturedBinding>,
) -> Option<String> {
    binding.map(|binding| {
        let symbol = builder.symbol_label(binding.symbol);
        let mode = debug(binding.mode);
        format!("{symbol}:{mode}")
    })
}

/// Return one capture directive label.
pub(super) fn capture_directive(directive: &dir::CaptureDirective) -> String {
    format!(
        "default:{} rules:{}",
        debug(directive.default),
        directive.rules.len()
    )
}

/// Return one macro trigger label.
pub(super) fn macro_trigger(
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
