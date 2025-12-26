use std::collections::HashMap;

use super::resolve::MemberResolution;
use crate::{AnalyzeError, AnalyzeResult, Compiler, InferContext, InferTable};
use destack_base::StringId;
use destack_dir::{
    Argument, Declaration, DynamicKey, Expression, GlobalSymbolId, LocalNodeId, LocalTypeId,
    NodeTree, StaticKey, SymbolTable, Type, TypeLiteral, TypeTable,
};
use destack_workspace::{Module, ProfileId};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Infer a member access expression.
    pub(super) fn infer_member_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        member_name: StringId,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        let left_ty_id =
            self.infer_expression(module, left_id, tree, symbols, types, infer, ctx)?;
        let left_ty = types.get_type(left_ty_id).clone();
        let mut member_instance_id = None;

        // inherit static arguments and substitutions from the receiver
        let inherited = self.resolve_inherited_static_arguments(
            module,
            ctx.profile,
            left_id.into_any(),
            &left_ty,
            tree,
            symbols,
            types,
        )?;

        // resolve member dispatch for the left type
        let member_key = StaticKey::Name(member_name);
        let member_resolution =
            self.resolve_member_resolution(module, &left_ty, &member_key, tree, symbols, types);
        let member_symbol = match &member_resolution {
            MemberResolution::Static { symbol } => Some(*symbol),
            _ => None,
        };

        // infer the member type
        let mut member_type_visited = Vec::new();
        let member_ty_id = self.infer_member_of_type(
            module,
            &left_ty,
            &member_key,
            types,
            &mut member_type_visited,
        );
        let has_member = member_ty_id.is_some();
        let resolved_member_ty_id = if let Some(member_ty_id) = member_ty_id {
            let member_ty_id = if !inherited.substitutions.is_empty() {
                let mut cache = HashMap::new();
                self.substitute_static_parameters(
                    member_ty_id,
                    &inherited.substitutions,
                    types,
                    &mut cache,
                )
            } else {
                member_ty_id
            };

            let resolved_member_ty_id = if let Some(static_argument_ids) = static_arguments {
                match types.get_type(member_ty_id).clone() {
                    Type::Function {
                        asynchrony,
                        cardinality,
                        static_parameters,
                        dynamic_parameters,
                        return_type,
                    } => {
                        let resolved = self.resolve_function_signature(
                            module,
                            expression_id.into_any(),
                            member_symbol,
                            Some(static_argument_ids),
                            &static_parameters,
                            &dynamic_parameters,
                            return_type,
                            ctx.profile,
                            tree,
                            symbols,
                            types,
                            infer,
                        )?;

                        let (resolved_dynamic_parameters, resolved_return_type) =
                            if inherited.substitutions.is_empty() {
                                (resolved.dynamic_parameters, resolved.return_type)
                            } else {
                                let mut cache = HashMap::new();
                                let dynamic_parameters = resolved
                                    .dynamic_parameters
                                    .iter()
                                    .map(|parameter| {
                                        self.substitute_static_parameters(
                                            *parameter,
                                            &inherited.substitutions,
                                            types,
                                            &mut cache,
                                        )
                                    })
                                    .collect::<Vec<_>>();
                                let return_type = resolved.return_type.map(|return_type| {
                                    self.substitute_static_parameters(
                                        return_type,
                                        &inherited.substitutions,
                                        types,
                                        &mut cache,
                                    )
                                });
                                (dynamic_parameters, return_type)
                            };

                        let instantiated_fn = Type::Function {
                            asynchrony,
                            cardinality,
                            static_parameters: Vec::new(),
                            dynamic_parameters: resolved_dynamic_parameters,
                            return_type: resolved_return_type,
                        };

                        if let Some(member_symbol) = member_symbol {
                            let mut instance_arguments = inherited.arguments.clone();
                            instance_arguments.extend(resolved.static_arguments);

                            if !instance_arguments.is_empty() {
                                let instance_id = self.register_instance_for_node(
                                    expression_id.into_global_any(module.id),
                                    member_symbol,
                                    instance_arguments,
                                    types,
                                );
                                member_instance_id = Some(instance_id);
                            }
                        }

                        types.insert_type_from(instantiated_fn, expression_id)
                    }
                    _ => {
                        self.error(AnalyzeError::MissingType {
                            node: expression_id.into_global_any(module.id),
                        });
                        member_ty_id
                    }
                }
            } else {
                member_ty_id
            };

            // record member resolution when possible
            self.record_member_resolution(
                expression_id.into_global_any(module.id),
                Some(left_ty_id),
                &member_resolution,
                member_instance_id,
                has_member,
                types,
            );

            resolved_member_ty_id
        } else {
            // member not found, report error and continue with unknown type
            self.error(AnalyzeError::MissingMember {
                node: expression_id.into_global_any(module.id),
                receiver_ty: left_ty_id.into_global(module.id),
                member_key,
            });

            // record unresolved member resolution
            self.record_member_resolution(
                expression_id.into_global_any(module.id),
                Some(left_ty_id),
                &member_resolution,
                member_instance_id,
                has_member,
                types,
            );

            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            types.insert_type_from(ty, expression_id)
        };

        Ok(resolved_member_ty_id)
    }

    /// Resolve the member symbol for a type and member key.
    pub(super) fn resolve_member_symbol_for_type(
        &self,
        module: &Module,
        receiver_ty: &Type,
        member_key: &StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> Option<GlobalSymbolId> {
        match receiver_ty {
            Type::Value { value } => {
                let value_ty = types.get_type(*value).clone();
                self.resolve_member_symbol_for_type(
                    module, &value_ty, member_key, tree, symbols, types, visited,
                )
            }
            Type::Reference { symbol, .. } => self.resolve_member_symbol_for_symbol(
                module, *symbol, member_key, tree, symbols, types, visited,
            ),
            _ => None,
        }
    }

    /// Resolve the member symbol for a nominal type symbol.
    pub(super) fn resolve_member_symbol_for_symbol(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> Option<GlobalSymbolId> {
        if visited.contains(&symbol) {
            return None;
        }
        visited.push(symbol);

        if let Some(member_symbol) =
            self.find_member_symbol_in_declaration(module, symbol, member_key, tree, symbols)
        {
            return Some(member_symbol);
        }

        if let Some(lineage) = types.get_lineage_for_symbol(symbol).cloned() {
            if let Some(extends) = lineage.extends
                && let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                    module, extends, member_key, tree, symbols, types, visited,
                )
            {
                return Some(member_symbol);
            }

            for implements in &lineage.implements {
                if let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                    module,
                    *implements,
                    member_key,
                    tree,
                    symbols,
                    types,
                    visited,
                ) {
                    return Some(member_symbol);
                }
            }

            for embedded in &lineage.embedded {
                if let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                    module, *embedded, member_key, tree, symbols, types, visited,
                ) {
                    return Some(member_symbol);
                }
            }
        }

        let extension_ids = types.get_extensions_for_target(symbol)?.clone();
        for extension_id in extension_ids {
            let extension = types.get_extension(extension_id);
            if !self.is_extension_visible(module, extension) {
                continue;
            }

            if let Some(member_symbol) = self.find_member_symbol_in_declaration(
                module,
                extension.symbol,
                member_key,
                tree,
                symbols,
            ) {
                return Some(member_symbol);
            }
        }

        None
    }

    /// Find a member symbol inside a declaration for a key.
    pub(super) fn find_member_symbol_in_declaration(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        if symbol.module_id != module.id {
            return None;
        }

        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let primary_declaration = symbol_entry.primary_declaration?;
        let declaration_id = primary_declaration
            .try_into_local_typed::<Declaration>()
            .ok()?;
        let declaration = tree.get(declaration_id);

        if let Declaration::Enum {
            fields, members, ..
        } = declaration
        {
            // check enum fields before methods
            for field_id in fields {
                let field = tree.get(*field_id);
                let field_key = StaticKey::Name(field.name);
                if &field_key == member_key {
                    return Some(field.symbol.into_global(module.id));
                }
            }

            // enum methods and members
            for member_id in members {
                let member = tree.get(*member_id);
                let static_key = member.key().and_then(|key| match key {
                    DynamicKey::Name(name) => Some(StaticKey::Name(*name)),
                    DynamicKey::Expression(_) | DynamicKey::NamedExpression { .. } => None,
                });

                if let Some(static_key) = static_key
                    && &static_key == member_key
                {
                    return Some(member.symbol().into_global(module.id));
                }
            }

            return None;
        }

        let members = match declaration {
            Declaration::Struct { members, .. }
            | Declaration::Class { members, .. }
            | Declaration::Interface { members, .. }
            | Declaration::Extension { members, .. } => members,
            _ => return None,
        };

        for member_id in members {
            let member = tree.get(*member_id);
            let static_key = member.key().and_then(|key| match key {
                DynamicKey::Name(name) => Some(StaticKey::Name(*name)),
                DynamicKey::Expression(_) | DynamicKey::NamedExpression { .. } => None,
            });

            if let Some(static_key) = static_key
                && &static_key == member_key
            {
                return Some(member.symbol().into_global(module.id));
            }
        }

        None
    }

    /// Resolve the canonical symbol for a reference.
    pub(super) fn canonical_symbol_id(
        &self,
        module: &Module,
        symbols: &SymbolTable,
        profile: ProfileId,
        symbol: GlobalSymbolId,
    ) -> GlobalSymbolId {
        let mut current_symbol = symbol;
        let mut visited = Vec::new();

        loop {
            if visited.contains(&current_symbol) {
                return current_symbol;
            }
            visited.push(current_symbol);

            let (canonical_symbol, target_symbol) = if current_symbol.module_id == module.id {
                let symbol_entry = symbols.get_symbol(current_symbol.local_id);
                (symbol_entry.canonical_symbol, symbol_entry.target_symbol)
            } else {
                let remote_module = self.program.modules.get(current_symbol.module_id);
                let remote_module = remote_module.read();
                let remote_symbols = remote_module.dir(profile).symbols.read();
                let symbol_entry = remote_symbols.get_symbol(current_symbol.local_id);
                (symbol_entry.canonical_symbol, symbol_entry.target_symbol)
            };

            if let Some(canonical_symbol) = canonical_symbol {
                return canonical_symbol;
            }

            if let Some(target_symbol) = target_symbol {
                current_symbol = target_symbol;
            } else {
                return current_symbol;
            }
        }
    }
}
