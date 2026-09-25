use tspp_dir as dir;
use tspp_source::{NodeSpanList, NodeSpanRegion, NodeSpanType, Span};

use crate::cursor::Cursor;
use crate::{ModuleQueryContext, ProgramQueryContext, QueryError, QueryResult};

/// One symbol occurrence at an authored source span.
#[derive(Debug, Clone)]
pub(crate) struct SymbolOccurrence {
    /// The recorded symbols in declaration order.
    pub symbols: Vec<dir::GlobalSymbolId>,
    /// The resolved type selected at this occurrence.
    pub type_id: Option<dir::GlobalTypeId>,
    /// The authored occurrence span.
    pub span: Span,
}

/// One type occurrence at an authored source span.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TypeOccurrence {
    /// The resolved type selected at this occurrence.
    pub type_id: dir::GlobalTypeId,
    /// The authored occurrence span.
    pub span: Span,
}

impl SymbolOccurrence {
    /// Return the symbol when this occurrence names exactly one declaration.
    pub(crate) fn symbol(&self) -> Option<dir::GlobalSymbolId> {
        let [symbol] = self.symbols.as_slice() else {
            return None;
        };

        Some(*symbol)
    }
}

impl Cursor<'_, '_> {
    /// Return the recorded symbol occurrence at an authored span.
    pub(crate) fn symbol(
        &self,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<Option<SymbolOccurrence>> {
        self.occurrence(|view, node_id, span, offset| {
            self.module
                .symbol_occurrence(program, view, node_id, span, offset)
        })
    }

    /// Return the type occurrence at an authored span.
    pub(crate) fn ty(&self) -> QueryResult<Option<TypeOccurrence>> {
        self.occurrence(|_view, node_id, span, _offset| {
            let node_id = node_id.into_global(self.module.module_id());
            let Some(type_id) = self.module.types()?.get_node_type_id(node_id) else {
                return Ok(None);
            };

            Ok(Some(TypeOccurrence { type_id, span }))
        })
    }

    /// Return the recorded declaration occurrence at an authored span.
    pub(crate) fn declaration(
        &self,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<Option<SymbolOccurrence>> {
        self.occurrence(|view, node_id, span, offset| {
            self.module
                .declaration_occurrence(program, view, node_id, span, offset)
        })
    }

    /// Return the identity used by a reference search at an authored span.
    pub(crate) fn reference(
        &self,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<Option<SymbolOccurrence>> {
        self.occurrence(|view, node_id, span, offset| {
            // explicit import aliases retain their local declaration identity
            let declaration = self
                .module
                .declaration_occurrence(program, view, node_id, span, offset)?;
            let is_local_alias = match declaration.as_ref().and_then(SymbolOccurrence::symbol) {
                Some(symbol) => self.module.is_local_import_alias(symbol)?,
                None => false,
            };
            let is_definition_member = match declaration.as_ref().and_then(SymbolOccurrence::symbol)
            {
                Some(symbol) => {
                    let module = program.module(symbol.module_id)?;

                    module.definition_member(program, symbol)?.is_some()
                }
                None => false,
            };
            if is_local_alias || is_definition_member {
                return Ok(declaration);
            }

            // all other occurrences use their selected targets
            self.module
                .symbol_occurrence(program, view, node_id, span, offset)
        })
    }

    /// Find one occurrence by visiting authored source owners at an offset.
    fn occurrence<T>(
        &self,
        mut occurrence_at_node: impl FnMut(
            dir::View<'_>,
            dir::LocalNodeIdAny,
            Span,
            u32,
        ) -> QueryResult<Option<T>>,
    ) -> QueryResult<Option<T>> {
        // exclude comments from symbol occurrences
        let is_comment = self
            .module
            .comments(self.file_id)?
            .iter()
            .any(|comment| comment.span.contains(self.offset));
        if is_comment {
            return Ok(None);
        }

        // visit authored source owners from smallest to largest
        let enclosing = self.enclosing();
        let view = self.module.view()?;
        let index = self.module.source_index()?;
        for enclosing_span in enclosing {
            let Some(node_id) = view.get_node_id_by_source_id(enclosing_span.source_id) else {
                continue;
            };
            let main_span = index.get_main_or_enclosing(enclosing_span.source_id);

            // select the authored name inside nodes that own several names
            let span = if node_id.ty == dir::NodeType::DependencyItem {
                main_span
            } else {
                let Some(span) = self
                    .module
                    .name_span(view, node_id, main_span, self.offset)?
                else {
                    continue;
                };

                span
            };

            if let Some(occurrence) = occurrence_at_node(view, node_id, span, self.offset)? {
                return Ok(Some(occurrence));
            }
        }

        Ok(None)
    }
}

impl ModuleQueryContext<'_> {
    /// Return the authored name span selected inside one DIR node.
    pub(crate) fn name_span(
        &self,
        view: dir::View<'_>,
        node_id: dir::LocalNodeIdAny,
        main_span: Span,
        offset: u32,
    ) -> QueryResult<Option<Span>> {
        if node_id.ty != dir::NodeType::TypeExpression {
            return Ok(main_span.owns_cursor(offset).then_some(main_span));
        }

        let type_id = dir::LocalNodeId::<dir::TypeExpression>::new(node_id.id);
        let dir::TypeExpression::Reference { path, .. } = view.get(type_id) else {
            return Ok(main_span.owns_cursor(offset).then_some(main_span));
        };
        if path.segments.len() == 1 {
            return Ok(main_span.owns_cursor(offset).then_some(main_span));
        }

        let source_id = view.get_source(type_id);
        let node = node_id.into_global(self.module_id());
        for (index, _) in path.segments.iter().enumerate() {
            let index = u16::try_from(index)
                .map_err(|_| QueryError::invalid(format!("type reference path: {node:?}")))?;
            let span_type = NodeSpanType::ListItem(NodeSpanList::Segment, index);
            let span =
                self.source_index()?
                    .get_side(source_id, span_type)
                    .ok_or(QueryError::missing(format!(
                        "type reference span: {node:?}, {index:?}"
                    )))?;
            if span.owns_cursor(offset) {
                return Ok(Some(span));
            }
        }

        Ok(None)
    }

    /// Return the occurrence introduced by one authored binding.
    fn binding_occurrence(
        &self,
        program: &ProgramQueryContext<'_>,
        node_id: dir::LocalNodeIdAny,
        span: Span,
    ) -> QueryResult<Option<SymbolOccurrence>> {
        let source = node_id.into_global(self.module_id());
        let symbols = match self.node_symbol(node_id)? {
            Some(symbol) => vec![symbol.into_global(self.module_id())],
            None => program
                .member_index(self.module_id())?
                .source_entries(source)
                .map(|member| member.symbol)
                .collect(),
        };
        if symbols.is_empty() {
            return Ok(None);
        }

        let bindings = self.bindings()?;
        let mut is_named = false;
        for symbol_id in &symbols {
            let symbol = bindings.get_symbol(symbol_id.local_id);
            let is_symbol_named = if symbol.name().is_some() {
                true
            } else {
                self.definition_member(program, *symbol_id)?
                    .is_some_and(|(_, _, member)| member.is_named())
            };
            if is_symbol_named {
                is_named = true;

                break;
            }
        }
        if !is_named {
            return Ok(None);
        }

        Ok(Some(SymbolOccurrence {
            symbols,
            type_id: self.types()?.get_node_type_id(source),
            span,
        }))
    }

    /// Return declaration symbols for one DIR node at its authored span.
    fn declaration_occurrence(
        &self,
        program: &ProgramQueryContext<'_>,
        view: dir::View<'_>,
        node_id: dir::LocalNodeIdAny,
        span: Span,
        offset: u32,
    ) -> QueryResult<Option<SymbolOccurrence>> {
        // dependency items distinguish imported and local authored names
        if node_id.ty == dir::NodeType::DependencyItem {
            let item_id = dir::LocalNodeId::<dir::DependencyItem>::new(node_id.id);

            return self.dependency_targets_at_offset(view, item_id, offset);
        }

        let source = node_id.into_global(self.module_id());

        // definition members retain their declaration identity at their exact source
        if program
            .member_index(self.module_id())?
            .source_entries(source)
            .next()
            .is_some()
        {
            return self.binding_occurrence(program, node_id, span);
        }

        // select only recorded identities for qualified type path segments
        if let Some((segment, segment_count)) = self.qualified_type_segment(view, node_id, span)? {
            let Some(symbols) = self.qualified_type_targets(source, segment, segment_count)? else {
                return Ok(None);
            };
            let type_id = if segment + 1 == segment_count {
                self.types()?.get_node_type_id(source)
            } else {
                None
            };

            return Ok(Some(SymbolOccurrence {
                symbols,
                type_id,
                span,
            }));
        }

        // references retain their authored declaration identities
        if let Some(symbols) = self
            .resolved()?
            .references
            .declaration(source)
            .and_then(dir::Reference::symbols)
        {
            return Ok(Some(SymbolOccurrence {
                symbols: symbols.to_vec(),
                type_id: self.types()?.get_node_type_id(source),
                span,
            }));
        }

        // prefer the selected use-site target
        if let Some(symbols) = self.symbol_targets(source)? {
            return Ok(Some(SymbolOccurrence {
                symbols,
                type_id: self.types()?.get_node_type_id(source),
                span,
            }));
        }

        // resolved bindings cover references that need no checked selection
        if let Some(dir::Reference::Bound(symbols)) = self.resolved()?.references.get(source) {
            return Ok(Some(SymbolOccurrence {
                symbols: symbols.to_vec(),
                type_id: self.types()?.get_node_type_id(source),
                span,
            }));
        }

        // declaration nodes read their recorded binding
        self.binding_occurrence(program, node_id, span)
    }

    /// Return the recorded symbols for one DIR node at its authored span.
    fn symbol_occurrence(
        &self,
        program: &ProgramQueryContext<'_>,
        view: dir::View<'_>,
        node_id: dir::LocalNodeIdAny,
        span: Span,
        offset: u32,
    ) -> QueryResult<Option<SymbolOccurrence>> {
        // dependency items distinguish imported and local authored names
        if node_id.ty == dir::NodeType::DependencyItem {
            let item_id = dir::LocalNodeId::<dir::DependencyItem>::new(node_id.id);

            return self.dependency_targets_at_offset(view, item_id, offset);
        }

        let global_node_id = node_id.into_global(self.module_id());

        // definition members retain their declaration identity at their exact source
        if program
            .member_index(self.module_id())?
            .source_entries(global_node_id)
            .next()
            .is_some()
        {
            return self.binding_occurrence(program, node_id, span);
        }

        // select only recorded identities for qualified type path segments
        if let Some((segment, segment_count)) = self.qualified_type_segment(view, node_id, span)? {
            let Some(symbols) =
                self.qualified_type_targets(global_node_id, segment, segment_count)?
            else {
                return Ok(None);
            };
            let type_id = if segment + 1 == segment_count {
                self.types()?.get_node_type_id(global_node_id)
            } else {
                None
            };

            return Ok(Some(SymbolOccurrence {
                symbols,
                type_id,
                span,
            }));
        }

        // use the checked decorator selection for decorator targets
        if node_id.ty == dir::NodeType::Expression {
            let expression_id = dir::LocalNodeId::<dir::Expression>::new(node_id.id);
            if let Some(occurrence) = self.subscript_key_occurrence(view, expression_id, span)? {
                return Ok(Some(occurrence));
            }
            if let Some(application) = self.decorators()?.application_for_expression(expression_id)
            {
                let symbol = match application.resolution.target {
                    dir::DecoratorTarget::LanguageItem { symbol, .. }
                    | dir::DecoratorTarget::Symbol { symbol } => symbol,
                };

                return Ok(Some(SymbolOccurrence {
                    symbols: vec![symbol],
                    type_id: Some(application.resolution.ty),
                    span,
                }));
            }

            // use the checked call selection for ordinary call callees
            if let Some(call_id) = self.callee_call(view, expression_id)
                && let Some(occurrence) = self.selected_call_occurrence(call_id, span)?
            {
                return Ok(Some(occurrence));
            }
        }

        // use sites return only the identities recorded by checking
        if let Some(mut symbols) = self.symbol_targets(global_node_id)? {
            if symbols.is_empty() {
                return Ok(None);
            }
            symbols.sort();
            symbols.dedup();

            return Ok(Some(SymbolOccurrence {
                symbols,
                type_id: self.types()?.get_node_type_id(global_node_id),
                span,
            }));
        }

        // declarations read only their recorded binding
        self.binding_occurrence(program, node_id, span)
    }

    /// Return the checked member selected by one authored string subscript key.
    fn subscript_key_occurrence(
        &self,
        view: dir::View<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        span: Span,
    ) -> QueryResult<Option<SymbolOccurrence>> {
        if !matches!(
            view.get(expression_id),
            dir::Expression::Literal(dir::Literal::String(_))
        ) {
            return Ok(None);
        }

        // require this literal as the exact key of one index expression
        let Some(parent) = view.get_parent_for(expression_id) else {
            return Ok(None);
        };
        if parent.ty != dir::NodeType::Expression {
            return Ok(None);
        }
        let parent_id = dir::LocalNodeId::<dir::Expression>::new(parent.id);
        if !matches!(
            view.get(parent_id),
            dir::Expression::Index {
                index: Some(index), ..
            } if *index == expression_id
        ) {
            return Ok(None);
        }

        // read only the exact target retained for this subscript
        let source = parent_id.into_global_any(self.module_id());
        let Some(resolution) = self.decisions()?.subscript_decision(source) else {
            return Ok(None);
        };
        let mut symbols = Vec::new();
        for subscript in resolution.arms() {
            if let dir::SubscriptTarget::Member(member) = &subscript.target {
                member.target.collect_symbols(&mut symbols);
            }
        }
        symbols.sort();
        symbols.dedup();
        if symbols.is_empty() {
            return Ok(None);
        }
        if span.end < span.start + 2 {
            return Err(QueryError::invalid(format!(
                "subscript string key span: {span:?}"
            )));
        }
        let span = Span::new(span.file, span.start + 1, span.end - 1);

        Ok(Some(SymbolOccurrence {
            symbols,
            type_id: Some(resolution.ty()),
            span,
        }))
    }

    /// Return the selected segment in one qualified type reference.
    pub(crate) fn qualified_type_segment(
        &self,
        view: dir::View<'_>,
        node_id: dir::LocalNodeIdAny,
        selected_span: Span,
    ) -> QueryResult<Option<(usize, usize)>> {
        if node_id.ty != dir::NodeType::TypeExpression {
            return Ok(None);
        }

        let type_id = dir::LocalNodeId::<dir::TypeExpression>::new(node_id.id);
        let dir::TypeExpression::Reference { path, .. } = view.get(type_id) else {
            return Ok(None);
        };
        if path.segments.len() <= 1 {
            return Ok(None);
        }

        let source_id = view.get_source(type_id);
        let node = node_id.into_global(self.module_id());
        let mut selected = None;
        for (index, _) in path.segments.iter().enumerate() {
            let index_id = u16::try_from(index)
                .map_err(|_| QueryError::invalid(format!("type reference path: {node:?}")))?;
            let span_type = NodeSpanType::ListItem(NodeSpanList::Segment, index_id);
            let span =
                self.source_index()?
                    .get_side(source_id, span_type)
                    .ok_or(QueryError::missing(format!(
                        "type reference span: {node:?}, {index_id:?}"
                    )))?;
            if span == selected_span {
                selected = Some(index);
                break;
            }
        }
        let Some(segment) = selected else {
            return Ok(None);
        };

        Ok(Some((segment, path.segments.len())))
    }

    /// Return exact symbols recorded for one qualified type reference segment.
    fn qualified_type_targets(
        &self,
        source: dir::GlobalNodeIdAny,
        segment: usize,
        segment_count: usize,
    ) -> QueryResult<Option<Vec<dir::GlobalSymbolId>>> {
        // nominal patterns retain their exact selected variant separately
        if segment + 1 == segment_count
            && let Some(symbol) = self.pattern_variant_symbol(source)?
        {
            return Ok(Some(vec![symbol]));
        }

        // checked member segments retain their selected symbol per segment
        let index = u16::try_from(segment)
            .map_err(|_| QueryError::invalid(format!("qualified segment: {source:?}")))?;
        if let Some(resolution) = self.resolutions()?.path_resolution(source, index) {
            return Ok(Some(resolution.symbols().to_vec()));
        }

        // namespace segments retain the exact authored declaration selected by resolve
        let site = dir::ReferenceSite::Path {
            node: source,
            segment: index,
        };
        if segment + 1 == segment_count
            && matches!(
                self.resolved()?.references.get(site),
                Some(dir::Reference::Bound(_))
            )
        {
            return self.symbol_targets(source);
        }

        let targets = self
            .resolved()?
            .references
            .declaration(site)
            .and_then(dir::Reference::symbols)
            .map(<[_]>::to_vec);

        Ok(targets)
    }

    /// Return the variant selected by one nominal pattern path.
    fn pattern_variant_symbol(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> QueryResult<Option<dir::GlobalSymbolId>> {
        if source.module_id != self.module_id()
            || source.local_id.ty != dir::NodeType::TypeExpression
        {
            return Ok(None);
        }

        // select only type paths owned by nominal patterns
        let view = self.view()?;
        let Some(parent) = view.get_parent_any(source.local_id) else {
            return Ok(None);
        };
        if parent.ty != dir::NodeType::Pattern {
            return Ok(None);
        }
        let pattern_id = dir::LocalNodeId::<dir::Pattern>::new(parent.id);
        let pattern = view.get(pattern_id);
        let pattern_type = match pattern {
            dir::Pattern::NominalTuple { ty, .. } | dir::Pattern::NominalObject { ty, .. } => *ty,
            _ => return Ok(None),
        };
        if pattern_type.into_any() != source.local_id {
            return Ok(None);
        }

        // read the exact variant selected for the pattern
        let pattern = pattern_id.into_global_any(self.module_id());
        let resolution = self
            .decisions()?
            .pattern_decision(pattern)
            .ok_or(QueryError::missing(format!(
                "qualified pattern: {pattern:?}"
            )))?;
        match resolution {
            dir::PatternDecision::Variant(resolution) => Ok(Some(resolution.case.variant)),
            dir::PatternDecision::Destructure(_) => Ok(None),
            _ => Err(QueryError::invalid(format!(
                "qualified pattern: {pattern:?}"
            ))),
        }
    }

    /// Return the checked call whose callee contains one expression.
    fn callee_call(
        &self,
        view: dir::View<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalNodeIdAny> {
        let mut callee_id = expression_id;

        // cross explicit generic application wrappers
        loop {
            let parent = view.get_parent_for(callee_id)?;
            if parent.ty != dir::NodeType::Expression {
                return None;
            }

            let parent_id = dir::LocalNodeId::<dir::Expression>::new(parent.id);
            match view.get(parent_id) {
                dir::Expression::Instantiation { left, .. } if *left == callee_id => {
                    callee_id = parent_id;
                }
                dir::Expression::Call { left, .. } if *left == callee_id => {
                    return Some(parent_id.into_global_any(self.module_id()));
                }
                _ => return None,
            }
        }
    }

    /// Return the exact checked selection for one call callee.
    fn selected_call_occurrence(
        &self,
        call_id: dir::GlobalNodeIdAny,
        span: Span,
    ) -> QueryResult<Option<SymbolOccurrence>> {
        match self.decisions()?.decision(call_id) {
            Some(dir::Decision::Construct(selection)) => Ok(Some(SymbolOccurrence {
                symbols: selection.target.symbol().into_iter().collect(),
                type_id: Some(selection.return_type),
                span,
            })),
            Some(dir::Decision::Call(selection)) => {
                let symbols = selection.target_symbols();
                if symbols.is_empty() {
                    return Ok(None);
                }

                Ok(Some(SymbolOccurrence {
                    symbols,
                    type_id: selection.agreed_callable_type(),
                    span,
                }))
            }
            _ => Ok(None),
        }
    }

    /// Return the recorded dependency symbol at one authored dependency name.
    fn dependency_targets_at_offset(
        &self,
        view: dir::View<'_>,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
        offset: u32,
    ) -> QueryResult<Option<SymbolOccurrence>> {
        let node_id = item_id.into_global(self.module_id()).into_any();
        let source_id = view.get_source(item_id);
        let imported_name = NodeSpanType::Region(NodeSpanRegion::Type);

        // imported names use the resolved dependency target
        if let Some(span) = self.source_index()?.get_side(source_id, imported_name)
            && span.owns_cursor(offset)
        {
            let symbols = self.dependency_declaration_symbols(item_id)?;
            if symbols.is_empty() {
                return Ok(None);
            }

            return Ok(Some(SymbolOccurrence {
                symbols,
                type_id: self.types()?.get_node_type_id(node_id),
                span,
            }));
        }

        // local names use the recorded dependency binding
        let Some(span) = self.source_index()?.get_main(source_id) else {
            return Ok(None);
        };
        if !span.owns_cursor(offset) {
            return Ok(None);
        }

        let Some(symbol_id) = self.dependency_local_symbol(item_id)? else {
            return Ok(None);
        };

        Ok(Some(SymbolOccurrence {
            symbols: vec![symbol_id],
            type_id: self.types()?.get_node_type_id(node_id),
            span,
        }))
    }
}
