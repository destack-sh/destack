use std::collections::HashSet;

use destack_dir::{
    DynamicKey, Expression, GlobalNodeIdAny, GlobalSymbolId, LocalTypeId, NodeTree, NodeType,
    PrimitiveType, ScalarLiteral, StaticKey, SymbolKey, SymbolTable, Type, TypeLiteral, TypeTable,
    WellKnownSymbol,
};
use destack_workspace::{ProfileId, WellKnownSymbols};

use super::mapped::MappedIndexKind;
use crate::Compiler;

/// Accumulated key information for `keyof` computation.
#[derive(Debug, Default, Clone)]
pub(super) struct KeySet {
    /// Literal keys explicitly present on a type.
    pub(super) literal_keys: HashSet<StaticKey>,
    /// Whether string index keys are present.
    pub(super) has_string: bool,
    /// Whether number index keys are present.
    pub(super) has_number: bool,
    /// Whether symbol index keys are present.
    pub(super) has_symbol: bool,
}

impl KeySet {
    /// Insert a literal key into the set.
    pub(super) fn insert_literal(&mut self, key: StaticKey) {
        self.literal_keys.insert(key);
    }

    /// Insert a key kind into the set.
    pub(super) fn insert_index_kind(&mut self, kind: MappedIndexKind) {
        match kind {
            MappedIndexKind::String => self.has_string = true,
            MappedIndexKind::Number => self.has_number = true,
            MappedIndexKind::Symbol => self.has_symbol = true,
        }
    }

    /// Check if this key set contains a literal key.
    pub(super) fn contains_key(&self, key: &StaticKey) -> bool {
        // allow index signatures to cover matching literal keys
        self.literal_keys.contains(key)
            || (self.has_string && key.is_string_like())
            || (self.has_number && key.is_number_like())
            || (self.has_symbol && key.is_symbol_like())
    }

    /// Union another key set into this one.
    pub(super) fn union_with(&mut self, other: &KeySet) {
        // merge literal keys
        self.literal_keys.extend(other.literal_keys.iter().copied());
        // merge index flags
        self.has_string = self.has_string || other.has_string;
        self.has_number = self.has_number || other.has_number;
        self.has_symbol = self.has_symbol || other.has_symbol;
    }

    /// Intersect another key set into this one.
    pub(super) fn intersect_with(&mut self, other: &KeySet) {
        // intersect literal keys with index compatibility
        let mut intersection = HashSet::new();
        for key in self
            .literal_keys
            .iter()
            .copied()
            .chain(other.literal_keys.iter().copied())
        {
            if self.contains_key(&key) && other.contains_key(&key) {
                intersection.insert(key);
            }
        }
        self.literal_keys = intersection;
        // intersect index flags
        self.has_string = self.has_string && other.has_string;
        self.has_number = self.has_number && other.has_number;
        self.has_symbol = self.has_symbol && other.has_symbol;
    }

    /// Iterate over literal keys in the set.
    pub(super) fn literal_keys(&self) -> impl Iterator<Item = StaticKey> + '_ {
        self.literal_keys.iter().copied()
    }
}

