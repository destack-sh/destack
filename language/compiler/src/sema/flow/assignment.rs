use tspp_dir as dir;

use crate::sema::{AssignedPlace, CheckState};

impl CheckState<'_> {
    /// Report one local binding read before its assignment.
    pub(in crate::sema) fn report_unassigned_read(
        &mut self,
        source: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) {
        // only local variable bindings have definite assignment state
        if symbol.module_id != self.module_id
            || self
                .own_symbol_kind(symbol)
                .is_none_or(|kind| !kind.is_binding())
        {
            return;
        }

        // bindings owned by an outer flow are assigned in that flow's order
        if let Some(function) = self.flow.current_function_symbol()
            && !self.is_symbol_owned_by_function(symbol, function)
        {
            return;
        }

        // report unassigned reads at the read occurrence
        if !self.flow.assigned.contains(&AssignedPlace::Symbol(symbol)) {
            self.report_use_before_assigned(self.module_id, source, symbol);
        }
    }

    /// Add one local flow place to the assigned set.
    pub(in crate::sema) fn assign_place(&mut self, place: AssignedPlace) {
        self.flow.insert_assigned(place);
    }

    /// Assign the bindings of one initialized or ambient declarator.
    pub(in crate::sema) fn assign_declarator_bindings(
        &mut self,
        declarator: &dir::Declarator,
        is_ambient: bool,
    ) {
        // only initialized and ambient declarators assign their pattern
        if is_ambient || declarator.value.is_some() {
            self.assign_bindings(declarator.pattern.into_any());
        }
    }

    /// Assign every binding introduced by one source node.
    pub(in crate::sema) fn assign_bindings(&mut self, source: dir::LocalNodeIdAny) {
        match source.ty {
            // parameter
            dir::NodeType::Parameter => {
                let id = source.into_typed::<dir::Parameter>();

                self.assign_parameter_bindings(id);
            }
            // pattern
            dir::NodeType::Pattern => {
                let id = source.into_typed::<dir::Pattern>();

                self.assign_pattern_bindings(id);
            }
            // pattern field
            dir::NodeType::PatternField => {
                let id = source.into_typed::<dir::PatternField>();

                self.assign_pattern_field_bindings(id);
            }
            // nodes without pattern bindings
            _ => {}
        }
    }

    /// Assign the symbol declared by one binding source.
    fn assign_declared_binding(&mut self, source: dir::LocalNodeIdAny) {
        if let Some(symbol) = self.module(self.module_id).declaration_symbol(source) {
            self.flow.insert_assigned(AssignedPlace::Symbol(symbol));
        }
    }

    /// Assign every binding introduced by one parameter.
    fn assign_parameter_bindings(&mut self, id: dir::LocalNodeId<dir::Parameter>) {
        self.assign_declared_binding(id.into_any());

        // walk parameter binding shape
        let node = self.module(self.module_id).view().get(id).clone();
        match node {
            // ({ name })
            dir::Parameter::Pattern { pattern, .. }
            // (...{ name })
            | dir::Parameter::VariadicPattern { pattern, .. } => {
                self.assign_pattern_bindings(pattern);
            }
            // (name)
            dir::Parameter::Named { .. }
            // (...name)
            | dir::Parameter::VariadicNamed { .. }
            // ignore error parameters
            | dir::Parameter::Error => {}
        }
    }

    /// Assign every binding introduced by one pattern.
    fn assign_pattern_bindings(&mut self, id: dir::LocalNodeId<dir::Pattern>) {
        self.assign_declared_binding(id.into_any());

        // walk pattern binding shape
        let node = self.module(self.module_id).view().get(id).clone();
        match node {
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
                self.assign_pattern_bindings(pattern);
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
                // assign each nested field pattern
                for field in fields {
                    self.assign_pattern_field_bindings(field);
                }
            }
            // a | b
            dir::Pattern::Union { patterns } => {
                // assign each alternative binding pattern
                for pattern in patterns {
                    self.assign_pattern_bindings(pattern);
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

    /// Assign every binding introduced by one pattern field.
    fn assign_pattern_field_bindings(&mut self, id: dir::LocalNodeId<dir::PatternField>) {
        self.assign_declared_binding(id.into_any());

        // walk pattern field binding shape
        let node = self.module(self.module_id).view().get(id).clone();
        match node {
            // { name: pattern }
            dir::PatternField::Named {
                pattern: Some(pattern),
                ..
            }
            // { ...pattern }
            | dir::PatternField::Rest {
                pattern: Some(pattern),
            }
            // { [key]: pattern }
            | dir::PatternField::Computed { pattern, .. }
            // [pattern]
            | dir::PatternField::Positional { pattern } => {
                self.assign_pattern_bindings(pattern);
            }
            // { name }
            dir::PatternField::Named { pattern: None, .. }
            // { ... }
            | dir::PatternField::Rest { pattern: None }
            // [,]
            | dir::PatternField::Elision => {}
        }
    }
}
