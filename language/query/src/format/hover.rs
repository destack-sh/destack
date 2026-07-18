use destack_dir as dir;

use super::signature::format_call_signature;
use super::types::format_global_type;
use crate::{MemberKeyName, ModuleQueryContext};

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

/// Format a simple symbol signature without additional metadata.
pub fn format_simple_signature(symbol_kind: dir::SymbolKind, name: Option<&str>) -> String {
    match symbol_kind {
        dir::SymbolKind::Variable => format_named_signature("let", name),
        dir::SymbolKind::Class => format_named_signature("class", name),
        dir::SymbolKind::Struct => format_named_signature("struct", name),
        dir::SymbolKind::Interface => format_named_signature("interface", name),
        dir::SymbolKind::NewtypeInterface => format_named_signature("newtype interface", name),
        dir::SymbolKind::Enum => format_named_signature("enum", name),
        dir::SymbolKind::Variant => format_named_signature("variant", name),
        dir::SymbolKind::Function => format_named_signature("function", name),
        dir::SymbolKind::Label => format_named_signature("label", name),
        dir::SymbolKind::Import => format_named_signature("import", name),
        dir::SymbolKind::Extension => format_named_signature("extension", name),
        dir::SymbolKind::AssociatedConst => format_named_signature("const", name),
        dir::SymbolKind::AssociatedType => format_named_signature("type", name),
        dir::SymbolKind::GenericValueParameter => format_named_signature("comptime", name),
        dir::SymbolKind::TypeAlias => format_named_signature("type", name),
        dir::SymbolKind::GenericTypeParameter => format_named_signature("type", name),
        dir::SymbolKind::Newtype => format_named_signature("newtype", name),
    }
}

/// Format one keyword plus an optional display name.
fn format_named_signature(keyword: &str, name: Option<&str>) -> String {
    if let Some(name) = name {
        format!("{keyword} {name}")
    } else {
        keyword.to_string()
    }
}

impl ModuleQueryContext<'_> {
    /// Format hover text for a member.
    pub(crate) fn member_hover(
        &self,
        member_id: dir::LocalNodeId<dir::Member>,
        container: Option<&str>,
    ) -> String {
        let view = self.view();
        let member = view.get::<dir::Member>(member_id);
        let member_name = self.member_hover_key(member.key());
        let node_id = member_id.into_global(self.module_id()).into_any();
        let qualified_name = qualified_hover_name(container, &member_name);

        match member {
            dir::Member::AssociatedType { name, .. } => {
                let member_name = self.strings().get(*name).to_string();
                let qualified_name = qualified_hover_name(container, &member_name);
                let ty = self.hover_node_type(node_id);

                format!("(type member) {qualified_name} = {ty}")
            }
            dir::Member::AssociatedConst { name, .. } => {
                let member_name = self.strings().get(*name).to_string();
                let qualified_name = qualified_hover_name(container, &member_name);
                let ty = self.hover_node_type(node_id);

                format!("(comptime const) {qualified_name}: {ty}")
            }
            dir::Member::Field { .. } => {
                let ty = self.hover_node_type(node_id);

                format!("(property) {qualified_name}: {ty}")
            }
            dir::Member::Method { signature, .. } => self.method_hover(&qualified_name, signature),
            dir::Member::StaticBlock { .. } => "(static block)".to_string(),
            dir::Member::ComptimeBlock { .. } => "(comptime block)".to_string(),
            dir::Member::Error => panic!("error member reached hover formatting"),
        }
    }

    /// Format hover text for an enum field.
    pub(crate) fn enum_field_hover(
        &self,
        field_id: dir::LocalNodeId<dir::EnumField>,
        container: Option<&str>,
    ) -> String {
        let field = self.view().get::<dir::EnumField>(field_id);
        let field_name = self.strings().get(field.name.string()).to_string();
        let qualified_name = qualified_hover_name(container, &field_name);
        let node_id = field_id.into_global(self.module_id()).into_any();
        let type_text = self.hover_node_type(node_id);

        format!("(enum member) {qualified_name} = {type_text}")
    }

    /// Format hover text for a parameter.
    pub(crate) fn parameter_hover(&self, param_id: dir::LocalNodeId<dir::Parameter>) -> String {
        let param = self.view().get::<dir::Parameter>(param_id);
        let name = self.parameter_hover_name(param);
        let node_id = param_id.into_global(self.module_id()).into_any();
        let type_text = self.hover_node_type(node_id);

        format!("(parameter) {name}: {type_text}")
    }

    /// Format hover text for a local variable.
    pub(crate) fn local_variable_hover(
        &self,
        name: Option<&str>,
        symbol_id: dir::GlobalSymbolId,
    ) -> String {
        let type_text = self.hover_symbol_type(symbol_id);

        if let Some(name) = name {
            format!("let {name}: {type_text}")
        } else {
            format!("let: {type_text}")
        }
    }

    /// Format one checked type id for hover text.
    fn hover_type(&self, type_id: dir::GlobalTypeId) -> String {
        format_global_type(type_id, self)
            .unwrap_or_else(|| panic!("unable to format checked hover type {type_id:?}"))
    }

    /// Format the checked type for one hover node.
    fn hover_node_type(&self, node_id: dir::GlobalNodeIdAny) -> String {
        let type_id = self
            .types()
            .get_node_type_id(node_id)
            .unwrap_or_else(|| panic!("missing checked hover type for node {node_id:?}"));

        self.hover_type(type_id)
    }

    /// Format the checked type for one hover symbol.
    fn hover_symbol_type(&self, symbol_id: dir::GlobalSymbolId) -> String {
        let type_id = self
            .types()
            .get_symbol_type_id(symbol_id)
            .unwrap_or_else(|| panic!("missing checked hover type for symbol {symbol_id:?}"));

        self.hover_type(type_id)
    }

    /// Format one member key for hover text.
    fn member_hover_key(&self, key: Option<&dir::Key>) -> String {
        key.and_then(|key| key.member_name(self.strings()))
            .unwrap_or_else(|| "[computed]".to_string())
    }

    /// Format one parameter label.
    fn parameter_hover_name(&self, param: &dir::Parameter) -> String {
        match param {
            dir::Parameter::Named { name, .. } => self.strings().get(*name).to_string(),
            dir::Parameter::Pattern { .. } => "_".to_string(),
            dir::Parameter::VariadicNamed { name, .. } => {
                let name = self.strings().get(*name);
                format!("...{name}")
            }
            dir::Parameter::VariadicPattern { .. } => "..._".to_string(),
            dir::Parameter::Error => panic!("error parameter reached hover formatting"),
        }
    }

    /// Format hover text for a method with full signature.
    fn method_hover(&self, qualified_name: &str, signature: &dir::FunctionSignature) -> String {
        let formatted = format_call_signature(qualified_name, signature, self, false);

        format!("(method) {}", formatted.label)
    }
}

/// Return a container-qualified hover name.
fn qualified_hover_name(container: Option<&str>, name: &str) -> String {
    match container {
        Some(container) => format!("{container}.{name}"),
        None => name.to_string(),
    }
}