impl Compiler {
    /// Resolve a static key from a dynamic key when possible.
    pub(crate) fn static_key_from_dynamic_key(
        &self,
        profile: ProfileId,
        key: DynamicKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> Option<StaticKey> {
        match key {
            DynamicKey::Name(name) => Some(StaticKey::Name(name)),
            DynamicKey::Number(name) => Some(StaticKey::Number(name)),
            DynamicKey::Expression(expression_id) => {
                self.static_key_from_expression(profile, expression_id, tree, symbols, types)
            }
            DynamicKey::NamedExpression { .. } => None,
        }
    }

    /// Resolve a static key from a key expression when possible.
    fn static_key_from_expression(
        &self,
        profile: ProfileId,
        expression_id: destack_dir::LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> Option<StaticKey> {
        let expression = tree.get(expression_id);

        // helpers for well-known symbol resolution across ambient libs
        let symbol_key_for_global = |symbol: GlobalSymbolId| {
            if symbol.module_id == symbols.module_id {
                return symbols.get_symbol(symbol.local_id).key;
            }

            let remote_module = self.program.modules.get(symbol.module_id);
            let remote_module = remote_module.read();
            let remote_symbols = remote_module.dir(profile).symbols.read();
            remote_symbols.get_symbol(symbol.local_id).key
        };
        let normalize_well_known_symbol =
            |symbol: GlobalSymbolId, well_known: &WellKnownSymbols| -> Option<GlobalSymbolId> {
                let group = well_known.get_group(WellKnownSymbol::Symbol)?;
                let candidates = [group.value, group.ty];

                for candidate in candidates.iter().flatten() {
                    if symbol == *candidate {
                        return Some(*candidate);
                    }
                }

                let is_ambient = self.program.builtins.as_ref().is_some_and(|builtins| {
                    let profile = self.program.profile(profile);
                    builtins
                        .ambient_libs(&profile.key)
                        .is_some_and(|modules| modules.contains(&symbol.module_id))
                });
                if !is_ambient {
                    return None;
                }

                let symbol_key = symbol_key_for_global(symbol)?;
                for candidate in candidates.iter().flatten() {
                    let base_key = symbol_key_for_global(*candidate)?;
                    if symbol_key == base_key {
                        return Some(*candidate);
                    }
                }

                None
            };

        // resolve Symbol.* member keys
        if let Expression::Member { left, name, .. } = expression {
            let well_known = self.get_well_known_symbols(profile)?;
            let symbol_key = tree.get(*left).target_symbol().and_then(|base_symbol| {
                well_known
                    .symbol_key_for_member(base_symbol, *name)
                    .or_else(|| {
                        let normalized = normalize_well_known_symbol(base_symbol, &well_known)?;
                        well_known.symbol_key_for_member(normalized, *name)
                    })
            })?;
            return Some(StaticKey::Symbol(SymbolKey::WellKnown(symbol_key)));
        }

        // resolve Symbol.for("name") keys
        if let Expression::Call {
            left,
            dynamic_arguments,
            ..
        } = expression
        {
            let Expression::Member { left, name, .. } = tree.get(*left) else {
                return None;
            };
            let well_known = self.get_well_known_symbols(profile)?;
            let symbol_symbol = well_known.get_symbol(WellKnownSymbol::Symbol)?;
            let base_symbol = tree.get(*left).target_symbol()?;
            if base_symbol != symbol_symbol
                && normalize_well_known_symbol(base_symbol, &well_known).is_none()
            {
                return None;
            }
            let member_name = self.program.strings.get(*name);
            if member_name.as_ref() != "for" {
                return None;
            }

            let argument_id = dynamic_arguments.first()?;
            let argument = tree.get(*argument_id);
            let value_id = argument.value();
            let Expression::ScalarLiteral {
                value: ScalarLiteral::String(name),
            } = tree.get(value_id)
            else {
                return None;
            };
            return Some(StaticKey::Symbol(SymbolKey::Registry(*name)));
        }

        // resolve unique symbol references
        if let Some(symbol) = expression.target_symbol() {
            // detect unique symbols without forcing a full evaluation
            let is_unique_symbol_type =
                |type_id: LocalTypeId, tree: &NodeTree, types: &TypeTable| match types
                    .get_type(type_id)
                {
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::UniqueSymbol),
                    } => true,
                    Type::Unevaluated(expression_id) => matches!(
                        tree.get(*expression_id),
                        Expression::TypeLiteral {
                            value: TypeLiteral::Primitive(PrimitiveType::UniqueSymbol),
                        }
                    ),
                    _ => false,
                };

            // prefer inferred value types
            if let Some(value_type_id) = types.get_value_type_id(symbol)
                && matches!(
                    types.get_type(value_type_id),
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::UniqueSymbol),
                    }
                )
            {
                return Some(StaticKey::Symbol(SymbolKey::Unique(symbol)));
            }

            // resolve declared types for symbols, including declarator fallbacks
            let declared_type_id_for_symbol =
                |symbols: &SymbolTable, types: &TypeTable, tree: &NodeTree| {
                    let symbol_entry = symbols.get_symbol(symbol.local_id);
                    let primary_declaration = symbol_entry.primary_declaration?;
                    if let Some(declared_type_id) = types.get_declared_type_id(primary_declaration)
                    {
                        return Some(declared_type_id);
                    }

                    // walk up to find declarator types for pattern bound symbols
                    let mut current_id = primary_declaration.local_id;
                    while let Some(parent) = tree.get_parent(current_id.id) {
                        if parent.ty == NodeType::Declarator {
                            let global_parent = GlobalNodeIdAny::new(symbols.module_id, parent);
                            return types.get_declared_type_id(global_parent);
                        }
                        current_id = parent;
                    }

                    None
                };

            // build a shared check for declared unique symbol types
            let is_unique_symbol = |symbols: &SymbolTable, types: &TypeTable, tree: &NodeTree| {
                let symbol_entry = symbols.get_symbol(symbol.local_id);
                if symbol_entry.primary_declaration.is_none() {
                    return false;
                };
                let Some(declared_type_id) = declared_type_id_for_symbol(symbols, types, tree)
                else {
                    return false;
                };

                is_unique_symbol_type(declared_type_id, tree, types)
            };

            // check declared types in the owning module
            if symbol.module_id == symbols.module_id {
                if is_unique_symbol(symbols, types, tree) {
                    return Some(StaticKey::Symbol(SymbolKey::Unique(symbol)));
                }
            } else {
                let remote_module = self.program.modules.get(symbol.module_id);
                let remote_module = remote_module.read();
                let remote_dir = remote_module.dir(profile);
                let remote_symbols = remote_dir.symbols.read();
                let remote_types = remote_dir.types.read();
                let remote_tree = remote_dir.tree.read();
                if is_unique_symbol(&remote_symbols, &remote_types, &remote_tree) {
                    return Some(StaticKey::Symbol(SymbolKey::Unique(symbol)));
                }
            }
        }

        None
    }
}
