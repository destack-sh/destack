use indexmap::{IndexMap, IndexSet};
use tspp_dir as dir;
use tspp_dir::NodeVisitor;

use super::super::state::BindState;

use crate::Compiler;

/// Binding keys exposed by one pattern alternative.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PatternBindingSet {
    /// The exposed binding keys.
    keys: IndexSet<dir::StaticKey>,
}

impl PatternBindingSet {
    /// Return the shared binding set exposed by one union pattern.
    fn from_union(tree: &dir::Tree, patterns: &[dir::LocalNodeId<dir::Pattern>]) -> Option<Self> {
        let mut expected = None;

        // compare each alternative binding set
        for pattern in patterns {
            let branch = Self::from_pattern(tree, *pattern)?;

            match &expected {
                None => expected = Some(branch),
                Some(expected) if expected == &branch => {}
                Some(_) => return None,
            }
        }

        expected
    }

    /// Return the binding set exposed by one pattern.
    fn from_pattern(tree: &dir::Tree, id: dir::LocalNodeId<dir::Pattern>) -> Option<Self> {
        let mut bindings = Self {
            keys: IndexSet::new(),
        };

        bindings.insert_pattern(tree, id).then_some(bindings)
    }

    /// Declare each binding key as one shared symbol.
    fn declare(self, state: &mut BindState<'_>) -> IndexMap<dir::StaticKey, dir::LocalSymbolId> {
        let modifiers = state.binding_modifiers();
        let mut symbols = IndexMap::new();

        // allocate one logical symbol per shared binding key
        for key in self.keys {
            let symbol_id = state.insert_binding_symbol(key, modifiers);
            symbols.insert(key, symbol_id);
        }

        symbols
    }

    /// Insert the binding keys introduced by one pattern.
    fn insert_pattern(&mut self, tree: &dir::Tree, id: dir::LocalNodeId<dir::Pattern>) -> bool {
        match tree.get(id) {
            dir::Pattern::Wildcard
            | dir::Pattern::Expression { .. }
            | dir::Pattern::Range { .. } => true,
            dir::Pattern::Binding { name, pattern } => {
                let inserted = self.keys.insert(dir::StaticKey::Name(*name));
                let nested = pattern.is_none_or(|pattern| self.insert_pattern(tree, pattern));

                inserted && nested
            }
            dir::Pattern::Must(pattern) | dir::Pattern::DereferenceOf { right: pattern } => {
                self.insert_pattern(tree, *pattern)
            }
            dir::Pattern::Default { pattern, .. }
            | dir::Pattern::BorrowOf { right: pattern, .. }
            | dir::Pattern::MoveOf { right: pattern, .. } => self.insert_pattern(tree, *pattern),
            dir::Pattern::Tuple { fields }
            | dir::Pattern::NominalTuple { fields, .. }
            | dir::Pattern::Sequence { fields }
            | dir::Pattern::Object { fields }
            | dir::Pattern::NominalObject { fields, .. } => fields
                .iter()
                .copied()
                .all(|field| self.insert_pattern_field(tree, field)),
            dir::Pattern::Union { patterns } => {
                let Some(branch) = Self::from_union(tree, patterns) else {
                    return false;
                };

                self.insert_set(branch)
            }
        }
    }

    /// Insert the binding keys introduced by one pattern field.
    fn insert_pattern_field(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::PatternField>,
    ) -> bool {
        match tree.get(id) {
            dir::PatternField::Named {
                name,
                pattern: None,
                ..
            } => self.keys.insert((*name).into()),
            dir::PatternField::Named {
                pattern: Some(pattern),
                ..
            }
            | dir::PatternField::Computed { pattern, .. }
            | dir::PatternField::Positional { pattern }
            | dir::PatternField::Rest {
                pattern: Some(pattern),
            } => self.insert_pattern(tree, *pattern),
            dir::PatternField::Rest { pattern: None } | dir::PatternField::Elision => true,
        }
    }

    /// Insert all binding keys from one nested binding set.
    fn insert_set(&mut self, bindings: Self) -> bool {
        for key in bindings.keys {
            if !self.keys.insert(key) {
                return false;
            }
        }

        true
    }
}

impl Compiler {
    /// Bind one union pattern.
    pub(in crate::bind) fn bind_union_pattern(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        patterns: &[dir::LocalNodeId<dir::Pattern>],
    ) {
        if state.has_union_pattern_symbols() {
            self.bind_union_pattern_branches(state, tree, patterns);

            return;
        }

        let Some(bindings) = PatternBindingSet::from_union(tree, patterns) else {
            self.bind_union_pattern_branches(state, tree, patterns);

            return;
        };

        let symbols = bindings.declare(state);
        state.push_union_pattern_symbols(symbols);
        self.bind_union_pattern_branches(state, tree, patterns);
        state.pop_union_pattern_symbols();
    }

    /// Bind one binding pattern symbol.
    pub(in crate::bind) fn bind_pattern_symbol(
        &self,
        state: &mut BindState<'_>,
        node_id: dir::LocalNodeId<dir::Pattern>,
        pattern: &dir::Pattern,
    ) {
        // ignore non binding patterns
        let dir::Pattern::Binding { name, .. } = pattern else {
            return;
        };

        // reuse a shared union pattern symbol
        let key = dir::StaticKey::Name(*name);
        if let Some(symbol_id) = state.union_pattern_symbol(key) {
            state.declare_symbol(symbol_id, node_id);

            return;
        }

        // declare pattern symbol
        let modifiers = state.binding_modifiers();
        let symbol_id = state.insert_binding_symbol(dir::StaticKey::Name(*name), modifiers);
        state.declare_symbol(symbol_id, node_id);
    }

    /// Bind one binding pattern field.
    pub(in crate::bind) fn bind_pattern_field(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        node_id: dir::LocalNodeId<dir::PatternField>,
        pattern_field: &dir::PatternField,
    ) {
        state.bind_node(node_id.into_any());

        // declare shorthand field binding
        if let dir::PatternField::Named {
            name,
            pattern: None,
            ..
        } = pattern_field
        {
            // reuse a shared union pattern symbol
            let key = (*name).into();
            if let Some(symbol_id) = state.union_pattern_symbol(key) {
                state.declare_symbol(symbol_id, node_id);
                return;
            }

            let modifiers = state.binding_modifiers();
            let symbol_id = state.insert_binding_symbol((*name).into(), modifiers);
            state.declare_symbol(symbol_id, node_id);
        }

        // visit nested field pattern
        dir::walk_pattern_field(state, tree, node_id, pattern_field);
    }

    /// Bind every branch of one union pattern.
    fn bind_union_pattern_branches(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        patterns: &[dir::LocalNodeId<dir::Pattern>],
    ) {
        for pattern in patterns {
            let pattern_node = tree.get(*pattern);

            state.visit_pattern(tree, *pattern, pattern_node);
        }
    }
}
