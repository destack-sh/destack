use std::collections::HashMap;

use super::resolve::MemberResolution;
use crate::{AnalyzeError, AnalyzeResult, Compiler, InferContext};
use destack_base::StringId;
use destack_dir::{
    Argument, Declaration, Expression, GlobalSymbolId, InferTable, LocalNodeId, LocalTypeId,
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

        // ensure instance types are available for reference receivers
        self.ensure_reference_instance_types_for_type(
            module,
            ctx.profile,
            expression_id.into_any(),
            left_ty_id,
            types,
        )?;

        // inherit static arguments and substitutions from the receiver
        let inherited = self.resolve_inherited_static_arguments(
            module,
            ctx.profile,
            left_id.into_any(),
            &left_ty,
            &ctx.options,
            tree,
            symbols,
            types,
        )?;

        // resolve member dispatch for the left type
        let member_key = StaticKey::Name(member_name);
        let member_resolution = self.resolve_member_resolution(
            module,
            &left_ty,
            &member_key,
            ctx.profile,
            tree,
            symbols,
            types,
        );
        let member_symbol = match &member_resolution {
            MemberResolution::Static { symbol } => Some(*symbol),
            _ => None,
        };

        // infer the member type
        let mut member_type_visited = Vec::new();
        let member_ty_id = self.infer_member_of_type(
            module,
            ctx.profile,
            expression_id.into_any(),
            &left_ty,
            &member_key,
            types,
            &mut member_type_visited,
        )?;
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
                        this_parameter,
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
                            &ctx.options,
                            tree,
                            symbols,
                            types,
                            infer,
                        )?;

                        let resolved_this_parameter = if inherited.substitutions.is_empty() {
                            this_parameter
                        } else {
                            let mut cache = HashMap::new();
                            this_parameter.map(|parameter| {
                                self.substitute_static_parameters(
                                    parameter,
                                    &inherited.substitutions,
                                    types,
                                    &mut cache,
                                )
                            })
                        };
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
                            this_parameter: resolved_this_parameter,
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
            let mut index_visited = Vec::new();
            let index_signature_ty_id = self.infer_index_signature_value_type_for_key(
                module,
                &left_ty,
                &member_key,
                types,
                &mut index_visited,
            );

            if let Some(index_signature_ty_id) = index_signature_ty_id {
                let options = ctx.options;
                if options.no_property_access_from_index_signature
                    && !self.is_import_meta_chain(tree, left_id)
                {
                    self.error(AnalyzeError::PropertyAccessFromIndexSignature {
                        node: expression_id.into_global_any(module.id),
                        receiver_ty: left_ty_id.into_global(module.id),
                        member_key,
                    });
                }

                self.record_member_resolution(
                    expression_id.into_global_any(module.id),
                    Some(left_ty_id),
                    &member_resolution,
                    member_instance_id,
                    true,
                    types,
                );

                index_signature_ty_id
            } else {
                if !self.is_import_meta_chain(tree, left_id) {
                    self.error(AnalyzeError::MissingMember {
                        node: expression_id.into_global_any(module.id),
                        receiver_ty: left_ty_id.into_global(module.id),
                        member_key,
                    });
                }

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
            }
        };

        let resolved_member_ty_id = {
            let mut cache = HashMap::new();
            self.substitute_this_type(resolved_member_ty_id, left_ty_id, types, &mut cache)
        };

        Ok(resolved_member_ty_id)
    }

    /// Return true when the expression is rooted at import.meta.
    fn is_import_meta_chain(
        &self,
        tree: &NodeTree,
        mut expression_id: LocalNodeId<Expression>,
    ) -> bool {
        loop {
            match tree.get(expression_id) {
                Expression::ImportMeta => return true,
                Expression::Member { left, .. } => {
                    expression_id = *left;
                }
                _ => return false,
            }
        }
    }

    /// Resolve the member symbol for a type and member key.
    pub(super) fn resolve_member_symbol_for_type(
        &self,
        module: &Module,
        receiver_ty: &Type,
        member_key: &StaticKey,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> Option<GlobalSymbolId> {
        match receiver_ty {
            Type::Value { value } => {
                let value_ty = types.get_type(*value).clone();
                self.resolve_member_symbol_for_type(
                    module, &value_ty, member_key, profile, tree, symbols, types, visited,
                )
            }
            Type::Reference { symbol, .. } => self.resolve_member_symbol_for_symbol(
                module, *symbol, member_key, profile, tree, symbols, types, visited,
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
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> Option<GlobalSymbolId> {
        // stop on cycles in symbol lookup
        if visited.contains(&symbol) {
            return None;
        }
        visited.push(symbol);

        // resolve members from the local module data
        if symbol.module_id == module.id {
            let allow_merge = module.language_type.supports_declaration_merging();
            return self.resolve_member_symbol_in_module(
                module,
                symbol,
                member_key,
                profile,
                tree,
                symbols,
                types,
                allow_merge,
                visited,
            );
        }

        // load remote module data for symbol lookup #RemoteAnalyze
        let remote_module = self.program.modules.get(symbol.module_id);
        let remote_module = remote_module.read();
        let remote_dir = remote_module.dir(profile);
        let remote_tree = remote_dir.tree.read();
        let remote_symbols = remote_dir.symbols.read();
        let remote_types = remote_dir.types.read();
        let allow_merge = remote_module.language_type.supports_declaration_merging();

        self.resolve_member_symbol_in_module(
            module,
            symbol,
            member_key,
            profile,
            &remote_tree,
            &remote_symbols,
            &remote_types,
            allow_merge,
            visited,
        )
    }

    /// Resolve member symbols using module-local declarations and merges. #RemoteAnalyze
    /// This follows infer_member_of_symbol lookup order using module data.
    fn resolve_member_symbol_in_module(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        allow_merge: bool,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> Option<GlobalSymbolId> {
        let symbol_entry = symbols.get_symbol(symbol.local_id);

        // step 1: check members declared directly on this symbol
        if let Some(member_symbol) = self.find_member_symbol_in_declaration(
            symbol.module_id,
            profile,
            symbol,
            member_key,
            tree,
            symbols,
        ) {
            return Some(member_symbol);
        }

        // step 2: check merge groups and global augmentations
        if allow_merge {
            // scan merge group peers for members
            if let Some(group_id) = symbol_entry.merge_group {
                for group_symbol in symbols.merge_group_symbols(group_id) {
                    if *group_symbol == symbol.local_id {
                        continue;
                    }

                    if let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                        module,
                        group_symbol.into_global(symbol.module_id),
                        member_key,
                        profile,
                        tree,
                        symbols,
                        types,
                        visited,
                    ) {
                        return Some(member_symbol);
                    }
                }
            }

            // scan global augmentations for additional members
            if let Some(key) = symbol_entry.key
                && let Some(global_symbols) =
                    self.get_global_symbol_group(module.id, profile, key, symbol_entry.space)
            {
                for global_symbol in global_symbols {
                    if global_symbol == symbol {
                        continue;
                    }

                    if let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                        module,
                        global_symbol,
                        member_key,
                        profile,
                        tree,
                        symbols,
                        types,
                        visited,
                    ) {
                        return Some(member_symbol);
                    }
                }
            }
        }

        // step 3: check inherited members and visible extensions
        if let Some(lineage) = types.get_lineage_for_symbol(symbol).cloned() {
            // follow extends first
            if let Some(extends) = lineage.extends
                && let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                    module, extends, member_key, profile, tree, symbols, types, visited,
                )
            {
                return Some(member_symbol);
            }

            // check implemented interfaces
            for implements in &lineage.implements {
                if let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                    module,
                    *implements,
                    member_key,
                    profile,
                    tree,
                    symbols,
                    types,
                    visited,
                ) {
                    return Some(member_symbol);
                }
            }

            // check embedded types
            for embedded in &lineage.embedded {
                if let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                    module, *embedded, member_key, profile, tree, symbols, types, visited,
                ) {
                    return Some(member_symbol);
                }
            }
        }

        // check visible extensions for this symbol
        let extension_ids = types.get_extensions_for_target(symbol)?.clone();
        for extension_id in extension_ids {
            let extension = types.get_extension(extension_id);
            if !self.is_extension_visible(module, extension) {
                continue;
            }

            if let Some(member_symbol) = self.find_member_symbol_in_declaration(
                symbol.module_id,
                profile,
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
        module_id: destack_source::ModuleId,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        let symbol_entry = symbols.get_symbol(symbol.local_id);

        // collect primary and secondary declarations to scan
        let mut declaration_ids = Vec::new();
        if let Some(primary_declaration) = symbol_entry.primary_declaration {
            declaration_ids.push(primary_declaration);
        }
        if let Some(secondary_declarations) = symbol_entry.secondary_declarations.as_deref() {
            declaration_ids.extend(secondary_declarations.iter().copied());
        }

        // scan declarations for a matching member
        for declaration_id in declaration_ids {
            let Ok(declaration_id) = declaration_id.try_into_local_typed::<Declaration>() else {
                continue;
            };
            let declaration = tree.get(declaration_id);

            if let Declaration::Enum {
                fields, members, ..
            } = declaration
            {
                // check enum fields before methods
                for field_id in fields {
                    let field = tree.get(*field_id);
                    let field_key = StaticKey::Name(field.name);
                    if field_key.matches(member_key) {
                        return Some(field.symbol.into_global(module_id));
                    }
                }

                // check enum methods and members
                for member_id in members {
                    let member = tree.get(*member_id);
                    let static_key = member
                        .key()
                        .and_then(|key| self.static_key_from_dynamic_key(profile, *key, tree));

                    if let Some(static_key) = static_key
                        && static_key.matches(member_key)
                    {
                        return Some(member.symbol().into_global(module_id));
                    }
                }

                continue;
            }

            // check type members on structured declarations
            let Some(members) = declaration.member_ids() else {
                continue;
            };

            for member_id in members {
                let member = tree.get(*member_id);
                let static_key = member
                    .key()
                    .and_then(|key| self.static_key_from_dynamic_key(profile, *key, tree));

                if let Some(static_key) = static_key
                    && static_key.matches(member_key)
                {
                    return Some(member.symbol().into_global(module_id));
                }
            }
        }

        None
    }
}
