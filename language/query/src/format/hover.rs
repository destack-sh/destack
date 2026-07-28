use destack_dir as dir;

use super::signature::format_call_signature;
use super::types::format_global_type;
use crate::{ModuleQueryContext, ProgramQueryContext, QueryResult};

impl ModuleQueryContext<'_> {
    /// Format one indexed member hover signature.
    pub(crate) fn member_hover(
        &self,
        program: &ProgramQueryContext<'_>,
        member: &dir::MemberEntry,
        call_detail: Option<&str>,
    ) -> QueryResult<Option<String>> {
        let container = match member.owner {
            Some(owner) => program.symbol_name(owner)?,
            None => member.container.clone(),
        };
        let qualified_name = qualified_hover_name(container.as_deref(), &member.name);

        match member.kind {
            dir::MemberKind::Field | dir::MemberKind::Property => {
                let Some(type_text) = self.member_type_text(program, member)? else {
                    return Ok(None);
                };

                Ok(Some(format!("(property) {qualified_name}: {type_text}")))
            }
            dir::MemberKind::Method
            | dir::MemberKind::Constructor
            | dir::MemberKind::CallSignature
            | dir::MemberKind::ConstructSignature => {
                let Some(signature) = self.member_signature(member.source) else {
                    return Ok(None);
                };
                let Some(signature) =
                    format_call_signature(&qualified_name, signature, self, program, false)?
                else {
                    return Ok(None);
                };
                let kind = if member.kind == dir::MemberKind::Method {
                    "method"
                } else {
                    "constructor"
                };

                Ok(Some(format!("({kind}) {signature}")))
            }
            dir::MemberKind::IndexSignature => {
                let Some(type_text) = self.member_type_text(program, member)? else {
                    return Ok(None);
                };

                Ok(Some(format!("(property) {qualified_name}: {type_text}")))
            }
            dir::MemberKind::AssociatedType => {
                let Some(type_text) = self.member_type_text(program, member)? else {
                    return Ok(None);
                };

                Ok(Some(format!(
                    "(type member) {qualified_name} = {type_text}"
                )))
            }
            dir::MemberKind::AssociatedConst => {
                let Some(type_text) = self.member_type_text(program, member)? else {
                    return Ok(None);
                };

                Ok(Some(format!(
                    "(comptime const) {qualified_name}: {type_text}"
                )))
            }
            dir::MemberKind::Variant if member.source.local_id.ty == dir::NodeType::EnumField => {
                let Some(type_text) = self.member_type_text(program, member)? else {
                    return Ok(None);
                };

                Ok(Some(format!("(enum member) {qualified_name}: {type_text}")))
            }
            dir::MemberKind::Variant => {
                Ok(call_detail.map(|call_detail| format!("(constructor) {call_detail}")))
            }
        }
    }

    /// Format hover text for a parameter.
    pub(crate) fn parameter_hover(
        &self,
        program: &ProgramQueryContext<'_>,
        parameter_id: dir::LocalNodeId<dir::Parameter>,
    ) -> QueryResult<Option<String>> {
        let parameter = self.view().get::<dir::Parameter>(parameter_id);
        let Some(name) = self.parameter_name(parameter)? else {
            return Ok(None);
        };
        let node_id = parameter_id.into_global(self.module_id()).into_any();
        let Some(type_text) = self.hover_node_type(program, node_id)? else {
            return Ok(None);
        };

        Ok(Some(format!("(parameter) {name}: {type_text}")))
    }

    /// Format hover text for a local variable.
    pub(crate) fn local_variable_hover(
        &self,
        program: &ProgramQueryContext<'_>,
        name: &str,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<String>> {
        let symbol = self.symbols().get_symbol(symbol_id.local_id);
        let Some(type_text) = self.hover_symbol_type(program, symbol_id)? else {
            return Ok(None);
        };
        let keyword = if symbol.binding_mutability == Some(dir::Mutability::Immutable) {
            "const"
        } else {
            "let"
        };

        Ok(Some(format!("{keyword} {name}: {type_text}")))
    }

    /// Format the checked type for one hover node.
    fn hover_node_type(
        &self,
        program: &ProgramQueryContext<'_>,
        node_id: dir::GlobalNodeIdAny,
    ) -> QueryResult<Option<String>> {
        let Some(type_id) = self.types().get_node_type_id(node_id) else {
            return Ok(None);
        };

        format_global_type(type_id, self, program)
    }

    /// Format the checked type for one hover symbol.
    fn hover_symbol_type(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<String>> {
        let Some(type_id) = self.types().get_symbol_type_id(symbol_id) else {
            return Ok(None);
        };

        format_global_type(type_id, self, program)
    }

    /// Return the authored function signature for one indexed member.
    fn member_signature(&self, source: dir::GlobalNodeIdAny) -> Option<&dir::FunctionSignature> {
        let view = self.view();

        match source.local_id.ty {
            dir::NodeType::Member => {
                let member_id = dir::LocalNodeId::<dir::Member>::new(source.local_id.id);

                view.get(member_id).signature()
            }
            dir::NodeType::TypeMember => {
                let member_id = dir::LocalNodeId::<dir::TypeMember>::new(source.local_id.id);

                view.get(member_id).signature()
            }
            _ => None,
        }
    }

    /// Format the checked type carried by one indexed member.
    fn member_type_text(
        &self,
        program: &ProgramQueryContext<'_>,
        member: &dir::MemberEntry,
    ) -> QueryResult<Option<String>> {
        let Some(type_id) = member.ty else {
            return Ok(None);
        };

        format_global_type(type_id, self, program)
    }
}

/// Return a container-qualified hover name.
fn qualified_hover_name(container: Option<&str>, name: &str) -> String {
    match container {
        Some(container) => format!("{container}.{name}"),
        None => name.to_string(),
    }
}
