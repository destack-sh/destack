#![allow(clippy::too_many_arguments)]

use destack_core::StringPool;
use destack_dir as dir;
use destack_source::ModuleId;
use destack_workspace::{Repository, Revision};

use super::signature::format_call_signature;
use super::types::format_local_type;

/// Format hover information as markdown for display.
pub fn format_hover_markdown(
    signature: &str,
    type_text: Option<&str>,
    documentation: Option<&str>,
    location: Option<&str>,
) -> String {
    // start with the signature block
    let mut result = String::new();
    result.push_str("**Signature**\n\n");
    result.push_str("```destack\n");
    result.push_str(signature);
    result.push_str("\n```");

    // add type information when available
    if let Some(type_text) = type_text {
        result.push_str("\n\n**Type**\n\n");
        result.push_str("```destack\n");
        result.push_str(type_text);
        result.push_str("\n```");
    }

    // add documentation when available
    if let Some(documentation) = documentation {
        result.push_str("\n\n**Documentation**\n\n");
        result.push_str(documentation);
    }

    // add source location when available
    if let Some(location) = location {
        result.push_str("\n\n**Location**\n\n`");
        result.push_str(location);
        result.push('`');
    }

    // return the formatted markdown
    result
}

/// Format hover text for a member (field, method, etc).
pub fn format_member_hover(
    strings: &StringPool,
    repository: &Repository,
    revision: Revision,
    member: &dir::Member,
    member_id: dir::LocalNodeId<dir::Member>,
    module_id: ModuleId,
    dir_tree: dir::View<'_>,
    types: &dir::TypeTable,
    container: Option<&str>,
) -> String {
    // resolve the member name from its key
    let member_name = member
        .key()
        .and_then(|key| match key {
            dir::Key::Name(name) => Some(strings.get(name.string()).to_string()),
            dir::Key::Private(string_id) => {
                let name = strings.get(*string_id);
                Some(format!("#{name}"))
            }
            dir::Key::Expression(_) => None,
        })
        .unwrap_or_else(|| "<anonymous>".to_string());

    // resolve the member type when available
    let node_id = dir::GlobalNodeIdAny {
        module_id,
        local_id: member_id.into(),
    };
    let type_str = types
        .get_declared_or_inferred_type_id(node_id)
        .map(|type_id| format_local_type(type_id, types, repository, revision, strings));

    // build the qualified name
    let qualified_name = match container {
        Some(name) => format!("{name}.{member_name}"),
        None => member_name,
    };

    // format the hover text by member kind
    match member {
        dir::Member::AssociatedType { .. } => {
            let member_name = strings.get(member.name().unwrap()).to_string();
            let qualified_name = match container {
                Some(container_name) => format!("{container_name}.{member_name}"),
                None => member_name,
            };

            if let Some(ty) = type_str {
                format!("(type member) {qualified_name} = {ty}")
            } else {
                format!("(type member) {qualified_name}")
            }
        }
        dir::Member::AssociatedConst { .. } => {
            let member_name = strings.get(member.name().unwrap()).to_string();
            let qualified_name = match container {
                Some(container_name) => format!("{container_name}.{member_name}"),
                None => member_name,
            };

            if let Some(ty) = type_str {
                format!("(comptime const) {qualified_name}: {ty}")
            } else {
                format!("(comptime const) {qualified_name}")
            }
        }
        dir::Member::Field { .. } => {
            if let Some(ty) = type_str {
                format!("(property) {qualified_name}: {ty}")
            } else {
                format!("(property) {qualified_name}")
            }
        }
        dir::Member::Method { .. } => format_method_hover(
            &qualified_name,
            member.signature().unwrap(),
            module_id,
            dir_tree,
            types,
            repository,
            revision,
            strings,
        ),
        dir::Member::StaticBlock { .. } => "(static block)".to_string(),
        dir::Member::ComptimeBlock { .. } => "(comptime block)".to_string(),
        dir::Member::Error => "(error member)".to_string(),
    }
}

