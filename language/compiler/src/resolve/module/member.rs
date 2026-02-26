use std::collections::HashSet;

use destack_dir::{
    BindingAnchor, Declaration, DynamicKey, Expression, GlobalSymbolId, Heritage, LocalNodeId,
    NodeTree, StaticKey, SymbolTable,
};
use destack_workspace::{Module, ProfileId};

use crate::{Compiler, ResolveError, ResolveResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve a static member symbol for a target symbol using module context fields.
    pub fn query_static_member_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        target_symbol: GlobalSymbolId,
        member_key: StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        let mut visited_targets = HashSet::new();
        self.query_static_member_symbol_inner(
            module,
            profile,
            target_symbol,
            member_key,
            tree,
            symbols,
            &mut visited_targets,
        )
    }

    /// Resolve a static member symbol for a target symbol using module context fields.
    fn query_static_member_symbol_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        target_symbol: GlobalSymbolId,
        member_key: StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
        visited_targets: &mut HashSet<GlobalSymbolId>,
    ) -> Option<GlobalSymbolId> {
        // stop recursive cycles in interface fallback traversal
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
            let (members, enum_fields, heritage, fields_static_by_default) = match declaration {
                Declaration::Class {
                    members, heritage, ..
                } => (members.as_slice(), None, Some(heritage), false),
                Declaration::Struct {
                    members, heritage, ..
                } => (members.as_slice(), None, Some(heritage), false),
                Declaration::Enum {
                    fields,
                    members,
                    heritage,
                    ..
                } => (
                    members.as_slice(),
                    Some(fields.as_slice()),
                    Some(heritage),
                    true,
                ),
                Declaration::Interface {
                    members, heritage, ..
                } => (members.as_slice(), None, Some(heritage), false),
                _ => continue,
            };

            if let Some(enum_fields) = enum_fields
                && let Some(symbol) = self.resolve_static_member_symbol_in_enum_fields(
                    target_symbol.module_id,
                    enum_fields,
                    member_key,
                    tree,
                )
            {
                return Some(symbol);
            }

            if let Some(symbol) = self.resolve_static_member_symbol_in_members(
                target_symbol.module_id,
                members,
                member_key,
                fields_static_by_default,
                tree,
            ) {
                return Some(symbol);
            }

            if let Some(heritage) = heritage
                && let Some(symbol) = self.query_static_member_symbol_in_heritage(
                    module,
                    profile,
                    heritage,
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
            let Declaration::Extension {
                target_symbol: Some(extension_target),
                heritage,
                members,
                ..
            } = declaration
            else {
                continue;
            };

            // require a canonical target match
            let canonical_target =
                self.canonical_symbol_in_tables(module, profile, *extension_target, symbols);
            if canonical_target != target_symbol {
                continue;
            }

            // resolve extension members before interface fallback
            if let Some(symbol) = self.resolve_static_member_symbol_in_members(
                module.id, members, member_key, false, tree,
            ) {
                return Some(symbol);
            }

            // fall back to implemented interfaces when no extension member matches
            if let Some(symbol) = self.query_static_member_symbol_in_heritage(
                module,
                profile,
                heritage,
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
        fields: &[LocalNodeId<destack_dir::EnumField>],
        member_key: StaticKey,
        tree: &NodeTree,
    ) -> Option<GlobalSymbolId> {
        for field_id in fields {
            let field = tree.get(*field_id);
            let field_key = StaticKey::Name(field.name);
            if field_key.matches(&member_key) {
                return Some(field.symbol.into_global(module_id));
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
        tree: &NodeTree,
    ) -> Option<GlobalSymbolId> {
        for member_id in members {
            let member = tree.get(*member_id);
            let (modifiers, static_key, symbol, requires_static_anchor) = match member {
                destack_dir::Member::Type { name, symbol, .. } => {
                    (None, Some(StaticKey::Name(*name)), *symbol, false)
                }
                destack_dir::Member::ComptimeConst { name, symbol, .. } => {
                    (None, Some(StaticKey::Name(*name)), *symbol, false)
                }
                destack_dir::Member::Field {
                    modifiers,
                    key,
                    symbol,
                    ..
                } => (
                    modifiers.as_ref(),
                    key.as_ref().and_then(|key| match key {
                        DynamicKey::Name(name) => Some(StaticKey::Name(*name)),
                        DynamicKey::Number(name) => Some(StaticKey::Number(*name)),
                        _ => None,
                    }),
                    *symbol,
                    true,
                ),
                destack_dir::Member::Method {
                    modifiers,
                    key,
                    symbol,
                    ..
                } => (
                    modifiers.as_ref(),
                    key.as_ref().and_then(|key| match key {
                        DynamicKey::Name(name) => Some(StaticKey::Name(*name)),
                        DynamicKey::Number(name) => Some(StaticKey::Number(*name)),
                        _ => None,
                    }),
                    *symbol,
                    true,
                ),
                _ => continue,
            };

            // require static members when the member kind needs it
            if requires_static_anchor {
                let has_static_anchor = modifiers
                    .and_then(|modifiers| modifiers.anchor)
                    .is_some_and(|anchor| anchor == BindingAnchor::Static);
                let is_static = has_static_anchor || fields_static_by_default;
                if !is_static {
                    continue;
                }
            }

            // match the member key against the static key
            let Some(static_key) = static_key else {
                continue;
            };
            if static_key.matches(&member_key) {
                return Some(symbol.into_global(module_id));
            }
        }

        None
    }

    /// Resolve inherited static members from heritage expressions.
    fn query_static_member_symbol_in_heritage(
        &self,
        module: &Module,
        profile: ProfileId,
        heritage: &Heritage,
        member_key: StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
        visited_targets: &mut HashSet<GlobalSymbolId>,
    ) -> Option<GlobalSymbolId> {
        let extends_types = heritage.extends_types.as_deref().unwrap_or_default();
        let implements_types = heritage.implements_types.as_deref().unwrap_or_default();

        for heritage_expression_id in extends_types.iter().chain(implements_types.iter()) {
            let heritage_expression = tree.get(*heritage_expression_id);
            let Some(heritage_symbol) = heritage_expression.target_symbol() else {
                continue;
            };
            let canonical_symbol =
                self.canonical_symbol_in_tables(module, profile, heritage_symbol, symbols);

            if canonical_symbol.module_id == module.id {
                if let Some(symbol) = self.query_static_member_symbol_inner(
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
                let remote_module = self.program.modules.get(canonical_symbol.module_id);
                let remote_module = remote_module.read();
                let remote_dir = remote_module.dir(profile);
                let remote_tree = remote_dir.tree.read();
                let remote_symbols = remote_dir.symbols.read();
                if let Some(symbol) = self.query_static_member_symbol_inner(
                    &remote_module,
                    profile,
                    canonical_symbol,
                    member_key,
                    &remote_tree,
                    &remote_symbols,
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
        if symbol.module_id == module.id {
            let symbol_entry = symbols.get_symbol(symbol.local_id);
            symbol_entry.canonical_symbol.unwrap_or(symbol)
        } else {
            let remote_module = self.program.modules.get(symbol.module_id);
            let remote_module = remote_module.read();
            let remote_symbols = remote_module.dir(profile).symbols.read();
            let symbol_entry = remote_symbols.get_symbol(symbol.local_id);
            symbol_entry.canonical_symbol.unwrap_or(symbol)
        }
    }

    /// Resolve a static member symbol for a target symbol across module tables.
    pub fn resolve_static_member_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        origin_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
        member_key: StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> ResolveResult<GlobalSymbolId> {
        // prefer the canonical symbol when available
        self.require_resolve_module_canonical(target_symbol.module_id, profile)?;
        let canonical_symbol = {
            let module = self.program.modules.get(target_symbol.module_id);
            let module = module.read();
            let symbols = module.dir(profile).symbols.read();
            let symbol = symbols.get_symbol(target_symbol.local_id);
            symbol.canonical_symbol.unwrap_or(target_symbol)
        };

        // use the canonical target for member lookup
        let target_symbol = canonical_symbol;

        // ensure target module symbols are resolved for member lookup
        self.require_resolve_module_direct(target_symbol.module_id, profile)?;

        // resolve the member when the target is in the current module
        if target_symbol.module_id == module.id {
            let Some(symbol) = self.query_static_member_symbol(
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
        let target_module = self.program.modules.get(target_symbol.module_id);
        let target_module = target_module.read();
        let target_dir = target_module.dir(profile);
        let target_tree = target_dir.tree.read();
        let target_symbols = target_dir.symbols.read();

        let Some(symbol) = self.query_static_member_symbol(
            &target_module,
            profile,
            target_symbol,
            member_key,
            &target_tree,
            &target_symbols,
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
