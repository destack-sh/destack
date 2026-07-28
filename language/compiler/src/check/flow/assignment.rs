use destack_dir as dir;

use crate::check::{AssignedPlace, MoveSite, Obligation, UseAfterMoveObligation, WalkState};

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

        // captured bindings are assigned by their owning flow
        if let Some(function) = self.flow().current_function_symbol()
            && self.is_captured_symbol_reference(symbol, function)
        {
            return;
        }

        // moved places defer to the obligation, where the selected transfer
        // and copyability decide whether the move was real
        if let Some(site) = self.flow().moved_site(AssignedPlace::Symbol(symbol)) {
            self.check.push_obligation(
                Obligation::UseAfterMove(UseAfterMoveObligation {
                    source: source.into_global(self.module),
                    symbol,
                    site,
                }),
                self.flow().template_scope(),
            );

            return;
        }

        // report unassigned reads at the read occurrence
        if !self
            .flow()
            .assigned
            .contains(&AssignedPlace::Symbol(symbol))
        {
            self.check
                .report_use_before_assigned(self.module, source, symbol);
        }
    }

    /// Mark one consuming-position source, when it names a local binding.
    ///
    /// Receiver positions carry their enclosing call so the obligation can
    /// read the selected receiver adjustment; initializer and assignment
    /// positions carry the receiving binding.
    pub(in crate::check) fn mark_moved_source(
        &mut self,
        source: dir::LocalNodeId<dir::Expression>,
        call: Option<dir::LocalNodeId<dir::Expression>>,
        target: Option<dir::GlobalSymbolId>,
    ) {
        let node = source.into_global_any(self.module);
        let site = MoveSite {
            node,
            call: call.map(|call| call.into_global_any(self.module)),
            target,
        };
        self.mark_moved_identifier(source, site);
    }

    /// Mark one consuming argument, keyed by its bound argument node.
    ///
    /// The selected resolution binds parameters to argument nodes, so the
    /// obligation matches the site against the argument, not its value.
    pub(in crate::check) fn mark_moved_argument(
        &mut self,
        value: dir::LocalNodeId<dir::Expression>,
        argument: dir::LocalNodeId<dir::Argument>,
        call: dir::LocalNodeId<dir::Expression>,
    ) {
        let site = MoveSite {
            node: argument.into_global_any(self.module),
            call: Some(call.into_global_any(self.module)),
            target: None,
        };
        self.mark_moved_identifier(value, site);
    }

    /// Mark one identifier source's place with one move site.
    fn mark_moved_identifier(&mut self, source: dir::LocalNodeId<dir::Expression>, site: MoveSite) {
        // only identifier sources move a tracked place
        if !matches!(self.tree.get(source), dir::Expression::Identifier { .. }) {
            return;
        }
        let node = source.into_global_any(self.module);
        let Some(symbol) = self
            .check
            .resolutions(self.module)
            .name_resolution(node)
            .and_then(|resolution| resolution.symbols().first().copied())
        else {
            return;
        };
        if self.check.symbol_kind(symbol) != dir::SymbolKind::Variable {
            return;
        }

        self.flow_mut()
            .mark_moved(AssignedPlace::Symbol(symbol), site);
    }

    /// Mark one local flow place as assigned.
    pub(in crate::check) fn mark_place_assigned(&mut self, place: AssignedPlace) {
        self.flow_mut().mark_assigned(place);
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

    /// Mark the symbol declared by one binding source.
    fn mark_declared_binding(&mut self, source: dir::LocalNodeIdAny) {
        if let Some(symbol) = self.check.module(self.module).declaration_symbol(source) {
            self.flow_mut().mark_assigned(AssignedPlace::Symbol(symbol));
        }
    }

    /// Mark bindings introduced by one parameter as definitely assigned.
    fn mark_parameter_bindings_assigned(&mut self, id: dir::LocalNodeId<dir::Parameter>) {
        self.mark_declared_binding(id.into_any());

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
        self.mark_declared_binding(id.into_any());

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
        self.mark_declared_binding(id.into_any());

        // walk pattern field binding shape
        match self.tree.get(id) {
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
                self.mark_pattern_bindings_assigned(*pattern);
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
