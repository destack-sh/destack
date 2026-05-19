use std::fmt::Debug;

use destack_dir as dir;
use destack_source::ModuleId;

use super::DirSnapshotBuilder;

/// Return one optional local type id label.
pub(super) fn optional_type_label(value: Option<dir::LocalTypeId>) -> Option<String> {
    value.map(type_label)
}

/// Return one instantiation id label.
pub(super) fn instantiation_label(value: dir::LocalInstantiationId) -> String {
    value.to_string()
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

/// Return one local layout id label.
pub(super) fn layout_label(layout_id: dir::LocalLayoutId) -> String {
    format!("layout{}", layout_id.0)
}

/// Return one optional local layout id label.
pub(super) fn optional_layout_label(value: Option<dir::LocalLayoutId>) -> Option<String> {
    value.map(layout_label)
}

/// Return one layout shape label.
pub(super) fn layout_shape_label(shape: &dir::LayoutShape) -> String {
    match shape {
        dir::LayoutShape::None => "none".to_string(),
        dir::LayoutShape::Scalar => "scalar".to_string(),
        dir::LayoutShape::Any => "any".to_string(),
        dir::LayoutShape::Struct(_) => "struct".to_string(),
        dir::LayoutShape::Tuple(_) => "tuple".to_string(),
        dir::LayoutShape::Variant(_) => "variant".to_string(),
        dir::LayoutShape::Newtype(_) => "newtype".to_string(),
        dir::LayoutShape::Function => "function".to_string(),
    }
}

/// Return one optional integer label.
pub(super) fn optional_u32_label(value: Option<u32>) -> Option<String> {
    value.map(|value| value.to_string())
}

/// Return one local symbol id label.
pub(super) fn local_symbol_label(symbol_id: dir::LocalSymbolId) -> String {
    format!("symbol{}", symbol_id.id)
}

/// Return one global symbol id label.
pub(super) fn symbol_label(
    builder: &DirSnapshotBuilder<'_>,
    symbol_id: dir::GlobalSymbolId,
) -> String {
    if symbol_id.module_id == builder.tree.module_id {
        builder.symbol_label(symbol_id)
    } else {
        let module = builder.module_path(symbol_id.module_id);
        let symbol = local_symbol_label(symbol_id.local_id);

        format!("{module}::{symbol}")
    }
}

/// Return one dependency target field.
pub(super) fn dependency_target_field(
    builder: &DirSnapshotBuilder<'_>,
    target: Option<ModuleId>,
) -> (&'static str, String) {
    match target {
        Some(module_id) => ("module", builder.module_path(module_id)),
        None => ("target", "<unresolved>".to_string()),
    }
}

/// Return one member candidate label.
pub(super) fn member_candidate_label(
    builder: &DirSnapshotBuilder<'_>,
    candidate: &dir::MemberCandidate,
) -> String {
    symbol_label(builder, candidate.symbol)
}

/// Return one call candidate label.
pub(super) fn call_candidate_label(
    builder: &DirSnapshotBuilder<'_>,
    candidate: &dir::CallCandidate,
) -> String {
    symbol_label(builder, candidate.symbol)
}

/// Return one static argument label.
pub(super) fn static_argument_label(
    builder: &DirSnapshotBuilder<'_>,
    argument: &dir::StaticArgument,
) -> String {
    let value = static_term_label(builder, &argument.value);
    if let Some(name) = argument.name {
        format!("{}={value}", builder.strings.get(name))
    } else {
        value
    }
}

/// Return one static term label.
pub(super) fn static_term_label(
    builder: &DirSnapshotBuilder<'_>,
    term: &dir::StaticTerm,
) -> String {
    match term {
        dir::StaticTerm::ScalarLiteral { value } => scalar_literal_label(builder, value),
        dir::StaticTerm::TypeLiteral { value } => debug_label(value),
        dir::StaticTerm::Declaration {
            declaration,
            generic_arguments,
        } => {
            let declaration =
                builder.node_label((*declaration).into_global_any(builder.tree.module_id));
            if let Some(arguments) = generic_arguments {
                let arguments = arguments
                    .iter()
                    .map(|argument| static_argument_label(builder, argument))
                    .collect::<Vec<_>>()
                    .join(", ");

                format!("{declaration}<{arguments}>")
            } else {
                declaration
            }
        }
        dir::StaticTerm::Type { ty } => type_label(*ty),
        dir::StaticTerm::Array { elements } => {
            let elements = elements
                .iter()
                .map(|element| static_term_label(builder, element))
                .collect::<Vec<_>>()
                .join(", ");

            format!("[{elements}]")
        }
        dir::StaticTerm::FixedArray { value, length } => {
            let value = static_term_label(builder, value);
            let length = static_term_label(builder, length);

            format!("[{value}; {length}]")
        }
        dir::StaticTerm::Tuple { elements } => {
            let elements = elements
                .iter()
                .map(|element| static_term_label(builder, element))
                .collect::<Vec<_>>()
                .join(", ");

            format!("({elements})")
        }
        dir::StaticTerm::Object { properties } => {
            let properties = static_property_labels(builder, properties);
            format!("{{{properties}}}")
        }
        dir::StaticTerm::Struct { ty, properties } => {
            let properties = static_property_labels(builder, properties);
            format!("{} {{{properties}}}", type_label(*ty))
        }
    }
}

/// Return static property labels.
fn static_property_labels(
    builder: &DirSnapshotBuilder<'_>,
    properties: &[dir::StaticProperty],
) -> String {
    properties
        .iter()
        .map(|property| static_property_label(builder, property))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Return one static property label.
fn static_property_label(
    builder: &DirSnapshotBuilder<'_>,
    property: &dir::StaticProperty,
) -> String {
    match property {
        dir::StaticProperty::Field { key, value } => {
            let key = builder.static_key(*key);
            let value = static_term_label(builder, value);
            format!("{key}: {value}")
        }
        dir::StaticProperty::Method { key, .. } => {
            let key = key
                .map(|key| builder.static_key(key))
                .unwrap_or_else(|| "<call>".to_string());
            format!("{key}()")
        }
        dir::StaticProperty::Spread { value } => {
            let value = static_term_label(builder, value);
            format!("...{value}")
        }
    }
}

/// Return one scalar literal label.
pub(super) fn scalar_literal_label(
    builder: &DirSnapshotBuilder<'_>,
    literal: &dir::ScalarLiteral,
) -> String {
    match literal {
        dir::ScalarLiteral::Null => "null".to_string(),
        dir::ScalarLiteral::Boolean(value) => value.to_string(),
        dir::ScalarLiteral::Integer(value) => value.to_string(),
        dir::ScalarLiteral::Bigint(value) => format!("{value}n"),
        dir::ScalarLiteral::Float(value) => value.to_string(),
        dir::ScalarLiteral::Character(value) => format!("'{value}'"),
        dir::ScalarLiteral::String(value) => format!("{:?}", builder.strings.get(*value)),
        dir::ScalarLiteral::RegexString { content, flags } => {
            let flags = flags.map(|flags| builder.strings.get(flags)).unwrap_or("");

            format!("/{}/{flags}", builder.strings.get(*content))
        }
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
