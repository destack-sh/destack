use destack_dir as dir;
use destack_source::{FileId, NodeSpanList, NodeSpanRegion, NodeSpanType, Span};

use crate::{ModuleQueryContext, QueryError, QueryResult};

/// One symbol occurrence at an authored source span.
#[derive(Debug, Clone)]
pub(crate) struct SymbolOccurrence {
    /// The recorded symbols in declaration order.
    pub symbols: Vec<dir::GlobalSymbolId>,
    /// The checked type selected at this occurrence.
    pub type_id: Option<dir::GlobalTypeId>,
    /// The authored occurrence span.
    pub span: Span,
}

/// One checked type occurrence at an authored source span.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TypeOccurrence {
    /// The checked type selected at this occurrence.
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

impl ModuleQueryContext<'_> {
    /// Return the recorded semantic symbol occurrence at an authored span.
    pub(crate) fn symbol_at_offset(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<SymbolOccurrence>> {
        self.occurrence_at_offset(file_id, offset, |view, node_id, span, offset| {
            self.symbol_occurrence(view, node_id, span, offset)
        })
    }

    /// Return the checked type occurrence at an authored span.
    pub(crate) fn type_at_offset(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<TypeOccurrence>> {
        self.occurrence_at_offset(file_id, offset, |_view, node_id, span, _offset| {
            let node_id = node_id.into_global(self.module_id());
            let Some(type_id) = self.types()?.get_node_type_id(node_id) else {
                return Ok(None);
            };

            Ok(Some(TypeOccurrence { type_id, span }))
        })
    }

    /// Return the recorded declaration occurrence at an authored span.
    pub(crate) fn declaration_at_offset(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<SymbolOccurrence>> {
        self.occurrence_at_offset(file_id, offset, |view, node_id, span, offset| {
            self.declaration_occurrence(view, node_id, span, offset)
        })
    }

    /// Return the identity used by a reference search at an authored span.
    pub(crate) fn reference_at_offset(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<SymbolOccurrence>> {
        self.occurrence_at_offset(file_id, offset, |view, node_id, span, offset| {
            // explicit import aliases retain their local declaration identity
            let declaration = self.declaration_occurrence(view, node_id, span, offset)?;
            let is_local_alias = match declaration.as_ref().and_then(SymbolOccurrence::symbol) {
                Some(symbol) => self.is_local_import_alias(symbol)?,
                None => false,
            };
            if is_local_alias {
                return Ok(declaration);
            }

            // all other occurrences use their final checked targets
            self.symbol_occurrence(view, node_id, span, offset)
        })
    }

    /// Find one occurrence by visiting authored source owners at an offset.
    fn occurrence_at_offset<T>(
        &self,
        file_id: FileId,
        offset: u32,
        mut occurrence_at_node: impl FnMut(
            dir::View<'_>,
            dir::LocalNodeIdAny,
            Span,
            u32,
        ) -> QueryResult<Option<T>>,
    ) -> QueryResult<Option<T>> {
        // exclude comments from semantic occurrences
        let is_comment = self
            .comments(file_id)?
            .iter()
            .any(|comment| comment.span.contains(offset));
        if is_comment {
            return Ok(None);
        }

        // visit authored source owners from smallest to largest
        let enclosing = self.enclosing_spans_at_cursor(file_id, offset)?;
        let view = self.view()?;
        for enclosing_span in enclosing {
            let Some(node_id) = view.get_node_id_by_source_id(enclosing_span.source_id) else {
                continue;
            };
            let main_span = self
                .source_index()?
                .get_main_or_enclosing(enclosing_span.source_id);

            // select the authored name inside nodes that own several names
            let span = if node_id.ty == dir::NodeType::DependencyItem {
                main_span
            } else {
                let Some(span) = self.name_span(view, node_id, main_span, offset)? else {
                    continue;
                };

                span
            };

            if let Some(occurrence) = occurrence_at_node(view, node_id, span, offset)? {
                return Ok(Some(occurrence));
            }
        }

        Ok(None)
    }

    /// Return the authored name span selected inside one DIR node.
    fn name_span(
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

    /// Return the occurrence introduced by one named binding.
    fn binding_occurrence(
        &self,
        node_id: dir::LocalNodeIdAny,
        span: Span,
    ) -> QueryResult<Option<SymbolOccurrence>> {
        let Some(symbol_id) = self.node_symbol(node_id)? else {
            return Ok(None);
        };
        if self.bindings()?.get_symbol(symbol_id).name().is_none() {
            return Ok(None);
        }
        let source = node_id.into_global(self.module_id());

        Ok(Some(SymbolOccurrence {
            symbols: vec![symbol_id.into_global(self.module_id())],
            type_id: self.types()?.get_node_type_id(source),
            span,
        }))
    }

    /// Return declaration symbols for one DIR node at its authored span.
    fn declaration_occurrence(
        &self,
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

        // select only recorded identities for qualified type path segments
        if let Some((segment, segment_count)) = self.qualified_type_segment(view, node_id, span)? {
            let Some(symbols) = self.qualified_type_targets(source, segment, segment_count)? else {
                return Ok(None);
            };

            return Ok(Some(SymbolOccurrence {
                symbols,
                type_id: self.types()?.get_node_type_id(source),
                span,
            }));
        }

        // imported path roots retain their local declaration identities
        if let Some(symbols) = self.resolved()?.references.declarations(source) {
            let root_span = self.reference_root_span(view, node_id, span)?;
            if root_span.owns_cursor(offset) {
                return Ok(Some(SymbolOccurrence {
                    symbols: symbols.to_vec(),
                    type_id: self.types()?.get_node_type_id(source),
                    span: root_span,
                }));
            }
        }

        // prefer the final checked use-site selection
        if let Some(symbols) = self.recorded_symbol_targets(source)? {
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
        self.binding_occurrence(node_id, span)
    }

    /// Return the first authored name span for one reference node.
    fn reference_root_span(
        &self,
        view: dir::View<'_>,
        node_id: dir::LocalNodeIdAny,
        selected_span: Span,
    ) -> QueryResult<Span> {
        if node_id.ty != dir::NodeType::TypeExpression {
            return Ok(selected_span);
        }

        let type_id = dir::LocalNodeId::<dir::TypeExpression>::new(node_id.id);
        let dir::TypeExpression::Reference { path, .. } = view.get(type_id) else {
            return Ok(selected_span);
        };
        if path.segments.len() == 1 {
            return Ok(selected_span);
        }

        let source_id = view.get_source(type_id);
        let root = NodeSpanType::ListItem(NodeSpanList::Segment, 0);

        let node = node_id.into_global(self.module_id());
        let span = self
            .source_index()?
            .get_side(source_id, root)
            .ok_or(QueryError::missing(format!(
                "type reference span: {:?}, {:?}",
                node, 0
            )))?;

        Ok(span)
    }

    /// Return the recorded symbols for one DIR node at its authored span.
    fn symbol_occurrence(
        &self,
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

        // select only recorded identities for qualified type path segments
        if let Some((segment, segment_count)) = self.qualified_type_segment(view, node_id, span)? {
            let Some(symbols) =
                self.qualified_type_targets(global_node_id, segment, segment_count)?
            else {
                return Ok(None);
            };

            return Ok(Some(SymbolOccurrence {
                symbols,
                type_id: self.types()?.get_node_type_id(global_node_id),
                span,
            }));
        }

        // use the checked decorator selection for decorator targets
        if node_id.ty == dir::NodeType::Expression {
            let expression_id = dir::LocalNodeId::<dir::Expression>::new(node_id.id);
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
            if let Some(call_id) = self.callee_call(view, expression_id) {
                let occurrence = self.selected_call_occurrence(call_id, span)?;

                return Ok(occurrence);
            }
        }

        // use sites return only the identities recorded by checking
        if let Some(mut symbols) = self.recorded_symbol_targets(global_node_id)? {
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
        self.binding_occurrence(node_id, span)
    }

    /// Return the selected segment in one qualified type reference.
    fn qualified_type_segment(
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
        // the root retains its lexical declaration identities
        if segment == 0
            && let Some(declarations) = self.resolved()?.references.declarations(source)
        {
            return Ok(Some(declarations.to_vec()));
        }

        // nominal patterns retain their exact selected variant separately
        if segment + 1 == segment_count
            && let Some(symbol) = self.pattern_variant_symbol(source)?
        {
            return Ok(Some(vec![symbol]));
        }

        let reference = self
            .resolved()?
            .references
            .get(source)
            .ok_or(QueryError::missing(format!(
                "qualified reference: {source:?}"
            )))?;
        let targets = match reference {
            // the final bound segment receives the checker's selected declaration
            dir::Reference::Bound(_) if segment + 1 == segment_count => {
                self.recorded_symbol_targets(source)?
            }

            // the bound prefix receives its exact projected base declaration
            dir::Reference::Projected { base, from }
                if segment + 1
                    == usize::try_from(*from).map_err(|_| {
                        QueryError::invalid(format!("projected segment: {source:?}"))
                    })? =>
            {
                Some(vec![*base])
            }

            // intermediate and unresolved segments have no recorded symbol identity
            dir::Reference::Bound(_)
            | dir::Reference::Namespace(_)
            | dir::Reference::Projected { .. }
            | dir::Reference::Ambiguous(_)
            | dir::Reference::Missing => None,
        };

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
        let resolution =
            self.resolutions()?
                .pattern_resolution(pattern)
                .ok_or(QueryError::missing(format!(
                    "qualified pattern: {pattern:?}"
                )))?;
        match resolution {
            dir::PatternResolution::Variant(resolution) => Ok(Some(resolution.case.variant)),
            dir::PatternResolution::Destructure(_) => Ok(None),
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
        let construct = self.resolutions()?.construct_resolution(call_id);
        let call = self.resolutions()?.call_resolution(call_id);

        // require one authoritative call selection
        if construct.is_some() && call.is_some() {
            return Err(QueryError::conflict(format!(
                "call resolution columns: {call_id:?}"
            )));
        }

        // read nominal calls from their exact checked construction
        if let Some(resolution) = construct {
            return Ok(Some(SymbolOccurrence {
                symbols: resolution.target.symbol().into_iter().collect(),
                type_id: Some(resolution.return_type),
                span,
            }));
        }

        // otherwise read an ordinary checked call selection
        let Some(resolution) = call else {
            return Ok(None);
        };
        let symbols = resolution.target_symbols();
        if symbols.is_empty() {
            return Ok(None);
        }

        Ok(Some(SymbolOccurrence {
            symbols,
            type_id: resolution.shared_callable_type(),
            span,
        }))
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
            let symbols = self.dependency_symbol_targets(item_id)?;
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
