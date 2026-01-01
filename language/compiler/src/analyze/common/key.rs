use std::collections::HashSet;

use destack_dir::{
    DynamicKey, Expression, NodeTree, PrimitiveType, ScalarLiteral, StaticKey, SymbolKey,
    SymbolTable, Type, TypeLiteral, TypeTable, WellKnownSymbol,
};
use destack_workspace::ProfileId;

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

        // resolve Symbol.* member keys
        if let Expression::Member { left, name, .. } = expression {
            let base_symbol = tree.get(*left).target_symbol()?;
            let well_known = self.get_well_known_symbols(profile)?;
            let symbol_key = well_known.symbol_key_for_member(base_symbol, *name)?;
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
            if base_symbol != symbol_symbol {
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

            // build a shared check for declared unique symbol types
            let is_unique_symbol = |symbols: &SymbolTable, types: &TypeTable| {
                let symbol_entry = symbols.get_symbol(symbol.local_id);
                let Some(declaration_id) = symbol_entry.primary_declaration else {
                    return false;
                };
                let Some(declared_type_id) = types.get_declared_type_id(declaration_id) else {
                    return false;
                };
                matches!(
                    types.get_type(declared_type_id),
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::UniqueSymbol),
                    }
                )
            };

            // check declared types in the owning module
            if symbol.module_id == symbols.module_id {
                if is_unique_symbol(symbols, types) {
                    return Some(StaticKey::Symbol(SymbolKey::Unique(symbol)));
                }
            } else {
                let remote_module = self.program.modules.get(symbol.module_id);
                let remote_module = remote_module.read();
                let remote_dir = remote_module.dir(profile);
                let remote_symbols = remote_dir.symbols.read();
                let remote_types = remote_dir.types.read();
                if is_unique_symbol(&remote_symbols, &remote_types) {
                    return Some(StaticKey::Symbol(SymbolKey::Unique(symbol)));
                }
            }
        }

        None
    }
}
