use std::collections::HashSet;

use destack_dir as dir;
use destack_source::{FileId, ModuleId, NodeSpanRegion, NodeSpanType, Span};

use crate::core::{DirQueryContext, ModuleQueryContext};

/// Reference search settings for one target symbol.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SymbolReferenceSearch<'a> {
    /// Whether to include direct expression references.
    pub include_expressions: bool,
    /// Whether to include member access references.
    pub include_members: bool,
    /// Whether to include dependency item references.
    pub include_dependency_items: bool,
    /// Whether to include namespace receiver references.
    pub include_namespace_receivers: bool,
    /// Whether to skip dependency aliases that target the symbol.
    pub skip_dependency_aliases: bool,
    /// Whether to prefer name spans inside dependency items.
    pub use_dependency_name_spans: bool,
    /// An optional target name for name span resolution.
    pub target_name: Option<&'a str>,
    /// Whether expression-like references must keep the same visible name.
    pub require_target_name_match: bool,
    /// The optional file filter for collected spans.
    pub limit_file: Option<FileId>,
}

impl ModuleQueryContext<'_> {
    /// Build reference index target keys for this module.
    pub(crate) fn build_reference_targets(&self) -> Vec<dir::GlobalSymbolId> {
        let mut targets = HashSet::new();
        let dir = self.dir();
        let dir_tree = dir.view();

        // expressions
        for (expression_id, expression) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
            if let Some(target_symbol) = dir.expression_reference_target(expression_id) {
                dir.insert_reference_target_keys(&mut targets, target_symbol);
            }

            if let dir::Expression::Member { left, .. } = expression
                && let Some(receiver_symbol) = dir.namespace_receiver_symbol_target(*left)
            {
                dir.insert_reference_target_keys(&mut targets, receiver_symbol);
            }

            if let dir::Expression::Member { .. } = expression
                && let Some(member_symbol) = dir.member_access_symbol_target(expression_id)
            {
                dir.insert_reference_target_keys(&mut targets, member_symbol);
            }

            let node_id = expression_id.into_global_any(dir.module_id());
            if let Some(resolution) = dir.resolutions().member_resolution(node_id) {
                match &resolution.target {
                    dir::MemberTarget::Symbol(candidate) => {
                        dir.insert_reference_target_keys(&mut targets, candidate.symbol)
                    }
                    dir::MemberTarget::Union(candidates) => {
                        for candidate in candidates {
                            dir.insert_reference_target_keys(&mut targets, candidate.symbol);
                        }
                    }
                    dir::MemberTarget::Builtin(_) | dir::MemberTarget::Field(_) => {}
                }
            }
            if let Some(resolution) = dir.resolutions().call_resolution(node_id) {
                match &resolution.target {
                    dir::CallTarget::Symbol(candidate) => {
                        dir.insert_reference_target_keys(&mut targets, candidate.symbol)
                    }
                    dir::CallTarget::Union(candidates) => {
                        for candidate in candidates {
                            dir.insert_reference_target_keys(&mut targets, candidate.symbol);
                        }
                    }
                    dir::CallTarget::Builtin(_) | dir::CallTarget::Expression { .. } => {}
                }
            }
            if let Some(resolution) = dir.resolutions().construct_resolution(node_id) {
                let symbol = resolution.target.symbol();

                dir.insert_reference_target_keys(&mut targets, symbol);
            }

            if let dir::Expression::QualifiedReference { path, .. } = expression {
                for segment_index in 0..path.segments.len() {
                    let segment_index =
                        u16::try_from(segment_index).expect("path segment index overflow");

                    if let Some(segment_symbol) =
                        dir.path_segment_symbol_target(expression_id, segment_index)
                    {
                        dir.insert_reference_target_keys(&mut targets, segment_symbol);
                    }
                }
            }
        }

        // dependency items
        for (item_id, _item) in dir_tree.iter_nodes_of_type::<dir::DependencyItem>() {
            if let Some(symbol_id) = dir.dependency_local_symbol(item_id) {
                dir.insert_reference_target_keys(&mut targets, symbol_id);
            }

            if let Some(target_symbol) = dir.dependency_symbol_target(item_id) {
                dir.insert_reference_target_keys(&mut targets, target_symbol);
            }
        }

        targets.into_iter().collect()
    }
}

