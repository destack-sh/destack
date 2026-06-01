use destack_core::{StringId, StringPool};
use destack_dir as dir;
use destack_source::{NodeSpanList, NodeSpanRegion, NodeSpanType, Span};
use std::str::FromStr;

use crate::core::{DirQueryContext, ModuleQueryContext};

/// Result of finding a symbol at an offset.
#[derive(Debug, Clone)]
pub(crate) struct SymbolAtOffset {
    /// The symbol that was referenced.
    pub symbol_id: dir::GlobalSymbolId,
    /// The DIR node that contains the reference.
    pub node_id: dir::LocalNodeIdAny,
    /// The span of the reference.
    pub span: Span,
}

impl ModuleQueryContext<'_> {
    /// Resolve the semantic target symbol used by semantic navigation queries.
    pub(crate) fn semantic_target_symbol_at_offset(
        &self,
        offset: u32,
        symbol_at: &SymbolAtOffset,
    ) -> Option<dir::GlobalSymbolId> {
        if symbol_at.node_id.ty != dir::NodeType::Expression {
            return Some(symbol_at.symbol_id);
        }

        let expression_id = symbol_at.node_id.try_into().ok()?;
        let dir = self.dir();

        // use the enclosing member target when the cursor is on a namespace receiver
        if let Some(target_symbol) =
            dir.member_access_target_symbol_at_offset(expression_id, offset)
            && target_symbol != symbol_at.symbol_id
        {
            return Some(target_symbol);
        }

        Some(symbol_at.symbol_id)
    }

    /// Resolve the local binding symbol used by declaration style queries.
    pub(crate) fn binding_symbol_at_offset(
        &self,
        offset: u32,
        symbol_at: &SymbolAtOffset,
    ) -> Option<dir::GlobalSymbolId> {
        if symbol_at.node_id.ty != dir::NodeType::Expression {
            return None;
        }

        let expression_id = symbol_at.node_id.try_into().ok()?;
        let dir = self.dir();

        // preserve plain path segments as their local binding symbols
        if let Some(result) =
            ModuleQueryContext::path_segment_symbol_at_offset(dir, expression_id, offset)
        {
            return Some(result.symbol_id);
        }

        // preserve namespace member receivers as their local binding symbols
        if let dir::Expression::Member { left, .. } =
            dir.view().get::<dir::Expression>(expression_id)
            && let Some(name_span) = dir.member_access_name_span(expression_id)
            && offset < name_span.start
        {
            return dir.namespace_receiver_symbol_target(*left);
        }

        dir.expression_symbol_target(expression_id)
    }

    /// Find the symbol for hover at a given offset.
    pub(crate) fn symbol_for_hover_at_offset(&self, offset: u32) -> Option<SymbolAtOffset> {
        self.find_symbol_at_offset(offset)
            .or_else(|| ModuleQueryContext::declaration_modifier_symbol_at_offset(self, offset))
    }
}

