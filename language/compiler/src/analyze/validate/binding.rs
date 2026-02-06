use std::str::FromStr;

use crate::{AnalyzeError, Compiler};
use destack_ast::Keyword;
use destack_base::StringId;
use destack_dir::{
    Declaration, LocalNodeId, Member, NodeTree, NodeType, Parameter, Property, StaticKey,
    SymbolSpace, SymbolTable,
};
use destack_workspace::{Module, ProfileId};

impl Compiler {
    /// Check if a keyword is reserved as a binding identifier.
    pub(super) fn is_reserved_binding_keyword(keyword: Keyword) -> bool {
        matches!(
            keyword,
            Keyword::Break
                | Keyword::Case
                | Keyword::Catch
                | Keyword::Class
                | Keyword::Const
                | Keyword::Continue
                | Keyword::Debugger
                | Keyword::Default
                | Keyword::Delete
                | Keyword::Do
                | Keyword::Else
                | Keyword::Enum
                | Keyword::Export
                | Keyword::Extends
                | Keyword::Finally
                | Keyword::For
                | Keyword::Function
                | Keyword::If
                | Keyword::Import
                | Keyword::In
                | Keyword::InstanceOf
                | Keyword::Interface
                | Keyword::Let
                | Keyword::New
                | Keyword::Package
                | Keyword::Private
                | Keyword::Protected
                | Keyword::Public
                | Keyword::Return
                | Keyword::Static
                | Keyword::Super
                | Keyword::Switch
                | Keyword::This
                | Keyword::Throw
                | Keyword::Try
                | Keyword::Typeof
                | Keyword::Var
                | Keyword::While
                | Keyword::With
                | Keyword::Yield
                | Keyword::Await
                | Keyword::Implements
        )
    }

    /// Check if a name is reserved as a binding identifier.
    pub(super) fn is_reserved_binding_name(&self, name: StringId) -> bool {
        if self.is_reserved_strict_assignment_name(name) {
            return true;
        }

        let name_str = self.program.strings.get(name);
        let Ok(keyword) = Keyword::from_str(name_str.as_ref()) else {
            return false;
        };
        Self::is_reserved_binding_keyword(keyword)
    }

    /// Check if a name is reserved for strict mode assignment targets.
    pub(super) fn is_reserved_strict_assignment_name(&self, name: StringId) -> bool {
        let eval_name = self.program.strings.intern("eval");
        let arguments_name = self.program.strings.intern("arguments");

        name == eval_name || name == arguments_name
    }

    /// Validate binding identifiers.
    pub(super) fn validate_binding_names(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) {
        if !module.is_user() {
            return;
        }

        for scope in symbols.scopes() {
            for (key, symbol_id) in symbols.active_named_symbols(scope) {
                let symbol = symbols.get_symbol(symbol_id);
                if symbol.space == SymbolSpace::Label {
                    continue;
                }
                let Some(primary_declaration) = symbol.primary_declaration else {
                    continue;
                };
                if let StaticKey::Name(name) = key
                    && self.is_reserved_binding_name(name)
                {
                    if self.program.strings.get(name) == "this"
                        && primary_declaration.local_id.ty == NodeType::Parameter
                    {
                        let parameter_id =
                            LocalNodeId::<Parameter>::new(primary_declaration.local_id.id);
                        if self.is_explicit_this_parameter(tree, parameter_id) {
                            continue;
                        }
                    }
                    self.error(AnalyzeError::ReservedIdentifier {
                        node: primary_declaration.into_anchored(Some(profile)),
                        name,
                    });
                }
            }
        }
    }

    /// Return true when a parameter is an explicit `this` parameter.
    fn is_explicit_this_parameter(
        &self,
        tree: &NodeTree,
        parameter_id: LocalNodeId<Parameter>,
    ) -> bool {
        let Some(parent) = tree.get_parent(parameter_id.id) else {
            return false;
        };

        match parent.ty {
            NodeType::Declaration => {
                let declaration = tree.get(parent.into_typed::<Declaration>());
                matches!(
                    declaration,
                    Declaration::Function { signature, .. }
                        if signature.this_parameter == Some(parameter_id)
                )
            }
            NodeType::Member => {
                let member = tree.get(parent.into_typed::<Member>());
                matches!(
                    member,
                    Member::Method { signature, .. }
                        if signature.this_parameter == Some(parameter_id)
                )
            }
            NodeType::Property => {
                let property = tree.get(parent.into_typed::<Property>());
                matches!(
                    property,
                    Property::Method { signature, .. }
                        if signature.this_parameter == Some(parameter_id)
                )
            }
            _ => false,
        }
    }
}