/// Check whether an expression is used as the receiver of a member access.
fn expression_is_member_receiver_expression(
    dir_tree: dir::View<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let Some(parent) = dir_tree.get_parent_for(expression_id) else {
        return false;
    };
    if parent.ty != dir::NodeType::Expression {
        return false;
    }

    let Ok(parent_expression_id) = parent.try_into() else {
        return false;
    };
    let parent_expression = dir_tree.get::<dir::Expression>(parent_expression_id);
    matches!(
        parent_expression,
        dir::Expression::Member { left, .. } if *left == expression_id
    )
}

impl DirQueryContext<'_> {
    /// Insert the usable reference target keys for one observed symbol.
    fn insert_reference_target_keys(
        self,
        targets: &mut HashSet<dir::GlobalSymbolId>,
        symbol_id: dir::GlobalSymbolId,
    ) {
        targets.insert(symbol_id);
        targets.insert(self.canonical_symbol(symbol_id));
    }

    /// Collect symbol references within a query context.
    pub(crate) fn symbol_references(
        self,
        canonical_id: dir::GlobalSymbolId,
        options: SymbolReferenceSearch<'_>,
    ) -> Vec<Span> {
        let ctx = self;
        // initialize the collected span list
        let mut spans = Vec::new();

        // compute namespace import aliases when namespace receiver matching is enabled
        let namespace_aliases = if options.include_namespace_receivers {
            ctx.namespace_import_aliases_for_module(canonical_id.module_id)
        } else {
            Vec::new()
        };

        // collect direct expression references
        if options.include_expressions {
            let expression_spans = ctx.collect_expression_reference_spans(canonical_id, options);
            spans.extend(expression_spans);
        }

        // collect member access references
        if options.include_members {
            let member_spans =
                ctx.collect_member_reference_spans(canonical_id, options, &namespace_aliases);
            spans.extend(member_spans);
        }

        // collect dependency item references
        if options.include_dependency_items {
            let dependency_spans = ctx.collect_dependency_reference_spans(canonical_id, options);
            spans.extend(dependency_spans);
        }

        // return the collected spans
        spans
    }

    /// Collect direct expression reference spans.
    fn collect_expression_reference_spans(
        self,
        canonical_id: dir::GlobalSymbolId,
        options: SymbolReferenceSearch<'_>,
    ) -> Vec<Span> {
        let ctx = self;
        // collect matching expression ids first to avoid borrow issues
        let matching_expression_ids: Vec<_> = {
            let dir_tree = ctx.view();
            dir_tree
                .iter_nodes_of_type::<dir::Expression>()
                .into_iter()
                .filter_map(|(expression_id, expression)| {
                    let resolved_target_symbol = ctx.expression_reference_target(expression_id);

                    // skip member expressions when member references are enabled
                    if options.include_members
                        && matches!(expression, dir::Expression::Member { .. })
                    {
                        return None;
                    }
                    // skip dependency aliases when requested
                    let is_alias = if options.skip_dependency_aliases {
                        resolved_target_symbol
                            .map(|target_symbol| {
                                ctx.is_dependency_alias_for_target(target_symbol, canonical_id)
                            })
                            .unwrap_or(false)
                    } else {
                        false
                    };
                    if is_alias {
                        return None;
                    }

                    // check for a canonical target match
                    if let Some(target_symbol) = resolved_target_symbol {
                        if ctx.symbol_matches_reference_target(target_symbol, canonical_id) {
                            return Some(expression_id);
                        }

                        return None;
                    }

                    None
                })
                .collect()
        };

        // resolve spans for the matching expressions
        let dir_tree = ctx.view();
        let mut spans = Vec::new();
        for expression_id in matching_expression_ids {
            let span = ctx.resolve_expression_reference_span(dir_tree, expression_id);
            let Some(span) = span else {
                continue;
            };

            // keep rename style collection on the same visible name
            if !ctx.span_matches_target_name(span, options) {
                continue;
            }

            // filter out spans outside the requested file
            if options
                .limit_file
                .is_some_and(|limit_file| span.file != limit_file)
            {
                continue;
            }

            spans.push(span);
        }

        // capture plain path segments from multi segment path expressions
        for (expression_id, expression) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
            let path = match expression {
                dir::Expression::QualifiedReference { path, .. } => path,
                _ => continue,
            };
            if path.segments.len() < 2 {
                continue;
            }

            for segment_index in 0..path.segments.len() {
                let segment_index =
                    u16::try_from(segment_index).expect("path reference segment index overflow");

                let Some(segment_symbol) =
                    ctx.path_segment_symbol_target(expression_id, segment_index)
                else {
                    continue;
                };
                if !ctx.symbol_matches_reference_target(segment_symbol, canonical_id) {
                    continue;
                }

                let Some(span) = ctx.path_segment_span(expression_id, segment_index) else {
                    continue;
                };

                // keep rename style collection on the same visible name
                if !ctx.span_matches_target_name(span, options) {
                    continue;
                }

                if options
                    .limit_file
                    .is_some_and(|limit_file| span.file != limit_file)
                {
                    continue;
                }

                spans.push(span);
            }
        }

        // capture member receiver spans when the receiver matches the target symbol
        if options.include_members {
            let dir_tree = ctx.view();
            for (expression_id, expression) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
                let dir::Expression::Member { left, .. } = expression else {
                    continue;
                };
                if !ctx.member_receiver_matches_target(
                    dir_tree,
                    *left,
                    canonical_id,
                    options.target_name,
                ) {
                    continue;
                }

                let span = ctx.resolve_member_receiver_reference_span(expression_id, *left);
                let Some(span) = span else {
                    continue;
                };

                // keep rename style collection on the same visible name
                if !ctx.span_matches_target_name(span, options) {
                    continue;
                }

                if let Some(limit_file) = options.limit_file
                    && span.file != limit_file
                {
                    continue;
                }

                spans.push(span);
            }
        }

        // return the matching spans
        spans
    }

    /// Check whether one collected span keeps the requested target name.
    fn span_matches_target_name(self, span: Span, options: SymbolReferenceSearch<'_>) -> bool {
        let ctx = self;
        if !options.require_target_name_match {
            return true;
        }

        let Some(target_name) = options.target_name else {
            return true;
        };

        let Some(file) = ctx
            .repository()
            .file(ctx.revision(), span.file)
            .ok()
            .flatten()
        else {
            return false;
        };
        let file_text = file.text();
        let start = span.start as usize;
        let end = span.end as usize;
        if start > end || end > file_text.len() {
            return false;
        }

        file_text
            .get(start..end)
            .is_some_and(|span_text| span_text == target_name)
    }

    /// Check whether a member receiver expression matches the canonical symbol target.
    fn member_receiver_matches_target(
        self,
        _dir_tree: dir::View<'_>,
        receiver_expression_id: dir::LocalNodeId<dir::Expression>,
        canonical_id: dir::GlobalSymbolId,
        _target_name: Option<&str>,
    ) -> bool {
        let ctx = self;
        let receiver_symbol = ctx.namespace_receiver_symbol_target(receiver_expression_id);
        let Some(target_symbol) = receiver_symbol else {
            return false;
        };

        ctx.symbol_matches_reference_target(target_symbol, canonical_id)
    }

    /// Return the target symbol for an expression reference.
    fn expression_reference_target(
        self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        let ctx = self;
        if expression_is_member_receiver_expression(ctx.view(), expression_id) {
            return ctx.namespace_receiver_symbol_target(expression_id);
        }

        ctx.expression_symbol_target(expression_id)
    }

    /// Resolve the best span for a reference expression.
    fn resolve_expression_reference_span(
        self,
        dir_tree: dir::View<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<Span> {
        let ctx = self;
        let span = ctx.get_node_tree_main_span(ctx.view(), expression_id.into());

        let Some(parent) = dir_tree.get_parent_for(expression_id) else {
            return Some(span);
        };
        if parent.ty != dir::NodeType::Expression {
            return Some(span);
        }

        let Ok(parent_expression_id) = parent.try_into() else {
            return Some(span);
        };
        let parent_expression = dir_tree.get::<dir::Expression>(parent_expression_id);
        let dir::Expression::Member { left, .. } = parent_expression else {
            return Some(span);
        };
        if *left != expression_id {
            return Some(span);
        }

        let Some(member_name_span) = ctx.member_access_name_span(parent_expression_id) else {
            return Some(span);
        };
        let receiver_end = member_name_span.start.saturating_sub(1);

        // clamp spans that include `.member` to only the receiver expression
        if span.file == member_name_span.file
            && span.start < receiver_end
            && span.end >= receiver_end
        {
            return Some(Span::new(span.file, span.start, receiver_end));
        }

        // derive receiver spans when direct mapping points at member names
        let member_span = ctx.get_node_tree_span(ctx.view(), parent_expression_id.into());
        if member_span.file == member_name_span.file && member_span.start < receiver_end {
            return Some(Span::new(member_span.file, member_span.start, receiver_end));
        }

        Some(span)
    }

    /// Resolve the best span for a member receiver expression.
    fn resolve_member_receiver_reference_span(
        self,
        member_expression_id: dir::LocalNodeId<dir::Expression>,
        receiver_expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<Span> {
        let ctx = self;
        let receiver_expression_id = {
            let dir_tree = ctx.view();
            let mut receiver_expression_id = receiver_expression_id;
            loop {
                let receiver_expression = dir_tree.get::<dir::Expression>(receiver_expression_id);
                let dir::Expression::Parenthesized { expression } = receiver_expression else {
                    break;
                };
                receiver_expression_id = *expression;
            }
            receiver_expression_id
        };

        let receiver_span = ctx
            .parsed_expression_span_for_dir_expression(receiver_expression_id)
            .unwrap_or_else(|| {
                ctx.get_node_tree_main_span(ctx.view(), receiver_expression_id.into())
            });

        let Some(member_name_span) = ctx.member_access_name_span(member_expression_id) else {
            return Some(receiver_span);
        };
        let receiver_end = member_name_span.start.saturating_sub(1);

        // clamp spans that include `.member` to only the receiver expression
        if receiver_span.file == member_name_span.file
            && receiver_span.start < receiver_end
            && receiver_span.end >= receiver_end
        {
            return Some(Span::new(
                receiver_span.file,
                receiver_span.start,
                receiver_end,
            ));
        }

        // derive receiver spans from member expression spans when needed
        let member_span = ctx.get_node_tree_span(ctx.view(), member_expression_id.into());
        if member_span.file == member_name_span.file && member_span.start < receiver_end {
            return Some(Span::new(member_span.file, member_span.start, receiver_end));
        }

        Some(receiver_span)
    }

    /// Resolve the main source span for a DIR expression source id.
    fn parsed_expression_span_for_dir_expression(
        self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<Span> {
        let ctx = self;
        let source_id = ctx.view().get_source(expression_id);

        let expression_id = dir::LocalNodeId::<dir::Expression>::new(source_id);
        ctx.source_expression_main_span_without_parentheses(expression_id)
    }

    /// Resolve a source expression main span, unwrapping parenthesized expressions.
    fn source_expression_main_span_without_parentheses(
        self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<Span> {
        let ctx = self;
        let mut expression_id = expression_id;
        loop {
            let expression = ctx.tree().get(expression_id);
            let dir::Expression::Parenthesized { expression } = expression else {
                break;
            };
            expression_id = *expression;
        }

        let expression = ctx.tree().get(expression_id);
        if matches!(
            expression,
            dir::Expression::Identifier { .. } | dir::Expression::QualifiedReference { .. }
        ) {
            let expression_span = ctx.tree().source_index.get(expression_id.id);
            if let Some(identifier_span) = ctx.source_identifier_span_in_expression(expression_span)
            {
                return Some(identifier_span);
            }
        }

        let span = ctx
            .tree()
            .source_index
            .get_main(expression_id.id)
            .unwrap_or_else(|| ctx.tree().source_index.get(expression_id.id));
        Some(Span::new(ctx.file_id(), span.start, span.end))
    }

    /// Resolve the last identifier token span inside an expression span.
    fn source_identifier_span_in_expression(self, expression_span: Span) -> Option<Span> {
        let ctx = self;
        let mut source_identifier_span = None;
        for token in ctx.tokens() {
            if token.span.file != ctx.file_id() {
                continue;
            }
            if token.token.ty() != dir::TokenType::Identifier {
                continue;
            }
            if token.span.start < expression_span.start || token.span.end > expression_span.end {
                continue;
            }

            source_identifier_span = Some(token.span);
        }

        source_identifier_span
    }

    /// Collect member access reference spans.
    fn collect_member_reference_spans(
        self,
        canonical_id: dir::GlobalSymbolId,
        options: SymbolReferenceSearch<'_>,
        namespace_aliases: &[dir::GlobalSymbolId],
    ) -> Vec<Span> {
        let ctx = self;
        // initialize the collected span list
        let mut spans = Vec::new();

        // scan member expressions for matches and namespace receivers
        let dir_tree = ctx.view();
        for (expression_id, expression) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
            let dir::Expression::Member { left, name, .. } = expression else {
                continue;
            };
            let Some(name) = name else {
                continue;
            };

            // use the recorded member target first
            if let Some(member_symbol) = ctx.member_access_symbol_target(expression_id) {
                if !ctx.symbol_matches_reference_target(member_symbol, canonical_id) {
                    continue;
                }

                let Some(span) = ctx.member_access_name_span(expression_id) else {
                    continue;
                };

                // filter out spans outside the requested file
                if let Some(limit_file) = options.limit_file
                    && span.file != limit_file
                {
                    continue;
                }

                spans.push(span);
                continue;
            }

            // accept recorded dynamic candidate matches when one member access has no single target
            if ctx.member_resolution_matches_reference_target(expression_id, canonical_id) {
                let Some(span) = ctx.member_access_name_span(expression_id) else {
                    continue;
                };

                if let Some(limit_file) = options.limit_file
                    && span.file != limit_file
                {
                    continue;
                }

                spans.push(span);
                continue;
            }

            // apply namespace import receiver matching when enabled
            if !options.include_namespace_receivers {
                continue;
            }

            // require a target name for namespace member matching
            let Some(target_name) = options.target_name else {
                continue;
            };

            // use the recorded receiver target and require it to be a namespace alias
            let Some(receiver_symbol) = ctx.namespace_receiver_symbol_target(*left) else {
                continue;
            };
            if !namespace_aliases.contains(&receiver_symbol) {
                continue;
            }

            // require the member name to match the requested target name
            let member_name = ctx.strings().get(*name).to_string();
            if member_name != target_name {
                continue;
            }

            let Some(span) = ctx.member_access_name_span(expression_id) else {
                continue;
            };

            // filter out spans outside the requested file
            if let Some(limit_file) = options.limit_file
                && span.file != limit_file
            {
                continue;
            }

            spans.push(span);
        }

        // return the collected spans
        spans
    }

    /// Collect dependency item reference spans.
    fn collect_dependency_reference_spans(
        self,
        canonical_id: dir::GlobalSymbolId,
        options: SymbolReferenceSearch<'_>,
    ) -> Vec<Span> {
        let ctx = self;
        // collect matching dependency item ids first to avoid borrow issues
        let matching_dependency_ids: Vec<_> = {
            let dir_tree = ctx.view();
            dir_tree
                .iter_nodes_of_type::<dir::DependencyItem>()
                .into_iter()
                .filter_map(|(item_id, _)| {
                    let target = ctx.dependency_symbol_target(item_id)?;
                    ctx.symbol_matches_reference_target(target, canonical_id)
                        .then_some(item_id)
                })
                .collect()
        };

        // resolve spans for the matching dependency items
        let mut spans = Vec::new();
        for item_id in matching_dependency_ids {
            let span = if options.use_dependency_name_spans {
                ctx.dependency_item_name_span(item_id, options.target_name)
            } else {
                None
            }
            .or_else(|| Some(ctx.get_node_tree_main_span(ctx.view(), item_id.into())));

            let Some(span) = span else {
                continue;
            };

            // keep rename style collection on the same visible name
            if !ctx.span_matches_target_name(span, options) {
                continue;
            }

            // filter out spans outside the requested file
            if let Some(limit_file) = options.limit_file
                && span.file != limit_file
            {
                continue;
            }

            spans.push(span);
        }

        // return the collected spans
        spans
    }

    /// Resolve a dependency item's name span when a target name is provided.
    fn dependency_item_name_span(
        self,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
        target_name: Option<&str>,
    ) -> Option<Span> {
        let ctx = self;
        // use the main span when no name was provided
        let Some(target_name) = target_name else {
            return Some(ctx.get_node_tree_main_span(ctx.view(), item_id.into()));
        };

        // resolve name + alias for matching
        let dir_tree = ctx.view();
        let item = dir_tree.get::<dir::DependencyItem>(item_id);
        let (name_id, alias_id) = match item {
            dir::DependencyItem::Binding { name, alias, .. } => {
                (name.map(|name| name.string()), *alias)
            }
            dir::DependencyItem::Error => return None,
        };

        // resolve the source node id for span lookup
        let source_node_id = dir_tree.get_source(item_id);

        // match the remote/local item name first
        if let Some(name_id) = name_id
            && ctx.strings().get(name_id) == target_name
        {
            let span = ctx
                .tree()
                .get_side_span_by_id(source_node_id, NodeSpanType::Region(NodeSpanRegion::Type))?;
            return Some(Span::new(ctx.file_id(), span.start, span.end));
        }

        // use alias when present
        if let Some(alias_id) = alias_id
            && ctx.strings().get(alias_id) == target_name
        {
            let span = ctx
                .tree()
                .get_side_span_by_id(source_node_id, NodeSpanType::Main)?;
            return Some(Span::new(ctx.file_id(), span.start, span.end));
        }

        None
    }

    /// Collect namespace import aliases that target a module.
    fn namespace_import_aliases_for_module(self, module_id: ModuleId) -> Vec<dir::GlobalSymbolId> {
        let ctx = self;
        // scan dependency items for namespace imports to the target module
        let dir_tree = ctx.view();
        dir_tree
            .iter_nodes_of_type::<dir::DependencyItem>()
            .into_iter()
            .filter_map(|(item_id, item)| {
                let dir::DependencyItem::Binding { binding, form, .. } = item else {
                    return None;
                };

                // require namespace value imports with a concrete symbol
                if *binding != dir::DependencyBinding::Namespace
                    || *form != Some(dir::DependencyForm::Plain)
                {
                    return None;
                }

                let local_symbol = ctx.symbol_for_node(item_id.into())?;
                let node_id = item_id.into_global_any(ctx.module_id());
                let target_module_id = ctx
                    .modules()
                    .target_for_source(node_id, dir::ModuleRelation::Import)?;
                if target_module_id != module_id {
                    return None;
                }

                Some(dir::GlobalSymbolId::new(ctx.module_id(), local_symbol))
            })
            .collect()
    }

    /// Check whether one member access resolution candidate set matches the target.
    fn member_resolution_matches_reference_target(
        self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        canonical_id: dir::GlobalSymbolId,
    ) -> bool {
        let ctx = self;
        // only genuinely dynamic accesses should reach this path
        let Some(resolution) = ctx
            .resolutions()
            .member_resolution(expression_id.into_global_any(ctx.module_id()))
        else {
            return false;
        };

        match &resolution.target {
            dir::MemberTarget::Union(candidates) => candidates.iter().any(|candidate| {
                ctx.symbol_matches_reference_target(candidate.symbol, canonical_id)
            }),
            _ => false,
        }
    }
}