impl DirQueryContext<'_> {
    /// Resolve one path segment span inside a plain multi segment path expression.
    pub(crate) fn path_segment_span(
        self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        segment_index: u16,
    ) -> Option<Span> {
        let expression = self.view().get::<dir::Expression>(expression_id);
        let path = match expression {
            dir::Expression::QualifiedReference { path, .. } => path,
            _ => return None,
        };

        if usize::from(segment_index) >= path.segments.len() {
            return None;
        }

        let source_id = self.view().get_source(expression_id);
        let span = self.tree().get_side_span_by_id(
            source_id,
            NodeSpanType::ListItem(NodeSpanList::Segment, segment_index),
        )?;

        Some(Span::new(self.file_id(), span.start, span.end))
    }

    /// Resolve the span for a member access name inside its expression span.
    pub(crate) fn member_access_name_span(
        self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<Span> {
        let expression = self.view().get::<dir::Expression>(expression_id);

        // resolve the expression span from DIR source indexes
        let expression_span = self.get_node_tree_span(self.view(), expression_id.into());

        // prefer the first identifier after the receiver span
        if let dir::Expression::Member { left, .. } = expression {
            let left_span = self.get_node_tree_span(self.view(), (*left).into());

            for token in self.tokens() {
                if token.span.file != self.file_id() {
                    continue;
                }
                if token.span.start <= left_span.end {
                    continue;
                }
                if token.span.start < expression_span.start || token.span.end > expression_span.end
                {
                    continue;
                }
                if !matches!(
                    token.token.ty(),
                    dir::TokenType::Identifier | dir::TokenType::InvalidIdentifier
                ) {
                    continue;
                }

                return Some(token.span);
            }
        }

        // prefer the last identifier token in the member expression
        let mut last_identifier = None;
        for token in self.tokens() {
            if token.span.file != self.file_id() {
                continue;
            }
            if token.span.start < expression_span.start || token.span.end > expression_span.end {
                continue;
            }
            if !matches!(
                token.token.ty(),
                dir::TokenType::Identifier | dir::TokenType::InvalidIdentifier
            ) {
                continue;
            }

            last_identifier = Some(token.span);
        }
        if let Some(last_identifier) = last_identifier {
            return Some(last_identifier);
        }

        // use the source DIR main span when no identifier token is found
        let source_node_id = self.view().get_source(expression_id);
        self.tree().get_main_span_by_id(source_node_id)
    }

    /// Resolve the member target for a receiver position inside one member access.
    fn member_access_target_symbol_at_offset(
        self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        offset: u32,
    ) -> Option<dir::GlobalSymbolId> {
        let dir_tree = self.view();
        let expression = dir_tree.get::<dir::Expression>(expression_id);

        // prefer the current member expression when the cursor is on its receiver
        if let dir::Expression::Member { .. } = expression
            && let Some(name_span) = self.member_access_name_span(expression_id)
            && offset < name_span.start
        {
            return self.member_access_symbol_target(expression_id);
        }

        // otherwise lift the current expression into its enclosing member receiver slot
        let parent = dir_tree.get_parent_for(expression_id)?;
        if parent.ty != dir::NodeType::Expression {
            return None;
        }

        let parent_expression_id = parent.try_into().ok()?;
        let parent_expression = dir_tree.get::<dir::Expression>(parent_expression_id);
        let dir::Expression::Member { left, .. } = parent_expression else {
            return None;
        };
        if *left != expression_id {
            return None;
        }

        let name_span = self.member_access_name_span(parent_expression_id)?;
        if offset >= name_span.start {
            return None;
        }

        self.member_access_symbol_target(parent_expression_id)
    }
}

