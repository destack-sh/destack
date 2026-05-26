#![allow(clippy::too_many_arguments)]

use destack_core::StringPool;
use destack_dir as dir;
use destack_source::ModuleId;

use super::signature::format_call_signature;
use super::types::format_local_type;
use crate::core::ModuleQueryContext;

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
    ctx: &ModuleQueryContext<'_>,
    member: &dir::Member,
    member_id: dir::LocalNodeId<dir::Member>,
    module_id: ModuleId,
    dir_tree: dir::View<'_>,
    types: &dir::TypeTable<'_>,
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
        .get_node_type_id(node_id)
        .map(|type_id| format_local_type(type_id, types, ctx));

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
            ctx,
        ),
        dir::Member::StaticBlock { .. } => "(static block)".to_string(),
        dir::Member::ComptimeBlock { .. } => "(comptime block)".to_string(),
        dir::Member::Error => "(error member)".to_string(),
    }
}

/// Format hover text for an enum field.
pub fn format_enum_field_hover(
    strings: &StringPool,
    ctx: &ModuleQueryContext<'_>,
    field: &dir::EnumField,
    field_id: dir::LocalNodeId<dir::EnumField>,
    module_id: ModuleId,
    types: &dir::TypeTable<'_>,
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
    if let Some(type_id) = types.get_node_type_id(node_id) {
        let type_text = format_local_type(type_id, types, ctx);
        format!("(enum member) {qualified_name} = {type_text}")
    } else {
        format!("(enum member) {qualified_name}")
    }
}

/// Format hover text for a parameter.
pub fn format_parameter_hover(
    strings: &StringPool,
    ctx: &ModuleQueryContext<'_>,
    param: &dir::Parameter,
    param_id: dir::LocalNodeId<dir::Parameter>,
    module_id: ModuleId,
    types: &dir::TypeTable<'_>,
) -> String {
    // resolve the parameter name
    let name = match param {
        dir::Parameter::Named { name, .. } => {
            format_parameter_name(strings.get(*name), param.is_comptime())
        }
        dir::Parameter::Pattern { .. } => format_parameter_name("_", param.is_comptime()),
        dir::Parameter::VariadicNamed { name, .. } => {
            let name = format!("...{}", strings.get(*name));
            format_parameter_name(&name, param.is_comptime())
        }
        dir::Parameter::VariadicPattern { .. } => {
            format_parameter_name("...<pattern>", param.is_comptime())
        }
        dir::Parameter::Error => "<error>".to_string(),
    };

    // resolve the parameter type when available
    let node_id = dir::GlobalNodeIdAny {
        module_id,
        local_id: param_id.into(),
    };
    if let Some(type_id) = types.get_node_type_id(node_id) {
        let type_text = format_local_type(type_id, types, ctx);
        format!("(parameter) {name}: {type_text}")
    } else {
        format!("(parameter) {name}")
    }
}

/// Format one parameter label with its phase prefix.
fn format_parameter_name(name: &str, is_comptime: bool) -> String {
    if is_comptime {
        format!("comptime {name}")
    } else {
        name.to_string()
    }
}

/// Format hover text for a local variable.
pub fn format_local_variable_hover(
    name: Option<&str>,
    symbol_id: dir::GlobalSymbolId,
    types: &dir::TypeTable<'_>,
    ctx: &ModuleQueryContext<'_>,
) -> String {
    // resolve the display name
    let name = name.unwrap_or("<anonymous>");

    // resolve the local type when available
    if let Some(type_id) = types.get_symbol_type_id(symbol_id) {
        let type_text = format_local_type(type_id, types, ctx);
        format!("let {name}: {type_text}")
    } else {
        format!("let {name}")
    }
}

/// Format a simple symbol signature without additional metadata.
pub fn format_simple_signature(symbol_kind: dir::SymbolKind, name: Option<&str>) -> String {
    // resolve the display name
    let name = name.unwrap_or("<anonymous>");

    match symbol_kind {
        dir::SymbolKind::Variable => format!("let {name}"),
        dir::SymbolKind::Class => format!("class {name}"),
        dir::SymbolKind::Struct => format!("struct {name}"),
        dir::SymbolKind::Interface => format!("interface {name}"),
        dir::SymbolKind::NewtypeInterface => format!("newtype interface {name}"),
        dir::SymbolKind::Enum => format!("enum {name}"),
        dir::SymbolKind::EnumField => format!("enum field {name}"),
        dir::SymbolKind::Function => format!("function {name}"),
        dir::SymbolKind::Label => format!("label {name}"),
        dir::SymbolKind::Import => format!("import {name}"),
        dir::SymbolKind::Extension => format!("extension {name}"),
        dir::SymbolKind::AssociatedConst => format!("const {name}"),
        dir::SymbolKind::AssociatedType => format!("type {name}"),
        dir::SymbolKind::GenericValueParameter => format!("comptime {name}"),
        dir::SymbolKind::TypeAlias => format!("type {name}"),
        dir::SymbolKind::GenericTypeParameter => format!("type {name}"),
        dir::SymbolKind::Newtype => format!("newtype {name}"),
    }
}

/// Format hover text for a method with full signature.
fn format_method_hover(
    qualified_name: &str,
    signature: &dir::FunctionSignature,
    module_id: ModuleId,
    dir_tree: dir::View<'_>,
    types: &dir::TypeTable<'_>,
    ctx: &ModuleQueryContext<'_>,
) -> String {
    // format a call signature label for the method
    let formatted = format_call_signature(
        qualified_name,
        signature,
        module_id,
        dir_tree,
        types,
        ctx,
        false,
    );

    // prefix the label with the member kind
    format!("(method) {}", formatted.label)
}