/// Format hover text for an enum field.
pub fn format_enum_field_hover(
    strings: &StringPool,
    repository: &Repository,
    revision: Revision,
    field: &dir::EnumField,
    field_id: dir::LocalNodeId<dir::EnumField>,
    module_id: ModuleId,
    types: &dir::TypeTable,
    container: Option<&str>,
) -> String {
    // resolve the enum field name
    let field_name = strings.get(field.name.string()).to_string();

    // build the qualified name
    let qualified_name = match container {
        Some(name) => format!("{name}.{field_name}"),
        None => field_name,
    };

    // resolve the enum field type when available
    let node_id = dir::GlobalNodeIdAny {
        module_id,
        local_id: field_id.into(),
    };
    if let Some(type_id) = types.get_declared_or_inferred_type_id(node_id) {
        let type_text = format_local_type(type_id, types, repository, revision, strings);
        format!("(enum member) {qualified_name} = {type_text}")
    } else {
        format!("(enum member) {qualified_name}")
    }
}

/// Format hover text for a parameter.
pub fn format_parameter_hover(
    strings: &StringPool,
    repository: &Repository,
    revision: Revision,
    param: &dir::Parameter,
    param_id: dir::LocalNodeId<dir::Parameter>,
    module_id: ModuleId,
    types: &dir::TypeTable,
) -> String {
    // resolve the parameter name
    let name = match param {
        dir::Parameter::Named { name, .. } => strings.get(*name).to_string(),
        dir::Parameter::Pattern { .. } => "_".to_string(),
        dir::Parameter::VariadicNamed { name, .. } => format!("...{}", strings.get(*name)),
        dir::Parameter::VariadicPattern { .. } => "...<pattern>".to_string(),
        dir::Parameter::Error => "<error>".to_string(),
    };

    // resolve the parameter type when available
    let node_id = dir::GlobalNodeIdAny {
        module_id,
        local_id: param_id.into(),
    };
    if let Some(type_id) = types.get_declared_or_inferred_type_id(node_id) {
        let type_text = format_local_type(type_id, types, repository, revision, strings);
        format!("(parameter) {name}: {type_text}")
    } else {
        format!("(parameter) {name}")
    }
}

/// Format hover text for a local variable.
pub fn format_local_variable_hover(
    name: Option<&str>,
    symbol_id: dir::GlobalSymbolId,
    symbols: &dir::BindingTable,
    types: &dir::TypeTable,
    repository: &Repository,
    revision: Revision,
    strings: &StringPool,
) -> String {
    // resolve the display name
    let name = name.unwrap_or("<anonymous>");

    // resolve the local type when available
    if let Some(type_id) = types.symbol_type_id(symbols, symbol_id) {
        let type_text = format_local_type(type_id, types, repository, revision, strings);
        format!("let {name}: {type_text}")
    } else {
        format!("let {name}")
    }
}

/// Format a simple symbol signature without additional metadata.
pub fn format_simple_signature(symbol_form: dir::SymbolForm, name: Option<&str>) -> String {
    // resolve the display name
    let name = name.unwrap_or("<anonymous>");

    match symbol_form {
        dir::SymbolForm::Variable => format!("let {name}"),
        dir::SymbolForm::Class => format!("class {name}"),
        dir::SymbolForm::Struct => format!("struct {name}"),
        dir::SymbolForm::Interface => format!("interface {name}"),
        dir::SymbolForm::Enum => format!("enum {name}"),
        dir::SymbolForm::Function => format!("function {name}"),
        dir::SymbolForm::Import => format!("import {name}"),
        dir::SymbolForm::Extension => format!("extension {name}"),
        dir::SymbolForm::TypeAlias => format!("type {name}"),
        dir::SymbolForm::Newtype => format!("newtype {name}"),
    }
}

/// Format hover text for a method with full signature.
fn format_method_hover(
    qualified_name: &str,
    signature: &dir::FunctionSignature,
    module_id: ModuleId,
    dir_tree: dir::View<'_>,
    types: &dir::TypeTable,
    repository: &Repository,
    revision: Revision,
    strings: &StringPool,
) -> String {
    // format a call signature label for the method
    let formatted = format_call_signature(
        qualified_name,
        signature,
        module_id,
        dir_tree,
        types,
        repository,
        revision,
        strings,
        false,
    );

    // prefix the label with the member kind
    format!("(method) {}", formatted.label)
}
