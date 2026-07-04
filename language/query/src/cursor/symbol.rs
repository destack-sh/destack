use std::str::FromStr;

use destack_core::{StringId, StringPool};
use destack_dir as dir;
use destack_source::{EnclosingSpan, NodeSpanRegion, NodeSpanType, Span};

use crate::ModuleQueryContext;

/// Symbol found under a cursor.
#[derive(Debug, Clone)]
pub(crate) struct SymbolHit {
    /// The symbol that was referenced.
    pub symbol_id: dir::GlobalSymbolId,
    /// The DIR node that contains the reference.
    pub node_id: dir::LocalNodeIdAny,
    /// The span of the reference.
    pub span: Span,
}

impl SymbolHit {
    /// Create one symbol cursor hit.
    fn new(symbol_id: dir::GlobalSymbolId, node_id: dir::LocalNodeIdAny, span: Span) -> Self {
        Self {
            symbol_id,
            node_id,
            span,
        }
    }
}

impl ModuleQueryContext<'_> {
    /// Resolve the semantic target symbol used by semantic navigation queries.
    pub(crate) fn semantic_target_symbol_at_offset(
        &self,
        offset: u32,
        symbol_at: &SymbolHit,
    ) -> dir::GlobalSymbolId {
        // keep non-expression symbols unchanged
        if symbol_at.node_id.ty != dir::NodeType::Expression {
            return symbol_at.symbol_id;
        }

        // read the expression id behind the symbol hit
        let expression_id = symbol_at.node_id.try_into().unwrap_or_else(|_| {
            panic!(
                "source symbol node has expression type but invalid id {}",
                symbol_at.node_id.id
            )
        });

        // use the enclosing member target when the cursor is on a namespace receiver
        let Some(target_symbol) = self.member_access_target_symbol_at_offset(expression_id, offset)
        else {
            return symbol_at.symbol_id;
        };

        // ignore equivalent targets
        if target_symbol != symbol_at.symbol_id {
            return target_symbol;
        }

        symbol_at.symbol_id
    }

    /// Resolve the local binding symbol used by declaration style queries.
    pub(crate) fn binding_symbol_at_offset(
        &self,
        offset: u32,
        symbol_at: &SymbolHit,
    ) -> dir::GlobalSymbolId {
        // keep non-expression symbols unchanged
        if symbol_at.node_id.ty != dir::NodeType::Expression {
            return symbol_at.symbol_id;
        }

        // read the expression id behind the symbol hit
        let expression_id = symbol_at.node_id.try_into().unwrap_or_else(|_| {
            panic!(
                "source symbol node has expression type but invalid id {}",
                symbol_at.node_id.id
            )
        });

        // preserve plain path segments as their local binding symbols
        if let Some(result) = self.path_segment_symbol_at_offset(expression_id, offset) {
            return result.symbol_id;
        }

        // preserve namespace member receivers as their local binding symbols
        if let Some(receiver_symbol) =
            self.namespace_receiver_binding_symbol_at_offset(expression_id, offset, symbol_at)
        {
            return receiver_symbol;
        }

        // otherwise use the expression target when one exists
        if let Some(expression_symbol) = self.expression_symbol_target(expression_id) {
            expression_symbol
        } else {
            symbol_at.symbol_id
        }
    }

    /// Resolve the hover symbol at a given offset.
    pub(crate) fn hover_symbol_at_offset(&self, offset: u32) -> Option<SymbolHit> {
        self.find_symbol_at_offset(offset)
            .or_else(|| ModuleQueryContext::declaration_modifier_symbol_at_offset(self, offset))
    }

    /// Resolve the namespace receiver binding symbol at one expression offset.
    fn namespace_receiver_binding_symbol_at_offset(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        offset: u32,
        symbol_at: &SymbolHit,
    ) -> Option<dir::GlobalSymbolId> {
        // require a member expression
        let dir::Expression::Member { left, .. } =
            self.view().get::<dir::Expression>(expression_id)
        else {
            return None;
        };

        // require the cursor before the member name
        let name_span = self.member_access_name_span(expression_id)?;
        if offset >= name_span.start {
            return None;
        }

        // use the namespace receiver target when it resolves
        if let Some(receiver_symbol) = self.namespace_receiver_symbol_target(*left) {
            return Some(receiver_symbol);
        }

        Some(symbol_at.symbol_id)
    }
}