impl ModuleQueryContext<'_> {
    /// Resolve a symbol from structural DIR mappings.
    pub(crate) fn find_symbol_at_offset(&self, offset: u32) -> Option<SymbolAtOffset> {
        let ctx = self;
        let dir = ctx.dir();

        // find source nodes at the offset
        let enclosing = dir
            .tree()
            .source_index
            .get_enclosing_spans(dir.file_id(), offset, offset);
        if enclosing.is_empty() {
            return None;
        }

        // sort by length: smallest first
        let mut enclosing = enclosing;
        enclosing.sort_by_key(|span| (span.length, -(span.source_id as i64)));

        // skip doc and comment tokens before resolving symbols
        let mut is_comment_token = |token: &dir::TokenSpan| {
            if token.span.file != dir.file_id() {
                return false;
            }
            matches!(
                token.token.ty(),
                dir::TokenType::DocLineComment
                    | dir::TokenType::DocBlockComment
                    | dir::TokenType::LineComment
                    | dir::TokenType::BlockComment
            ) && token.span.contains(offset)
        };

        if dir.tokens().iter().any(&mut is_comment_token)
            || dir.side_tokens().iter().any(&mut is_comment_token)
        {
            return None;
        }

        let dir_tree = dir.view();

        // check generic parameters first to avoid capturing the enclosing declaration
        if let Some(result) = Self::generic_parameter_symbol_at_offset(ctx, offset) {
            return Some(result);
        }

        // scan member access expressions first to lock onto the member name span
        for (expr_id, expr) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
            let dir::Expression::Member { left, name, .. } = expr else {
                continue;
            };
            let Some(name) = name else {
                continue;
            };

            if let Some(result) =
                Self::member_symbol_at_offset(ctx, expr_id, expr_id.into(), *left, *name, offset)
            {
                return Some(result);
            }
        }

        // try each source node from smallest to largest
        for enclosing_span in &enclosing {
            let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enclosing_span.source_id)
            else {
                continue;
            };

            match dir_node_id.ty {
                dir::NodeType::Expression => {
                    let Ok(expr_id) = dir_node_id.try_into() else {
                        continue;
                    };
                    let expr = dir_tree.get::<dir::Expression>(expr_id);
                    let mut skip_expression_target_symbol = false;

                    if let dir::Expression::Member { left, name, .. } = expr {
                        let Some(name) = name else {
                            continue;
                        };
                        let member_name = dir.strings().get(*name);
                        let is_member_name_token = ctx
                            .token_at_offset(offset)
                            .as_deref()
                            .is_some_and(|token| member_name == token);

                        if !is_member_name_token {
                            skip_expression_target_symbol = true;
                        }

                        let result = Self::member_symbol_at_offset(
                            ctx,
                            expr_id,
                            dir_node_id,
                            *left,
                            *name,
                            offset,
                        );
                        if let Some(result) = result {
                            return Some(result);
                        }

                        // use the recorded member target when the cursor is on the member name
                        if is_member_name_token
                            && let Some(target_symbol) = dir.member_access_symbol_target(expr_id)
                        {
                            let span = dir
                                .member_access_name_span(expr_id)
                                .or_else(|| {
                                    dir.token_span_at_offset(offset).map(|token| token.span)
                                })
                                .unwrap_or_else(|| {
                                    Span::new(
                                        dir.file_id(),
                                        enclosing_span.span.start,
                                        enclosing_span.span.end,
                                    )
                                });

                            return Some(SymbolAtOffset {
                                symbol_id: target_symbol,
                                node_id: dir_node_id,
                                span,
                            });
                        }

                        // treat offsets before the member name as receiver positions
                        if let Some(name_span) = dir.member_access_name_span(expr_id)
                            && offset < name_span.start
                        {
                            skip_expression_target_symbol = true;

                            if let Some(receiver_symbol) =
                                dir.namespace_receiver_symbol_target(*left)
                            {
                                let member_span =
                                    dir.get_node_tree_main_span(dir.view(), dir_node_id);
                                let receiver_end = name_span.start.saturating_sub(1);
                                let receiver_span = (member_span.start <= receiver_end).then_some(
                                    Span::new(member_span.file, member_span.start, receiver_end),
                                );

                                if let Some(receiver_span) = receiver_span
                                    && Self::offset_matches_symbol_span(offset, receiver_span)
                                {
                                    return Some(SymbolAtOffset {
                                        symbol_id: receiver_symbol,
                                        node_id: (*left).into(),
                                        span: receiver_span,
                                    });
                                }
                            }
                        }

                        // check if the cursor is on the left expression and resolve that symbol
                        let left_span = dir.get_node_tree_span(dir.view(), (*left).into());
                        if offset >= left_span.start
                            && offset <= left_span.end
                            && let Some(target_symbol) = dir.namespace_receiver_symbol_target(*left)
                        {
                            return Some(SymbolAtOffset {
                                symbol_id: target_symbol,
                                node_id: (*left).into(),
                                span: left_span,
                            });
                        }
                    }

                    if let Some(result) = Self::path_segment_symbol_at_offset(dir, expr_id, offset)
                    {
                        return Some(result);
                    }

                    if !skip_expression_target_symbol
                        && let Some(target_symbol) = dir.expression_symbol_target(expr_id)
                    {
                        let span = dir.get_node_tree_main_span(dir.view(), dir_node_id);
                        if !Self::offset_matches_symbol_span(offset, span) {
                            continue;
                        }

                        return Some(SymbolAtOffset {
                            symbol_id: target_symbol,
                            node_id: dir_node_id,
                            span,
                        });
                    }
                }

                dir::NodeType::Pattern => {
                    let Ok(pattern_id): Result<dir::LocalNodeId<dir::Pattern>, _> =
                        dir_node_id.try_into()
                    else {
                        continue;
                    };
                    if let Some(symbol_id) = dir.global_symbol_for_node(pattern_id.into()) {
                        let span = dir.get_node_tree_main_span(dir.view(), dir_node_id);
                        if !Self::offset_matches_symbol_span(offset, span) {
                            continue;
                        }

                        return Some(SymbolAtOffset {
                            symbol_id,
                            node_id: dir_node_id,
                            span,
                        });
                    }
                }

                dir::NodeType::PatternField => {
                    let Ok(field_id): Result<dir::LocalNodeId<dir::PatternField>, _> =
                        dir_node_id.try_into()
                    else {
                        continue;
                    };
                    if let Some(symbol_id) = dir.global_symbol_for_node(field_id.into()) {
                        let span = dir.get_node_tree_main_span(dir.view(), dir_node_id);

                        return Some(SymbolAtOffset {
                            symbol_id,
                            node_id: dir_node_id,
                            span,
                        });
                    }

                    if let Some(symbol_at) =
                        Self::pattern_field_symbol_at_offset(dir, dir_tree, field_id, offset)
                    {
                        return Some(symbol_at);
                    }
                }

                dir::NodeType::Declaration => {
                    let Ok(declaration_id): Result<dir::LocalNodeId<dir::Declaration>, _> =
                        dir_node_id.try_into()
                    else {
                        continue;
                    };
                    let Some(symbol_id) = dir.global_symbol_for_node(declaration_id.into()) else {
                        continue;
                    };
                    let span = dir.get_node_tree_main_span(dir.view(), dir_node_id);
                    if !Self::offset_matches_symbol_span(offset, span) {
                        continue;
                    }

                    return Some(SymbolAtOffset {
                        symbol_id,
                        node_id: dir_node_id,
                        span,
                    });
                }

                dir::NodeType::Member => {
                    let Ok(member_id): Result<dir::LocalNodeId<dir::Member>, _> =
                        dir_node_id.try_into()
                    else {
                        continue;
                    };
                    let Some(symbol_id) = dir.global_symbol_for_node(member_id.into()) else {
                        continue;
                    };
                    let span = dir.get_node_tree_main_span(dir.view(), dir_node_id);
                    if !Self::offset_matches_symbol_span(offset, span) {
                        continue;
                    }

                    return Some(SymbolAtOffset {
                        symbol_id,
                        node_id: dir_node_id,
                        span,
                    });
                }

                dir::NodeType::EnumField => {
                    continue;
                }

                dir::NodeType::Parameter => {
                    let Ok(param_id): Result<dir::LocalNodeId<dir::Parameter>, _> =
                        dir_node_id.try_into()
                    else {
                        continue;
                    };
                    let Some(symbol_id) = dir.global_symbol_for_node(param_id.into()) else {
                        continue;
                    };
                    let span = dir.get_node_tree_main_span(dir.view(), dir_node_id);
                    if !Self::offset_matches_symbol_span(offset, span) {
                        continue;
                    }

                    return Some(SymbolAtOffset {
                        symbol_id,
                        node_id: dir_node_id,
                        span,
                    });
                }

                dir::NodeType::DependencyItem => {
                    let Ok(item_id): Result<dir::LocalNodeId<dir::DependencyItem>, _> =
                        dir_node_id.try_into()
                    else {
                        continue;
                    };
                    let source_node_id = dir_tree.get_source(item_id);

                    // prefer alias spans as local binding targets in aliased imports
                    if let Some(alias_side_span) = dir
                        .tree()
                        .get_side_span_by_id(source_node_id, NodeSpanType::Main)
                        .map(|span| Span::new(dir.file_id(), span.start, span.end))
                        && Self::offset_matches_symbol_span(offset, alias_side_span)
                        && let Some(symbol_id) = dir.global_symbol_for_node(item_id.into())
                    {
                        return Some(SymbolAtOffset {
                            symbol_id,
                            node_id: dir_node_id,
                            span: alias_side_span,
                        });
                    }

                    // prefer imported name spans as target symbol references in aliased imports
                    if let Some(name_side_span) = dir
                        .tree()
                        .get_side_span_by_id(
                            source_node_id,
                            NodeSpanType::Region(NodeSpanRegion::Type),
                        )
                        .map(|span| Span::new(dir.file_id(), span.start, span.end))
                        && Self::offset_matches_symbol_span(offset, name_side_span)
                        && let Ok(item_id) = dir_node_id.try_into_typed::<dir::DependencyItem>()
                        && let Some(symbol_id) = dir.dependency_symbol_target(item_id)
                    {
                        return Some(SymbolAtOffset {
                            symbol_id,
                            node_id: dir_node_id,
                            span: name_side_span,
                        });
                    }

                    let Some(symbol_id) = dir.global_symbol_for_node(item_id.into()) else {
                        continue;
                    };

                    let span = dir.get_node_tree_main_span(dir.view(), dir_node_id);
                    if !Self::offset_matches_symbol_span(offset, span) {
                        continue;
                    }

                    return Some(SymbolAtOffset {
                        symbol_id,
                        node_id: dir_node_id,
                        span,
                    });
                }
                _ => {}
            }
        }

        // scan type position expressions directly when source to dir mapping is absent
        if let Some(result) = Self::type_expression_symbol_at_offset(dir, offset) {
            return Some(result);
        }

        None
    }

    /// Return one plain path segment symbol when the cursor is on that segment.
    fn path_segment_symbol_at_offset(
        ctx: DirQueryContext<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        offset: u32,
    ) -> Option<SymbolAtOffset> {
        let segment_count =
            Self::path_segment_count(ctx.view().get::<dir::Expression>(expression_id))?;

        for segment_index in 0..segment_count {
            let segment_index =
                u16::try_from(segment_index).expect("path segment offset index overflow");

            let span = ctx.path_segment_span(expression_id, segment_index)?;
            if !Self::offset_matches_symbol_span(offset, span) {
                continue;
            }

            let symbol_id = ctx.path_segment_symbol_target(expression_id, segment_index)?;
            return Some(SymbolAtOffset {
                symbol_id,
                node_id: expression_id.into(),
                span,
            });
        }

        None
    }

    /// Resolve one symbol from a type-position expression that covers the cursor.
    fn type_expression_symbol_at_offset(
        ctx: DirQueryContext<'_>,
        offset: u32,
    ) -> Option<SymbolAtOffset> {
        let dir_tree = ctx.view();
        let token_span = ctx.token_span_at_offset(offset).map(|token| token.span);

        for (expression_id, _expression) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
            if !ctx.expression_is_type_position(expression_id) {
                continue;
            }

            let expression_span = ctx.get_node_tree_span(ctx.view(), expression_id.into());
            if !expression_span.contains(offset) {
                continue;
            }

            let expression = dir_tree.get::<dir::Expression>(expression_id);
            let symbol_id = match expression {
                dir::Expression::Member { .. } => ctx.member_access_symbol_target(expression_id),
                _ => ctx.expression_symbol_target(expression_id),
            }?;

            let span = match expression {
                dir::Expression::Member { .. } => ctx
                    .member_access_name_span(expression_id)
                    .or(token_span)
                    .unwrap_or(expression_span),
                _ => token_span.unwrap_or(expression_span),
            };

            return Some(SymbolAtOffset {
                symbol_id,
                node_id: expression_id.into(),
                span,
            });
        }

        None
    }

    /// Return the number of segments in one plain path expression.
    fn path_segment_count(expression: &dir::Expression) -> Option<usize> {
        let path = match expression {
            dir::Expression::QualifiedReference { path, .. } => path,
            _ => return None,
        };

        (path.segments.len() > 1).then_some(path.segments.len())
    }

    /// Resolve a declaration symbol from a declaration modifier keyword.
    fn declaration_modifier_symbol_at_offset(&self, offset: u32) -> Option<SymbolAtOffset> {
        let ctx = self;
        let token = ctx.dir().token_span_at_offset(offset)?;
        if token.token.ty() != dir::TokenType::Identifier {
            return None;
        }

        let source_file = ctx
            .repository()
            .file(ctx.revision(), ctx.file_id())
            .ok()
            .flatten()?;
        let token_text = source_file.span_str(token.span);
        let Ok(keyword) = dir::Keyword::from_str(token_text) else {
            return None;
        };
        if !Self::is_declaration_target_modifier_keyword(keyword) {
            return None;
        }

        let dir_tree = ctx.dir().view();

        let mut enclosing =
            ctx.dir()
                .tree()
                .source_index
                .get_enclosing_spans(ctx.file_id(), offset, offset);
        enclosing.sort_by_key(|span| span.length);

        for enclosing_span in enclosing {
            let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enclosing_span.source_id)
            else {
                continue;
            };

            if dir_node_id.ty != dir::NodeType::Declaration {
                continue;
            }

            let Some(name_span) = ctx
                .dir()
                .tree()
                .source_index
                .get_main(enclosing_span.source_id)
            else {
                continue;
            };
            if token.span.start >= name_span.start {
                continue;
            }

            let Ok(declaration_id): Result<dir::LocalNodeId<dir::Declaration>, _> =
                dir_node_id.try_into()
            else {
                continue;
            };
            let Some(symbol_id) = ctx.dir().global_symbol_for_node(declaration_id.into()) else {
                continue;
            };

            return Some(SymbolAtOffset {
                symbol_id,
                node_id: dir_node_id,
                span: token.span,
            });
        }

        None
    }

    /// Check whether a modifier keyword targets the enclosing declaration symbol.
    fn is_declaration_target_modifier_keyword(keyword: dir::Keyword) -> bool {
        matches!(
            keyword,
            dir::Keyword::Export
                | dir::Keyword::Declare
                | dir::Keyword::Abstract
                | dir::Keyword::Async
                | dir::Keyword::Readonly
                | dir::Keyword::Static
        )
    }

    /// Resolve a generic parameter symbol at the given offset.
    fn generic_parameter_symbol_at_offset(&self, offset: u32) -> Option<SymbolAtOffset> {
        let ctx = self;
        let dir = ctx.dir();
        let token_name = ctx.token_at_offset(offset);

        let dir_tree = dir.view();
        for (_decl_id, declaration) in dir_tree.iter_nodes_of_type::<dir::Declaration>() {
            let Some(parameters) = declaration.generic_parameters() else {
                continue;
            };

            for &parameter_id in parameters {
                let source_node_id = dir_tree.get_source(parameter_id);
                let main_span = dir
                    .tree()
                    .get_main_span_by_id(source_node_id)
                    .unwrap_or_else(|| {
                        dir.tree()
                            .source_index
                            .get_main_or_enclosing(source_node_id)
                    });
                let parameter = dir_tree.get::<dir::GenericParameter>(parameter_id);
                let span = Span::new(ctx.file_id(), main_span.start, main_span.end);
                if !Self::offset_matches_symbol_span(offset, span) {
                    continue;
                }

                if let Some(token_name) = token_name.as_deref()
                    && let Some(parameter_name) =
                        Self::generic_parameter_name(dir.strings(), parameter)
                    && parameter_name != token_name
                {
                    continue;
                }

                let Some(symbol_id) = dir.global_symbol_for_node(parameter_id.into()) else {
                    continue;
                };
                return Some(SymbolAtOffset {
                    symbol_id,
                    node_id: parameter_id.into(),
                    span,
                });
            }
        }

        None
    }

    /// Resolve the declared name for a generic parameter when available.
    fn generic_parameter_name(
        strings: &StringPool,
        parameter: &dir::GenericParameter,
    ) -> Option<String> {
        match parameter {
            dir::GenericParameter::Type { name, .. }
            | dir::GenericParameter::VariadicType { name, .. }
            | dir::GenericParameter::Value { name, .. }
            | dir::GenericParameter::VariadicValue { name, .. } => {
                Some(strings.get(*name).to_string())
            }
            dir::GenericParameter::Error => None,
        }
    }

    /// Resolve a binding symbol inside a pattern field at the cursor.
    fn pattern_field_symbol_at_offset(
        ctx: DirQueryContext<'_>,
        dir_tree: dir::View<'_>,
        field_id: dir::LocalNodeId<dir::PatternField>,
        offset: u32,
    ) -> Option<SymbolAtOffset> {
        let field = dir_tree.get::<dir::PatternField>(field_id);

        match field {
            dir::PatternField::Named { pattern, .. } => {
                let node_id = field_id.into();
                let span = ctx.get_node_tree_main_span(ctx.view(), node_id);
                if Self::offset_matches_symbol_span(offset, span)
                    && let Some(symbol_id) = ctx.global_symbol_for_node(node_id)
                {
                    return Some(SymbolAtOffset {
                        symbol_id,
                        node_id,
                        span,
                    });
                }

                let pattern = *pattern.as_ref()?;
                Self::pattern_symbol_at_offset(ctx, dir_tree, pattern, offset)
            }
            dir::PatternField::Computed { pattern, .. } => {
                Self::pattern_symbol_at_offset(ctx, dir_tree, *pattern, offset)
            }
            dir::PatternField::Spread { pattern, .. } => {
                let pattern = *pattern.as_ref()?;
                Self::pattern_symbol_at_offset(ctx, dir_tree, pattern, offset)
            }
            dir::PatternField::Positional { pattern, .. } => {
                Self::pattern_symbol_at_offset(ctx, dir_tree, *pattern, offset)
            }
            dir::PatternField::Elision => None,
        }
    }

    /// Resolve a binding symbol inside a pattern at the cursor.
    fn pattern_symbol_at_offset(
        ctx: DirQueryContext<'_>,
        dir_tree: dir::View<'_>,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
        offset: u32,
    ) -> Option<SymbolAtOffset> {
        let pattern = dir_tree.get::<dir::Pattern>(pattern_id);

        match pattern {
            dir::Pattern::Assign { pattern, .. } => {
                Self::pattern_symbol_at_offset(ctx, dir_tree, *pattern, offset)
            }
            dir::Pattern::Binding { pattern, .. } => {
                let node_id = pattern_id.into();
                let span = ctx.get_node_tree_main_span(ctx.view(), node_id);
                if Self::offset_matches_symbol_span(offset, span)
                    && let Some(symbol_id) = ctx.global_symbol_for_node(node_id)
                {
                    return Some(SymbolAtOffset {
                        symbol_id,
                        node_id,
                        span,
                    });
                }

                if let Some(inner_pattern) = pattern {
                    return Self::pattern_symbol_at_offset(ctx, dir_tree, *inner_pattern, offset);
                }

                None
            }
            dir::Pattern::Must(inner)
            | dir::Pattern::BorrowOf { right: inner, .. }
            | dir::Pattern::MoveOf { right: inner, .. }
            | dir::Pattern::DereferenceOf { right: inner } => {
                Self::pattern_symbol_at_offset(ctx, dir_tree, *inner, offset)
            }
            dir::Pattern::TypeExpression { .. } => None,
            dir::Pattern::Tuple { fields }
            | dir::Pattern::Newtype { fields, .. }
            | dir::Pattern::Sequence { fields }
            | dir::Pattern::Object { fields }
            | dir::Pattern::NominalObject { fields, .. } => {
                for field_id in fields {
                    if let Some(symbol_at) =
                        Self::pattern_field_symbol_at_offset(ctx, dir_tree, *field_id, offset)
                    {
                        return Some(symbol_at);
                    }
                }

                None
            }
            dir::Pattern::Union { patterns } => {
                for pattern_id in patterns {
                    if let Some(symbol_at) =
                        Self::pattern_symbol_at_offset(ctx, dir_tree, *pattern_id, offset)
                    {
                        return Some(symbol_at);
                    }
                }

                None
            }
            dir::Pattern::Wildcard
            | dir::Pattern::Expression { .. }
            | dir::Pattern::Range { .. } => None,
        }
    }

    /// Check whether a cursor offset should resolve to a symbol span.
    fn offset_matches_symbol_span(offset: u32, span: Span) -> bool {
        if offset >= span.start && offset <= span.end {
            return true;
        }

        offset.saturating_add(1) >= span.start && offset < span.end
    }

    /// Return a member access symbol when the cursor is on the member name.
    fn member_symbol_at_offset(
        &self,
        expr_id: dir::LocalNodeId<dir::Expression>,
        node_id: dir::LocalNodeIdAny,
        _left: dir::LocalNodeId<dir::Expression>,
        name: StringId,
        offset: u32,
    ) -> Option<SymbolAtOffset> {
        let ctx = self;
        let dir = ctx.dir();
        let member_name = dir.strings().get(name);
        let source_file = ctx
            .repository()
            .file(ctx.revision(), ctx.file_id())
            .ok()
            .flatten()?;
        let token_span = dir.token_span_at_offset(offset).map(|token| token.span);

        if let Some(token_span) = token_span {
            let token_name = source_file.span_str(token_span);
            if member_name == token_name {
                let symbol_id = dir.member_access_symbol_target(expr_id)?;

                return Some(SymbolAtOffset {
                    symbol_id,
                    node_id,
                    span: token_span,
                });
            }
        }

        let name_span = dir.member_access_name_span(expr_id)?;
        if offset < name_span.start || offset > name_span.end {
            return None;
        }

        let token_name = source_file.span_str(name_span);
        if member_name != token_name {
            return None;
        }

        let symbol_id = dir.member_access_symbol_target(expr_id)?;
        Some(SymbolAtOffset {
            symbol_id,
            node_id,
            span: name_span,
        })
    }
}
