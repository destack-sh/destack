use std::collections::HashSet;

use destack_dir::{
    Declaration, Expression, GlobalSymbolId, Key, LocalNodeId, Name, StaticKey, SymbolTable, Tree,
    TypeExpression, TypeMember,
};
use destack_workspace::{Module, ProfileId, Revision};

use crate::{Compiler, ResolveError, ResolveResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Return the nominal owner symbol denoted by one expression.
    fn nominal_owner_symbol_for_expression(
        &self,
        tree: &Tree,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<GlobalSymbolId> {
        let expression = tree.get(expression_id);

        // direct resolved references own themselves
        if let Some(target_symbol) = expression.target_symbol() {
            return Some(target_symbol);
        }

        // parenthesized wrappers preserve ownership
        if let Expression::Parenthesized { expression } = expression {
            return self.nominal_owner_symbol_for_expression(tree, *expression);
        }

        // instantiation wrappers preserve the nominal owner on the left
        if let Expression::Instantiation { left, .. } = expression {
            return self.nominal_owner_symbol_for_expression(tree, *left);
        }

        None
    }

    /// Return the nominal owner symbol denoted by one type expression.
    pub(crate) fn nominal_owner_symbol_for_type_expression(
        &self,
        tree: &Tree,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> Option<GlobalSymbolId> {
        let expression = tree.get(expression_id);

        // direct resolved references own themselves
        if let Some(target_symbol) = expression.target_symbol() {
            return Some(target_symbol);
        }

        // parenthesized wrappers preserve ownership
        if let TypeExpression::Parenthesized { expression } = expression {
            return self.nominal_owner_symbol_for_type_expression(tree, *expression);
        }

        None
    }

    /// Return inherited nominal symbols for one declaration.
    fn inherited_heritage_symbols(
        &self,
        tree: &Tree,
        declaration: &Declaration,
    ) -> Vec<GlobalSymbolId> {
        // struct heritage
        if let Declaration::Struct(declaration) = declaration {
            return declaration
                .implements_types
                .iter()
                .chain(declaration.embedded_types.iter())
                .filter_map(|type_id| self.nominal_owner_symbol_for_type_expression(tree, *type_id))
                .collect();
        }

        // class heritage
        if let Declaration::Class(declaration) = declaration {
            return declaration
                .implements_types
                .iter()
                .filter_map(|type_id| self.nominal_owner_symbol_for_type_expression(tree, *type_id))
                .collect();
        }

        // enum heritage
        if let Declaration::Enum(declaration) = declaration {
            return declaration
                .implements_types
                .iter()
                .filter_map(|type_id| self.nominal_owner_symbol_for_type_expression(tree, *type_id))
                .collect();
        }

        // interface heritage
        if let Declaration::Interface(declaration) = declaration {
            return declaration
                .extends
                .iter()
                .filter_map(|heritage| {
                    self.nominal_owner_symbol_for_expression(tree, heritage.expression)
                })
                .collect();
        }

        // extension heritage
        if let Declaration::Extension(declaration) = declaration {
            return declaration
                .implements_types
                .iter()
                .filter_map(|type_id| self.nominal_owner_symbol_for_type_expression(tree, *type_id))
                .collect();
        }

        Vec::new()
    }

    /// Convert one member key into a static lookup key when possible.
    fn static_key_from_member_key(&self, key: Option<&Key>) -> Option<StaticKey> {
        let key = key?;

        match key {
            // named keys
            Key::Name(Name::Identifier(name)) | Key::Name(Name::String(name)) => {
                Some(StaticKey::Name(*name))
            }
            Key::Name(Name::Number(name)) => Some(StaticKey::Number(*name)),

            // private and computed keys are never static exports
            Key::Private(_) | Key::Expression(_) => None,
        }
    }

    /// Get one resolved dir snapshot for a module when available.
    fn dir_resolved_snapshot(
        &self,
        module_id: destack_source::ModuleId,
        profile: ProfileId,
    ) -> Option<std::sync::Arc<destack_artifact::DirResolved>> {
        self.dir_resolved(module_id, profile)
    }

    /// Get one prepared dir snapshot for a module when available.
    fn dir_prepared_snapshot(
        &self,
        module_id: destack_source::ModuleId,
        profile: ProfileId,
    ) -> Option<std::sync::Arc<destack_artifact::DirPrepared>> {
        self.dir_prepared(module_id, profile)
    }

    /// Resolve a static member symbol for a target symbol using module context fields.
    pub fn query_static_member_symbol(
        &self,
        revision: Revision,
        module: &Module,
        profile: ProfileId,
        target_symbol: GlobalSymbolId,
        member_key: StaticKey,
        tree: &Tree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        // canonicalize the target before member lookup
        let target_symbol =
            self.canonical_symbol_in_tables(module, profile, target_symbol, symbols);
        let mut visited_targets = HashSet::new();

        // use the current module tables when the target is local
        if target_symbol.module_id == module.id {
            return self.query_static_member_symbol_inner(
                revision,
                module,
                profile,
                target_symbol,
                member_key,
                tree,
                symbols,
                &mut visited_targets,
            );
        }

        // otherwise switch to the canonical target module snapshot
        let target_module = self
            .cache_module_snapshot(revision, target_symbol.module_id)
            .ok()?;
        let target_module = target_module.as_ref();
        let snapshot = self.dir_resolved_snapshot(target_symbol.module_id, profile)?;

        self.query_static_member_symbol_inner(
            revision,
            target_module,
            profile,
            target_symbol,
            member_key,
            &snapshot.tree,
            &snapshot.symbols,
            &mut visited_targets,
        )
    }

    /// Resolve a static member symbol for a target symbol using module context fields.
    fn query_static_member_symbol_inner(
        &self,
        revision: Revision,
        module: &Module,
        profile: ProfileId,
        target_symbol: GlobalSymbolId,
        member_key: StaticKey,
        tree: &Tree,
        symbols: &SymbolTable,
        visited_targets: &mut HashSet<GlobalSymbolId>,
    ) -> Option<GlobalSymbolId> {
        // stop recursive cycles in inherited member traversal
        if !visited_targets.insert(target_symbol) {
            return None;
        }

        // collect target declarations
        let symbol_entry = symbols.get_symbol(target_symbol.local_id);
        let mut declaration_ids = Vec::new();
        if let Some(primary_declaration) = symbol_entry.primary_declaration {
            declaration_ids.push(primary_declaration);
        }
        if let Some(secondary_declarations) = symbol_entry.secondary_declarations.as_deref() {
            declaration_ids.extend(secondary_declarations.iter().copied());
        }

        // scan declaration members for matching static fields or methods
        for declaration_id in declaration_ids {
            let Ok(declaration_id) = declaration_id.try_into_local_typed::<Declaration>() else {
                continue;
            };
            let declaration = tree.get(declaration_id);
            let (enum_fields, fields_static_by_default) = match declaration {
                Declaration::Class(_) => (None, false),
                Declaration::Struct(_) => (None, false),
                Declaration::Enum(declaration) => (Some(declaration.fields.as_slice()), true),
                Declaration::Interface(_) => (None, false),
                _ => continue,
            };

            let inherited_symbols = self.inherited_heritage_symbols(tree, declaration);

            if let Some(enum_fields) = enum_fields
                && let Some(symbol) = self.resolve_static_member_symbol_in_enum_fields(
                    target_symbol.module_id,
                    declaration_id,
                    enum_fields,
                    member_key,
                    tree,
                    symbols,
                )
            {
                return Some(symbol);
            }

            if let Some(member_ids) = declaration.member_ids()
                && let Some(symbol) = self.resolve_static_member_symbol_in_members(
                    target_symbol.module_id,
                    member_ids,
                    member_key,
                    fields_static_by_default,
                    tree,
                    symbols,
                )
            {
                return Some(symbol);
            }

            if let Some(member_ids) = declaration.type_member_ids()
                && let Some(symbol) = self.resolve_static_member_symbol_in_type_members(
                    target_symbol.module_id,
                    member_ids,
                    member_key,
                    tree,
                    symbols,
                )
            {
                return Some(symbol);
            }

            // check the class extends target after the class's own members
            if let Declaration::Class(declaration) = declaration
                && let Some(extends_expression_id) = declaration.extends_expression
                && let Some(heritage_symbol) =
                    self.nominal_owner_symbol_for_expression(tree, extends_expression_id)
            {
                let heritage_symbol =
                    self.canonical_symbol_in_tables(module, profile, heritage_symbol, symbols);
                if let Some(symbol) = self.query_static_member_symbol_inner(
                    revision,
                    module,
                    profile,
                    heritage_symbol,
                    member_key,
                    tree,
                    symbols,
                    visited_targets,
                ) {
                    return Some(symbol);
                }
            }

            if !inherited_symbols.is_empty()
                && let Some(symbol) = self.query_static_member_symbol_in_heritage(
                    revision,
                    module,
                    profile,
                    &inherited_symbols,
                    member_key,
                    tree,
                    symbols,
                    visited_targets,
                )
            {
                return Some(symbol);
            }
        }

        // scan extension declarations in this module
        for declaration_id in tree.iter_node_ids_of_type::<Declaration>() {
            let declaration = tree.get(declaration_id);
            let Declaration::Extension(declaration) = declaration else {
                continue;
            };
            let Some(extension_target) = declaration.target_symbol else {
                continue;
            };

            // require a canonical target match
            let canonical_target =
                self.canonical_symbol_in_tables(module, profile, extension_target, symbols);
            if canonical_target != target_symbol {
                continue;
            }

            // check extension members before inherited types
            if let Some(symbol) = self.resolve_static_member_symbol_in_members(
                module.id,
                &declaration.members,
                member_key,
                false,
                tree,
                symbols,
            ) {
                return Some(symbol);
            }

            // then check inherited symbols for the extension
            let inherited_symbols = declaration
                .implements_types
                .iter()
                .filter_map(|type_id| self.nominal_owner_symbol_for_type_expression(tree, *type_id))
                .collect::<Vec<_>>();
            if let Some(symbol) = self.query_static_member_symbol_in_heritage(
                revision,
                module,
                profile,
                &inherited_symbols,
                member_key,
                tree,
                symbols,
                visited_targets,
            ) {
                return Some(symbol);
            }
        }

        None
    }

    /// Resolve a static member symbol within one enum field list.
    fn resolve_static_member_symbol_in_enum_fields(
        &self,
        module_id: destack_source::ModuleId,
        declaration_id: LocalNodeId<Declaration>,
        fields: &[LocalNodeId<destack_dir::EnumField>],
        member_key: StaticKey,
        tree: &Tree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        for field_id in fields {
            let field = tree.get(*field_id);
            let field_key = StaticKey::Name(field.name.string());
            if field_key.matches(&member_key) {
                if let Some(symbol_id) =
                    self.enum_field_symbol_maybe(tree, symbols, declaration_id, *field_id)
                {
                    let symbol_entry = symbols.get_symbol(symbol_id);
                    let symbol_id = symbol_id.with_type(symbol_entry.ty);
                    return Some(GlobalSymbolId::new(module_id, symbol_id));
                }
            }
        }

        None
    }

    /// Resolve a static member symbol within a declaration member list.
    fn resolve_static_member_symbol_in_members(
        &self,
        module_id: destack_source::ModuleId,
        members: &[LocalNodeId<destack_dir::Member>],
        member_key: StaticKey,
        fields_static_by_default: bool,
        tree: &Tree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        for member_id in members {
            let member = tree.get(*member_id);
            let requires_static_member = member.key().is_some();

            // member key
            let static_key = if let Some(name) = member.name() {
                Some(StaticKey::Name(name))
            } else {
                self.static_key_from_member_key(member.key())
            };

            // require static members when the member kind needs it
            if requires_static_member && !member.is_static() && !fields_static_by_default {
                continue;
            }

            // match the member key against the static key
            let Some(static_key) = static_key else {
                continue;
            };
            if static_key.matches(&member_key) {
                let symbol = member.symbol();
                let symbol_entry = symbols.get_symbol(symbol);
                let symbol_id = symbol.with_type(symbol_entry.ty);
                return Some(GlobalSymbolId::new(module_id, symbol_id));
            }
        }

        None
    }

    /// Resolve a static member symbol within a type member list.
    fn resolve_static_member_symbol_in_type_members(
        &self,
        module_id: destack_source::ModuleId,
        members: &[LocalNodeId<TypeMember>],
        member_key: StaticKey,
        tree: &Tree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        for member_id in members {
            let member = tree.get(*member_id);

            // member key
            let static_key = if let Some(name) = member.name() {
                Some(StaticKey::Name(name))
            } else {
                self.static_key_from_member_key(member.key())
            };

            let Some(static_key) = static_key else {
                continue;
            };
            if static_key.matches(&member_key) {
                let symbol = member.symbol();
                let symbol_entry = symbols.get_symbol(symbol);
                let symbol_id = symbol.with_type(symbol_entry.ty);
                return Some(GlobalSymbolId::new(module_id, symbol_id));
            }
        }

        None
    }

    /// Resolve inherited static members from heritage expressions.
    fn query_static_member_symbol_in_heritage(
        &self,
        revision: Revision,
        module: &Module,
        profile: ProfileId,
        heritage_symbols: &[GlobalSymbolId],
        member_key: StaticKey,
        tree: &Tree,
        symbols: &SymbolTable,
        visited_targets: &mut HashSet<GlobalSymbolId>,
    ) -> Option<GlobalSymbolId> {
        for heritage_symbol in heritage_symbols {
            let canonical_symbol =
                self.canonical_symbol_in_tables(module, profile, *heritage_symbol, symbols);

            if canonical_symbol.module_id == module.id {
                if let Some(symbol) = self.query_static_member_symbol_inner(
                    revision,
                    module,
                    profile,
                    canonical_symbol,
                    member_key,
                    tree,
                    symbols,
                    visited_targets,
                ) {
                    return Some(symbol);
                }
            } else {
                let remote_module = self
                    .cache_module_snapshot(revision, canonical_symbol.module_id)
                    .ok()?;
                let remote_module = remote_module.as_ref();
                let Some(snapshot) =
                    self.dir_resolved_snapshot(canonical_symbol.module_id, profile)
                else {
                    return None;
                };

                if let Some(symbol) = self.query_static_member_symbol_inner(
                    revision,
                    remote_module,
                    profile,
                    canonical_symbol,
                    member_key,
                    &snapshot.tree,
                    &snapshot.symbols,
                    visited_targets,
                ) {
                    return Some(symbol);
                }
            }
        }

        None
    }

    /// Resolve canonical symbol identity using the provided symbol tables.
    fn canonical_symbol_in_tables(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
    ) -> GlobalSymbolId {
        self.canonical_symbol_in_tables_inner(module, profile, symbol, symbols, &mut HashSet::new())
    }

    /// Resolve canonical symbol identity using the provided symbol tables.
    fn canonical_symbol_in_tables_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        visited_symbols: &mut HashSet<GlobalSymbolId>,
    ) -> GlobalSymbolId {
        // stop recursive target walks in malformed import graphs
        if !visited_symbols.insert(symbol) {
            return symbol;
        }

        let (canonical_symbol, target_symbol) = if symbol.module_id == module.id {
            let symbol_entry = symbols.get_symbol(symbol.local_id);
            (symbol_entry.canonical_symbol, symbol_entry.target_symbol)
        } else {
            let Some(snapshot) = self.dir_prepared_snapshot(symbol.module_id, profile) else {
                return symbol;
            };
            let symbol_entry = snapshot.symbols.get_symbol(symbol.local_id);
            (symbol_entry.canonical_symbol, symbol_entry.target_symbol)
        };

        // prefer an explicitly resolved canonical symbol
        if let Some(canonical_symbol) = canonical_symbol {
            return canonical_symbol;
        }

        // otherwise keep following target aliases until the chain stabilizes
        if let Some(target_symbol) = target_symbol {
            return self.canonical_symbol_in_tables_inner(
                module,
                profile,
                target_symbol,
                symbols,
                visited_symbols,
            );
        }

        symbol
    }

    /// Resolve a static member symbol for a target symbol across module tables.
    pub fn resolve_static_member_symbol(
        &self,
        revision: Revision,
        module: &Module,
        profile: ProfileId,
        origin_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
        member_key: StaticKey,
        tree: &Tree,
        symbols: &SymbolTable,
    ) -> ResolveResult<GlobalSymbolId> {
        // prefer the canonical symbol when available
        self.require_dir_resolved(revision, target_symbol.module_id, profile)?;
        let canonical_symbol =
            self.canonical_symbol_in_tables(module, profile, target_symbol, symbols);

        // use the canonical target for member lookup
        let target_symbol = canonical_symbol;

        // ensure target module symbols are resolved for member lookup
        self.require_dir_resolved(revision, target_symbol.module_id, profile)?;

        // resolve the member when the target is in the current module
        if target_symbol.module_id == module.id {
            let Some(symbol) = self.query_static_member_symbol(
                revision,
                module,
                profile,
                target_symbol,
                member_key,
                tree,
                symbols,
            ) else {
                return Err(ResolveError::UnsupportedConstruct {
                    node: origin_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                });
            };
            return Ok(symbol);
        }

        // load the target module tables for member lookup
        let target_module = self
            .cache_module_snapshot(revision, target_symbol.module_id)
            .map_err(|error| ResolveError::Internal {
                message: format!("failed to load module snapshot: {error}"),
            })?;
        let target_module = target_module.as_ref();
        let snapshot = self
            .dir_resolved_snapshot(target_symbol.module_id, profile)
            .ok_or_else(|| ResolveError::Internal {
                message: format!(
                    "missing committed resolved dir artifact for {:?}",
                    target_symbol.module_id
                ),
            })?;

        let Some(symbol) = self.query_static_member_symbol(
            revision,
            target_module,
            profile,
            target_symbol,
            member_key,
            &snapshot.tree,
            &snapshot.symbols,
        ) else {
            return Err(ResolveError::UnsupportedConstruct {
                node: origin_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
            });
        };

        Ok(symbol)
    }
}