impl ModuleQueryContext<'_> {
    /// Resolve one path segment span inside a plain multi segment path expression.
    pub(crate) fn path_segment_span(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        segment_index: u16,
    ) -> Option<Span> {
        // collect the member chain nodes from leaf to root
        let mut nodes = Vec::new();
        let mut current = Some(expression_id);
        while let Some(id) = current {
            nodes.push(id);
            current = match self.view().get::<dir::Expression>(id) {
                dir::Expression::Member { left, .. } => Some(*left),
                _ => None,
            };
        }

        // put path segments in source order
        nodes.reverse();

        // the segment span is the matching chain node's own span
        let node = *nodes.get(usize::from(segment_index))?;
        let source_id = self.view().get_source(node);
        let span = self.tree().get_main_span_by_id(source_id)?;

        Some(Span::new(self.file_id(), span.start, span.end))
    }

    /// Resolve the span for a member access name inside its expression span.
    pub(crate) fn member_access_name_span(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<Span> {
        // read the expression and its source extent
        let expression = self.view().get::<dir::Expression>(expression_id);
        let expression_span = self.get_span(self.view(), expression_id.into());

        // prefer the first identifier after the receiver span
        if let dir::Expression::Member { left, .. } = expression {
            let left_span = self.get_span(self.view(), (*left).into());

            for token in self.tokens() {
                // require tokens in this file
                if token.span.file != self.file_id() {
                    continue;
                }

                // require a token after the receiver
                if token.span.start <= left_span.end {
                    continue;
                }

                // require a token inside the member expression
                if token.span.start < expression_span.start || token.span.end > expression_span.end
                {
                    continue;
                }

                // require an identifier-shaped token
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
            // require tokens in this file
            if token.span.file != self.file_id() {
                continue;
            }

            // require a token inside the member expression
            if token.span.start < expression_span.start || token.span.end > expression_span.end {
                continue;
            }

            // require an identifier-shaped token
            if !matches!(
                token.token.ty(),
                dir::TokenType::Identifier | dir::TokenType::InvalidIdentifier
            ) {
                continue;
            }

            last_identifier = Some(token.span);
        }

        // use the last identifier found in the expression
        if let Some(last_identifier) = last_identifier {
            return Some(last_identifier);
        }

        // use the source DIR main span when no identifier token is found
        let source_node_id = self.view().get_source(expression_id);
        self.tree().get_main_span_by_id(source_node_id)
    }

    /// Resolve the member target for a receiver position inside one member access.
    fn member_access_target_symbol_at_offset(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        offset: u32,
    ) -> Option<dir::GlobalSymbolId> {
        // read the current expression
        let view = self.view();
        let expression = view.get::<dir::Expression>(expression_id);

        // prefer the current member expression when the cursor is on its receiver
        if let dir::Expression::Member { .. } = expression {
            if let Some(name_span) = self.member_access_name_span(expression_id) {
                if offset < name_span.start {
                    return self.member_access_symbol_target(expression_id);
                }
            }
        }

        // resolve the parent expression
        let parent = view.get_parent_for(expression_id)?;
        if parent.ty != dir::NodeType::Expression {
            return None;
        }

        let parent_expression_id = parent
            .try_into()
            .unwrap_or_else(|_| panic!("member receiver parent is not an expression: {parent:?}"));
        let parent_expression = view.get::<dir::Expression>(parent_expression_id);
        let dir::Expression::Member { left, .. } = parent_expression else {
            return None;
        };
        if *left != expression_id {
            return None;
        }

        // require the cursor on the receiver side
        let name_span = self.member_access_name_span(parent_expression_id)?;
        if offset >= name_span.start {
            return None;
        }

        // lift the current expression into its enclosing member receiver slot
        self.member_access_symbol_target(parent_expression_id)
    }
}

impl ModuleQueryContext<'_> {
    /// Resolve a symbol from structural DIR mappings.
    pub(crate) fn find_symbol_at_offset(&self, offset: u32) -> Option<SymbolHit> {
        // collect source owners around the cursor
        let enclosing = self.symbol_enclosing_spans(offset)?;

        // ignore comments before consulting semantic mappings
        if self.comment_token_contains_offset(offset) {
            return None;
        }

        // prefer generic parameter names over enclosing declarations
        if let Some(result) = self.generic_parameter_symbol_at_offset(offset) {
            return Some(result);
        }

        // prefer member name spans before enclosing expression spans
        if let Some(result) = self.member_expression_symbol_at_offset(offset) {
            return Some(result);
        }

        // read checked DIR view once for source node scans
        let view = self.view();

        // scan source nodes from smallest to largest
        for enclosing_span in &enclosing {
            if let Some(result) = self.symbol_at_enclosing_span(view, enclosing_span, offset) {
                return Some(result);
            }
        }

        // inspect type positions after source node mappings
        if let Some(result) = self.type_expression_symbol_at_offset(offset) {
            return Some(result);
        }

        None
    }

    /// Return sorted source spans that contain the cursor.
    fn symbol_enclosing_spans(&self, offset: u32) -> Option<Vec<EnclosingSpan>> {
        // collect source spans that own the cursor
        let mut enclosing =
            self.tree()
                .source_index
                .get_enclosing_spans(self.file_id(), offset, offset);
        if enclosing.is_empty() {
            return None;
        }

        // order from innermost to outermost
        enclosing.sort_by_key(|span| (span.length, -(span.source_id as i64)));

        Some(enclosing)
    }

    /// Return whether a comment token owns the cursor.
    fn comment_token_contains_offset(&self, offset: u32) -> bool {
        // classify comments in this file at the cursor
        let is_comment_token = |token: &dir::TokenSpan| {
            if token.span.file != self.file_id() {
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

        // check ordinary and side token streams
        self.tokens().iter().any(is_comment_token)
            || self.side_tokens().iter().any(is_comment_token)
    }

    /// Resolve one member expression symbol from its name span.
    fn member_expression_symbol_at_offset(&self, offset: u32) -> Option<SymbolHit> {
        // scan checked expressions for member accesses
        let view = self.view();

        for (expression_id, expression) in view.iter_nodes_of_type::<dir::Expression>() {
            // require member expressions with an explicit name
            let dir::Expression::Member { name, .. } = expression else {
                continue;
            };
            let Some(name) = name else {
                continue;
            };

            // use the member-name hit when the cursor is on it
            if let Some(result) =
                self.member_symbol_at_offset(expression_id, expression_id.into(), *name, offset)
            {
                return Some(result);
            }
        }

        None
    }

    /// Resolve one symbol from an enclosing source span.
    fn symbol_at_enclosing_span(
        &self,
        view: dir::View<'_>,
        enclosing_span: &EnclosingSpan,
        offset: u32,
    ) -> Option<SymbolHit> {
        // map the source span to a checked DIR node
        let node_id = view.get_node_id_by_source_id(enclosing_span.source_id)?;

        // dispatch by checked DIR node family
        match node_id.ty {
            dir::NodeType::Expression => {
                let expression_id: dir::LocalNodeId<dir::Expression> = node_id.try_into().ok()?;
                self.expression_symbol_at_node(view, expression_id, node_id, offset)
            }
            dir::NodeType::Pattern => {
                let pattern_id: dir::LocalNodeId<dir::Pattern> = node_id.try_into().ok()?;
                self.node_symbol_at_main_span(pattern_id.into(), node_id, offset)
            }
            dir::NodeType::PatternField => {
                let field_id: dir::LocalNodeId<dir::PatternField> = node_id.try_into().ok()?;
                self.pattern_field_symbol_at_node(view, field_id, node_id, offset)
            }
            dir::NodeType::Declaration => {
                let declaration_id: dir::LocalNodeId<dir::Declaration> = node_id.try_into().ok()?;
                self.node_symbol_at_main_span(declaration_id.into(), node_id, offset)
            }
            dir::NodeType::Member => {
                let member_id: dir::LocalNodeId<dir::Member> = node_id.try_into().ok()?;
                self.node_symbol_at_main_span(member_id.into(), node_id, offset)
            }
            dir::NodeType::Parameter => {
                let parameter_id: dir::LocalNodeId<dir::Parameter> = node_id.try_into().ok()?;
                self.node_symbol_at_main_span(parameter_id.into(), node_id, offset)
            }
            dir::NodeType::DependencyItem => {
                let item_id: dir::LocalNodeId<dir::DependencyItem> = node_id.try_into().ok()?;
                self.dependency_item_symbol_at_node(view, item_id, node_id, offset)
            }
            dir::NodeType::EnumField => None,
            _ => None,
        }
    }

    /// Resolve a symbol from one expression node at the cursor.
    fn expression_symbol_at_node(
        &self,
        view: dir::View<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        node_id: dir::LocalNodeIdAny,
        offset: u32,
    ) -> Option<SymbolHit> {
        // read the expression node
        let expression = view.get::<dir::Expression>(expression_id);
        let mut reads_expression_target = true;

        // resolve member names and namespace receivers before generic expression targets
        if let dir::Expression::Member { left, name, .. } = expression {
            let name = *name.as_ref()?;
            let is_member_name_token = self.member_name_token_matches(name, offset);
            reads_expression_target = is_member_name_token;

            // prefer a direct member-name hit
            if let Some(result) = self.member_symbol_at_offset(expression_id, node_id, name, offset)
            {
                return Some(result);
            }

            // use checked member resolution on member names
            if let Some(result) =
                self.member_name_target_symbol_at_offset(expression_id, node_id, offset)
            {
                return Some(result);
            }

            // use namespace receiver resolution on receiver spans
            if let Some(result) =
                self.member_receiver_symbol_at_offset(expression_id, *left, offset)
            {
                return Some(result);
            }
        }

        // resolve individual path segments in plain multi segment paths
        if let Some(result) = self.path_segment_symbol_at_offset(expression_id, offset) {
            return Some(result);
        }

        // use the expression target when no more specific member or path hit exists
        if reads_expression_target {
            return self.expression_target_symbol_at_offset(expression_id, node_id, offset);
        }

        None
    }

    /// Return whether the current token text matches one member name.
    fn member_name_token_matches(&self, name: StringId, offset: u32) -> bool {
        // resolve the member name text
        let member_name = self.strings().get(name);

        // compare it against the token under the cursor
        self.token_at_offset(offset)
            .as_deref()
            .is_some_and(|token| member_name == token)
    }

    /// Resolve the checked member target when the cursor is on the member name.
    fn member_name_target_symbol_at_offset(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        node_id: dir::LocalNodeIdAny,
        offset: u32,
    ) -> Option<SymbolHit> {
        // require the cursor on the member name span
        let name_span = self.member_access_name_span(expression_id)?;
        if !Self::offset_matches_symbol_span(offset, name_span) {
            return None;
        }

        // resolve the checked member target
        let symbol_id = self.member_access_symbol_target(expression_id)?;

        Some(SymbolHit::new(symbol_id, node_id, name_span))
    }

    /// Resolve a namespace receiver symbol inside one member expression.
    fn member_receiver_symbol_at_offset(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        offset: u32,
    ) -> Option<SymbolHit> {
        // read the member name span
        let name_span = self.member_access_name_span(expression_id)?;

        // use the sliced receiver span before the member name
        if offset < name_span.start {
            if let Some(symbol_at) =
                self.namespace_receiver_symbol_before_name(expression_id, left, name_span, offset)
            {
                return Some(symbol_at);
            }
        }

        // use the full receiver expression span when the cursor is inside it
        let left_span = self.get_span(self.view(), left.into());
        if !left_span.contains(offset) {
            return None;
        }

        // resolve the checked namespace receiver target
        let symbol_id = self.namespace_receiver_symbol_target(left)?;

        Some(SymbolHit::new(symbol_id, left.into(), left_span))
    }

    /// Resolve a namespace receiver symbol from the span before a member name.
    fn namespace_receiver_symbol_before_name(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        name_span: Span,
        offset: u32,
    ) -> Option<SymbolHit> {
        // resolve the checked namespace receiver target
        let symbol_id = self.namespace_receiver_symbol_target(left)?;

        // build the receiver span before the member name
        let member_span = self.get_main_span(self.view(), expression_id.into());
        let receiver_end = name_span.start.saturating_sub(1);
        if member_span.start > receiver_end {
            return None;
        }

        let receiver_span = Span::new(member_span.file, member_span.start, receiver_end);

        // require the cursor on the receiver span
        if !Self::offset_matches_symbol_span(offset, receiver_span) {
            return None;
        }

        Some(SymbolHit::new(symbol_id, left.into(), receiver_span))
    }

    /// Resolve a generic expression target at the cursor.
    fn expression_target_symbol_at_offset(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        node_id: dir::LocalNodeIdAny,
        offset: u32,
    ) -> Option<SymbolHit> {
        // resolve the checked expression target
        let symbol_id = self.expression_symbol_target(expression_id)?;

        // require the cursor on the expression main span
        let span = self.get_main_span(self.view(), node_id);
        if !Self::offset_matches_symbol_span(offset, span) {
            return None;
        }

        Some(SymbolHit::new(symbol_id, node_id, span))
    }

    /// Resolve a node symbol when the cursor is on the node main span.
    fn node_symbol_at_main_span(
        &self,
        symbol_node_id: dir::LocalNodeIdAny,
        source_node_id: dir::LocalNodeIdAny,
        offset: u32,
    ) -> Option<SymbolHit> {
        // resolve the checked symbol attached to the node
        let symbol_id = self.global_node_symbol(symbol_node_id)?;

        // require the cursor on the source node main span
        let span = self.get_main_span(self.view(), source_node_id);
        if !Self::offset_matches_symbol_span(offset, span) {
            return None;
        }

        Some(SymbolHit::new(symbol_id, source_node_id, span))
    }

    /// Resolve an import or export item symbol at the cursor.
    fn dependency_item_symbol_at_node(
        &self,
        view: dir::View<'_>,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
        node_id: dir::LocalNodeIdAny,
        offset: u32,
    ) -> Option<SymbolHit> {
        // resolve the source node attached to the dependency item
        let source_node_id = view.get_source(item_id);

        // prefer alias spans as local binding targets in aliased imports
        if let Some(alias_span) = self.dependency_item_side_span(source_node_id, NodeSpanType::Main)
        {
            if Self::offset_matches_symbol_span(offset, alias_span) {
                if let Some(symbol_id) = self.global_node_symbol(item_id.into()) {
                    return Some(SymbolHit::new(symbol_id, node_id, alias_span));
                }
            }
        }

        // prefer imported name spans as target symbol references in aliased imports
        let name_region = NodeSpanType::Region(NodeSpanRegion::Type);
        if let Some(name_span) = self.dependency_item_side_span(source_node_id, name_region) {
            if Self::offset_matches_symbol_span(offset, name_span) {
                if let Some(symbol_id) = self.dependency_symbol_target(item_id) {
                    return Some(SymbolHit::new(symbol_id, node_id, name_span));
                }
            }
        }

        // otherwise use the dependency item local symbol
        self.node_symbol_at_main_span(item_id.into(), node_id, offset)
    }

    /// Return one dependency item side span in this module.
    fn dependency_item_side_span(
        &self,
        source_node_id: u32,
        span_type: NodeSpanType,
    ) -> Option<Span> {
        // read the side span and remap it to this file
        self.tree()
            .get_side_span_by_id(source_node_id, span_type)
            .map(|span| Span::new(self.file_id(), span.start, span.end))
    }

    /// Resolve one pattern field symbol from a field node.
    fn pattern_field_symbol_at_node(
        &self,
        view: dir::View<'_>,
        field_id: dir::LocalNodeId<dir::PatternField>,
        node_id: dir::LocalNodeIdAny,
        offset: u32,
    ) -> Option<SymbolHit> {
        // use the field node symbol when one exists
        if let Some(symbol_id) = self.global_node_symbol(field_id.into()) {
            let span = self.get_main_span(self.view(), node_id);

            return Some(SymbolHit::new(symbol_id, node_id, span));
        }

        // otherwise inspect nested binding patterns
        self.pattern_field_symbol_at_offset(view, field_id, offset)
    }

    /// Return one plain path segment symbol when the cursor is on that segment.
    fn path_segment_symbol_at_offset(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        offset: u32,
    ) -> Option<SymbolHit> {
        // multi segment name paths carry one symbol per segment
        let path = self.tree().reference_path(expression_id)?;
        if path.segments.len() <= 1 {
            return None;
        }
        let segment_count = path.segments.len();

        // scan path segments in source order
        for segment_index in 0..segment_count {
            let Ok(segment_index) = u16::try_from(segment_index) else {
                break;
            };

            // require the cursor on this segment span
            let span = self.path_segment_span(expression_id, segment_index)?;
            if !Self::offset_matches_symbol_span(offset, span) {
                continue;
            }

            // resolve the checked segment target
            let symbol_id = self.path_segment_symbol_target(expression_id, segment_index)?;

            return Some(SymbolHit::new(symbol_id, expression_id.into(), span));
        }

        None
    }

    /// Resolve one symbol from a type-position expression that covers the cursor.
    fn type_expression_symbol_at_offset(&self, offset: u32) -> Option<SymbolHit> {
        // read source token and checked expression data
        let view = self.view();
        let token_span = self.token_span_at_offset(offset).map(|token| token.span);

        // scan type-position expressions
        for (expression_id, _expression) in view.iter_nodes_of_type::<dir::Expression>() {
            if !self.expression_is_type_position(expression_id) {
                continue;
            }

            // require the cursor inside the expression span
            let expression_span = self.get_span(self.view(), expression_id.into());
            if !expression_span.contains(offset) {
                continue;
            }

            // resolve the checked type-position target
            let expression = view.get::<dir::Expression>(expression_id);
            let symbol_id = match expression {
                dir::Expression::Member { .. } => self.member_access_symbol_target(expression_id),
                _ => self.expression_symbol_target(expression_id),
            }?;

            // prefer member names, then source tokens, then the expression span
            let span = match expression {
                dir::Expression::Member { .. } => self.member_access_name_span(expression_id)?,
                _ => {
                    if let Some(token_span) = token_span {
                        token_span
                    } else {
                        expression_span
                    }
                }
            };

            return Some(SymbolHit::new(symbol_id, expression_id.into(), span));
        }

        None
    }

    /// Resolve a declaration symbol from a declaration modifier keyword.
    fn declaration_modifier_symbol_at_offset(&self, offset: u32) -> Option<SymbolHit> {
        // require an identifier token at the cursor
        let token = self.token_span_at_offset(offset)?;
        if token.token.ty() != dir::TokenType::Identifier {
            return None;
        }

        // require a modifier keyword that can target declarations
        let source_file = self.source_file();
        let token_text = source_file.span_str(token.span);
        let Ok(keyword) = dir::Keyword::from_str(token_text) else {
            return None;
        };
        if !Self::is_declaration_target_modifier_keyword(keyword) {
            return None;
        }

        // collect enclosing source spans from inner to outer
        let view = self.view();
        let mut enclosing =
            self.tree()
                .source_index
                .get_enclosing_spans(self.file_id(), offset, offset);
        enclosing.sort_by_key(|span| span.length);

        // scan enclosing declaration spans
        for enclosing_span in enclosing {
            let Some(dir_node_id) = view.get_node_id_by_source_id(enclosing_span.source_id) else {
                continue;
            };

            // require a checked declaration node
            if dir_node_id.ty != dir::NodeType::Declaration {
                continue;
            }

            // require the modifier before the declaration name
            let Some(name_span) = self.tree().source_index.get_main(enclosing_span.source_id)
            else {
                continue;
            };
            if token.span.start >= name_span.start {
                continue;
            }

            // resolve the declaration symbol
            let Ok(declaration_id): Result<dir::LocalNodeId<dir::Declaration>, _> =
                dir_node_id.try_into()
            else {
                continue;
            };
            let Some(symbol_id) = self.global_node_symbol(declaration_id.into()) else {
                continue;
            };

            return Some(SymbolHit::new(symbol_id, dir_node_id, token.span));
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
    fn generic_parameter_symbol_at_offset(&self, offset: u32) -> Option<SymbolHit> {
        // read the source token for name disambiguation
        let token_name = self.token_at_offset(offset);

        // scan declaration generic parameter lists
        let view = self.view();
        for (_decl_id, declaration) in view.iter_nodes_of_type::<dir::Declaration>() {
            let Some(parameters) = declaration.generic_parameters() else {
                continue;
            };

            // scan parameters in source order
            for &parameter_id in parameters {
                if let Some(result) =
                    self.generic_parameter_symbol(parameter_id, token_name.as_deref(), offset)
                {
                    return Some(result);
                }
            }
        }

        None
    }

    /// Resolve one generic parameter symbol at the cursor.
    fn generic_parameter_symbol(
        &self,
        parameter_id: dir::LocalNodeId<dir::GenericParameter>,
        token_name: Option<&str>,
        offset: u32,
    ) -> Option<SymbolHit> {
        // resolve the source span for the parameter name
        let source_node_id = self.view().get_source(parameter_id);
        let main_span = self
            .tree()
            .get_main_span_by_id(source_node_id)
            .unwrap_or_else(|| {
                panic!("missing main source span for generic parameter {source_node_id}")
            });
        let span = Span::new(self.file_id(), main_span.start, main_span.end);

        // require the cursor on the parameter name
        if !Self::offset_matches_symbol_span(offset, span) {
            return None;
        }

        // compare token text when the cursor owns an identifier token
        let parameter = self.view().get::<dir::GenericParameter>(parameter_id);
        if !Self::generic_parameter_token_matches(self.strings(), parameter, token_name) {
            return None;
        }

        // resolve the checked generic parameter symbol
        let symbol_id = self.global_node_symbol(parameter_id.into())?;

        Some(SymbolHit::new(symbol_id, parameter_id.into(), span))
    }

    /// Return whether one generic parameter matches the cursor token.
    fn generic_parameter_token_matches(
        strings: &StringPool,
        parameter: &dir::GenericParameter,
        token_name: Option<&str>,
    ) -> bool {
        // accept cursors that do not own an identifier token
        let Some(token_name) = token_name else {
            return true;
        };

        // compare checked parameter names against the source token
        let parameter_name = Self::generic_parameter_name(strings, parameter);

        parameter_name.is_none_or(|parameter_name| parameter_name == token_name)
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
            dir::GenericParameter::Error => {
                panic!("error generic parameter reached source symbol query")
            }
        }
    }

    /// Resolve a binding symbol inside a pattern field at the cursor.
    fn pattern_field_symbol_at_offset(
        &self,
        view: dir::View<'_>,
        field_id: dir::LocalNodeId<dir::PatternField>,
        offset: u32,
    ) -> Option<SymbolHit> {
        // read the pattern field
        let field = view.get::<dir::PatternField>(field_id);

        // dispatch by pattern field shape
        match field {
            dir::PatternField::Named { pattern, .. } => {
                // prefer the field's own binding span
                let node_id = field_id.into();
                let span = self.get_main_span(self.view(), node_id);
                if Self::offset_matches_symbol_span(offset, span) {
                    if let Some(symbol_id) = self.global_node_symbol(node_id) {
                        return Some(SymbolHit::new(symbol_id, node_id, span));
                    }
                }

                // otherwise inspect the nested binding pattern
                let pattern = *pattern.as_ref()?;
                self.pattern_symbol_at_offset(view, pattern, offset)
            }
            dir::PatternField::Computed { pattern, .. } => {
                self.pattern_symbol_at_offset(view, *pattern, offset)
            }
            dir::PatternField::Spread { pattern, .. } => {
                let pattern = *pattern.as_ref()?;
                self.pattern_symbol_at_offset(view, pattern, offset)
            }
            dir::PatternField::Positional { pattern, .. } => {
                self.pattern_symbol_at_offset(view, *pattern, offset)
            }
            dir::PatternField::Elision => None,
        }
    }

    /// Resolve a binding symbol inside a pattern at the cursor.
    fn pattern_symbol_at_offset(
        &self,
        view: dir::View<'_>,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
        offset: u32,
    ) -> Option<SymbolHit> {
        // read the pattern node
        let pattern = view.get::<dir::Pattern>(pattern_id);

        // dispatch by pattern shape
        match pattern {
            dir::Pattern::Default { pattern, .. } => {
                self.pattern_symbol_at_offset(view, *pattern, offset)
            }
            dir::Pattern::Binding { pattern, .. } => {
                // prefer the binding's own name span
                let node_id = pattern_id.into();
                let span = self.get_main_span(self.view(), node_id);
                if Self::offset_matches_symbol_span(offset, span) {
                    if let Some(symbol_id) = self.global_node_symbol(node_id) {
                        return Some(SymbolHit::new(symbol_id, node_id, span));
                    }
                }

                // otherwise inspect any nested pattern
                if let Some(inner_pattern) = pattern {
                    return self.pattern_symbol_at_offset(view, *inner_pattern, offset);
                }

                None
            }
            dir::Pattern::Must(inner)
            | dir::Pattern::BorrowOf { right: inner, .. }
            | dir::Pattern::MoveOf { right: inner, .. }
            | dir::Pattern::DereferenceOf { right: inner } => {
                self.pattern_symbol_at_offset(view, *inner, offset)
            }
            dir::Pattern::Tuple { fields }
            | dir::Pattern::NominalTuple { fields, .. }
            | dir::Pattern::Sequence { fields }
            | dir::Pattern::Object { fields }
            | dir::Pattern::NominalObject { fields, .. } => {
                // scan nested pattern fields
                for field_id in fields {
                    if let Some(symbol_at) =
                        self.pattern_field_symbol_at_offset(view, *field_id, offset)
                    {
                        return Some(symbol_at);
                    }
                }

                None
            }
            dir::Pattern::Union { patterns } => {
                // scan alternative patterns
                for pattern_id in patterns {
                    if let Some(symbol_at) =
                        self.pattern_symbol_at_offset(view, *pattern_id, offset)
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
        // accept a cursor inside or at the end of the span
        if offset >= span.start && offset <= span.end {
            return true;
        }

        // accept a cursor immediately before the span at token boundaries
        offset.saturating_add(1) >= span.start && offset < span.end
    }

    /// Return a member access symbol when the cursor is on the member name.
    fn member_symbol_at_offset(
        &self,
        expr_id: dir::LocalNodeId<dir::Expression>,
        node_id: dir::LocalNodeIdAny,
        name: StringId,
        offset: u32,
    ) -> Option<SymbolHit> {
        // read member and source token text
        let member_name = self.strings().get(name);
        let source_file = self.source_file();
        let token_span = self.token_span_at_offset(offset).map(|token| token.span);

        // prefer the token under the cursor
        if let Some(token_span) = token_span {
            let token_name = source_file.span_str(token_span);
            if member_name == token_name {
                let symbol_id = self.member_access_symbol_target(expr_id)?;

                return Some(SymbolHit::new(symbol_id, node_id, token_span));
            }
        }

        // require the cursor on the member name span
        let name_span = self.member_access_name_span(expr_id)?;
        if offset < name_span.start || offset > name_span.end {
            return None;
        }

        // require source text to match the checked member name
        let token_name = source_file.span_str(name_span);
        if member_name != token_name {
            return None;
        }

        // resolve the checked member target
        let symbol_id = self.member_access_symbol_target(expr_id)?;

        Some(SymbolHit::new(symbol_id, node_id, name_span))
    }
}
