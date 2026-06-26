use destack_dir as dir;

use crate::check::{Place, PlaceTarget, WalkState};

impl WalkState<'_, '_> {
    /// Check that one local binding is assigned before a read.
    pub(in crate::check) fn check_assigned_read(
        &mut self,
        source: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) {
        // only local variable bindings have definite assignment state
        if symbol.module_id != self.module
            || self.check.symbol_kind(symbol) != dir::SymbolKind::Variable
        {
            return;
        }

        // report unassigned reads at the read occurrence
        if !self.flow().is_assigned(symbol) {
            self.check
                .report_use_before_assigned(self.module, source, symbol);
        }
    }

    /// Mark one assigned place if it names a local binding.
    pub(in crate::check) fn mark_place_assigned(&mut self, place: Place) {
        // ignore non binding places
        if let PlaceTarget::Binding { symbol } = place.target {
            // ignore imported bindings
            if symbol.module_id != self.module {
                return;
            }

            self.flow_mut().mark_assigned(symbol);
        }
    }

    /// Mark bindings assigned by one initialized or ambient declarator.
    pub(in crate::check) fn mark_declarator_assigned(
        &mut self,
        declarator: &dir::Declarator,
        is_ambient: bool,
    ) {
        // only initialized and ambient declarators assign their pattern
        if is_ambient || declarator.value.is_some() {
            self.mark_bindings_assigned(declarator.pattern.into_any());
        }
    }

    /// Mark all bindings introduced by one source node as definitely assigned.
    pub(in crate::check) fn mark_bindings_assigned(&mut self, source: dir::LocalNodeIdAny) {
        // mark direct declaration symbol first
        if let Some(symbol) = self.check.module(self.module).declaration_symbol(source) {
            self.flow_mut().mark_assigned(symbol);
        }

        // walk nested binding shapes
        match source.ty {
            // parameter
            dir::NodeType::Parameter => {
                let id = source.into_typed::<dir::Parameter>();

                self.mark_parameter_bindings_assigned(id);
            }
            // pattern
            dir::NodeType::Pattern => {
                let id = source.into_typed::<dir::Pattern>();

                self.mark_pattern_bindings_assigned(id);
            }
            // pattern field
            dir::NodeType::PatternField => {
                let id = source.into_typed::<dir::PatternField>();

                self.mark_pattern_field_bindings_assigned(id);
            }
            // nodes without pattern bindings
            _ => {}
        }
    }

    /// Mark bindings introduced by one parameter as definitely assigned.
    fn mark_parameter_bindings_assigned(&mut self, id: dir::LocalNodeId<dir::Parameter>) {
        // walk parameter binding shape
        match self.tree.get(id) {
            // ({ name })
            dir::Parameter::Pattern { pattern, .. }
            // (...{ name })
            | dir::Parameter::VariadicPattern { pattern, .. } => {
                self.mark_pattern_bindings_assigned(*pattern);
            }
            // (name)
            dir::Parameter::Named { .. }
            // (...name)
            | dir::Parameter::VariadicNamed { .. }
            // ignore error parameters
            | dir::Parameter::Error => {}
        }
    }

    /// Mark bindings introduced by one pattern as definitely assigned.
    fn mark_pattern_bindings_assigned(&mut self, id: dir::LocalNodeId<dir::Pattern>) {
        // walk pattern binding shape
        match self.tree.get(id) {
            // name: pattern
            dir::Pattern::Binding {
                pattern: Some(pattern),
                ..
            }
            // pattern!
            | dir::Pattern::Must(pattern)
            // &pattern
            | dir::Pattern::BorrowOf { right: pattern, .. }
            // move pattern
            | dir::Pattern::MoveOf { right: pattern, .. }
            // *pattern
            | dir::Pattern::DereferenceOf { right: pattern }
            // pattern = value
            | dir::Pattern::Default { pattern, .. } => {
                self.mark_pattern_bindings_assigned(*pattern);
            }
            // [a, b]
            dir::Pattern::Tuple { fields }
            // [...items]
            | dir::Pattern::Sequence { fields }
            // { name }
            | dir::Pattern::Object { fields }
            // T(a, b)
            | dir::Pattern::NominalTuple { fields, .. }
            // T { name }
            | dir::Pattern::NominalObject { fields, .. } => {
                // mark each nested field pattern
                for field in fields {
                    self.mark_pattern_field_bindings_assigned(*field);
                }
            }
            // a | b
            dir::Pattern::Union { patterns } => {
                // mark each alternative binding pattern
                for pattern in patterns {
                    self.mark_pattern_bindings_assigned(*pattern);
                }
            }
            // name
            dir::Pattern::Binding { pattern: None, .. }
            // _
            | dir::Pattern::Wildcard
            // value
            | dir::Pattern::Expression { .. }
            // start..end
            | dir::Pattern::Range { .. } => {}
        }
    }

    /// Mark bindings introduced by one pattern field as definitely assigned.
    fn mark_pattern_field_bindings_assigned(&mut self, id: dir::LocalNodeId<dir::PatternField>) {
        // walk pattern field binding shape
        match self.tree.get(id) {
            // { name: pattern }
            dir::PatternField::Named {
                pattern: Some(pattern),
                ..
            }
            // { ...pattern }
            | dir::PatternField::Spread {
                pattern: Some(pattern),
            }
            // { [key]: pattern }
            | dir::PatternField::Computed { pattern, .. }
            // [pattern]
            | dir::PatternField::Positional { pattern } => {
                self.mark_pattern_bindings_assigned(*pattern);
            }
            // { name }
            dir::PatternField::Named { pattern: None, .. }
            // { ... }
            | dir::PatternField::Spread { pattern: None }
            // [,]
            | dir::PatternField::Elision => {}
        }
    }
}
