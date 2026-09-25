use crate::lex::decode_html_entities;
use crate::parse::{ExpressionPosition, ExpressionStop, RangedPath, TokenMode};
use crate::{ParseStart, Parser, ParserError, ParserResult};
use smallvec::SmallVec;
use tspp_dir::{
    Expression, GenericArgument, LocalNodeId, Name, NodeType, Path, StringId, TokenLiteral,
    TokenType, TreeAttribute, TreeAttributeValue, TreeChild,
};
use tspp_source::{ByteRange, NodeSpanRegion, NodeSpanType};

/// One open tree literal awaiting children.
struct TreeFrame {
    /// The parse start for the tree literal.
    start: ParseStart,
    /// The tag path.
    path: Option<Path>,
    /// The source ranges for the tag path segments.
    path_segment_ranges: Option<SmallVec<[ByteRange; 3]>>,
    /// The generic arguments.
    generic_arguments: Vec<LocalNodeId<GenericArgument>>,
    /// The tag attributes.
    attributes: Option<Vec<LocalNodeId<TreeAttribute>>>,
    /// The children collected so far.
    children: Vec<LocalNodeId<TreeChild>>,
    /// The range of the opening tag.
    opening_range: ByteRange,
    /// The source start of the tree body.
    body_start: u32,
    /// The tokenization mode after this literal closes.
    close_follow_mode: TokenMode,
}

/// One tree literal closing tag.
struct TreeLiteralClose {
    /// The closing tag path, or none for a fragment.
    path: Option<Path>,
    /// The complete closing tag range.
    range: ByteRange,
}

impl Parser {
    /// Return true when the current token starts a tree literal.
    #[inline]
    pub fn peek_tree_literal_start(&self) -> bool {
        self.peek_tree_literal_start_at(0)
    }

    /// Return true when an offset token starts a tree literal.
    #[inline]
    pub(crate) fn peek_tree_literal_start_at(&self, offset: usize) -> bool {
        if self.peek_token_type_at(offset) != TokenType::LessThan {
            return false;
        }

        let peek_next_token_type = self.peek_token_type_at(offset + 1);
        if peek_next_token_type == TokenType::Divide {
            return false;
        }

        let is_tag_close = Self::is_type_angle_close_start(peek_next_token_type);
        if !is_tag_close
            && !matches!(
                peek_next_token_type,
                TokenType::Divide | TokenType::Identifier
            )
        {
            return false;
        }

        if peek_next_token_type == TokenType::Identifier {
            return self.peek_token_type_at(offset + 2) != TokenType::Comma;
        }

        true
    }

    /// Parse one tree child and advance in the requested tree mode after delimiters.
    ///
    /// Examples:
    /// ```tspp
    /// text
    /// {value}
    /// {...children}
    /// <Widget prop=value />
    /// ```
    pub(crate) fn parse_tree_child(
        &mut self,
        follow_mode: TokenMode,
    ) -> ParserResult<LocalNodeId<TreeChild>> {
        let start = self.mark_parse_start();

        // expression container
        if self.peek_is(TokenType::OpenBrace) {
            let container_start = self.mark_parse_start();
            self.bump_with_mode(TokenMode::Ordinary);

            if self.peek_is(TokenType::CloseBrace) {
                self.bump_with_mode(follow_mode);
                return Ok(self.insert_node(TreeChild::Empty, self.range_since(&start)));
            }

            // parse the optional spread marker and shared value
            let is_spread = self.peek_is(TokenType::Spread);
            if is_spread {
                self.bump();
            }
            let value = self.parse_expression(ExpressionPosition::Value, ExpressionStop::default());
            let value = match value {
                Ok(value) => value,
                Err(error) => {
                    return Ok(self.recover_tree_expression_child(&start, error, follow_mode));
                }
            };

            // recover a missing close as one damaged tree child
            if !self.peek_is(TokenType::CloseBrace) {
                let error = ParserError::unexpected(self.peek_token_span());

                return Ok(self.recover_tree_expression_child(&start, error, follow_mode));
            }
            self.eat_tree_expression_close_brace(follow_mode, NodeType::TreeChild)?;

            self.extend_node_region_range(
                value,
                NodeSpanRegion::TreeContainer,
                self.range_since(&container_start),
            );
            let child = if is_spread {
                TreeChild::Spread { value }
            } else {
                TreeChild::Expression { value }
            };

            return Ok(self.insert_node(child, self.range_since(&start)));
        }

        // text child
        let token = self.peek_token_span();
        let is_tree_text = token.token.ty() == TokenType::Literal
            && matches!(
                token.token.literal(),
                Some(TokenLiteral::TreeString)
                    | Some(TokenLiteral::Character {
                        is_html_entity: true,
                        ..
                    })
            );
        if is_tree_text {
            let value = self.eat_tree_child_text(follow_mode)?;
            return Ok(self.insert_node(TreeChild::Text { value }, self.range_since(&start)));
        }

        // nested tree child
        if self.peek_tree_literal_start() {
            let value = self.parse_tree_literal_in_mode(follow_mode)?;
            return Ok(self.insert_node(TreeChild::Tree { value }, self.range_since(&start)));
        }

        Err(ParserError::unexpected(token))
    }

    /// Eat one tree text child payload and advance in the requested tree mode.
    fn eat_tree_child_text(&mut self, follow_mode: TokenMode) -> ParserResult<StringId> {
        let token = self.peek_token_span();
        let Some(body) = token.token.literal() else {
            return Err(ParserError::unexpected(token));
        };

        let literal_str = self.file.span_str(token.span);
        let value = match body {
            TokenLiteral::Character {
                is_terminated,
                is_html_entity,
                ..
            } => {
                if !is_terminated || !is_html_entity {
                    return Err(
                        ParserError::expected(token.span.range(), TokenType::Literal)
                            .in_node(NodeType::TreeChild),
                    );
                }

                let Some(decoded) = decode_html_entities(literal_str) else {
                    return Err(
                        ParserError::expected(token.span.range(), TokenType::Literal)
                            .in_node(NodeType::TreeChild),
                    );
                };

                self.strings.intern(&decoded)
            }
            TokenLiteral::TreeString => self.strings.intern(literal_str),
            _ => return Err(ParserError::unexpected(token)),
        };

        self.bump_with_mode(follow_mode);

        Ok(value)
    }

    /// Parse a tree literal attribute (e.g., `x=1` or `long-name=2` or `flag-is-set`).
    ///
    /// Examples:
    /// ```tspp
    /// x=1
    /// y
    /// {...args}
    /// ```
    #[inline]
    pub(crate) fn parse_tree_attribute(&mut self) -> ParserResult<LocalNodeId<TreeAttribute>> {
        let start = self.mark_parse_start();
        // spread expression container
        if self.peek_is(TokenType::OpenBrace) {
            let container_start = self.mark_parse_start();
            self.bump_with_mode(TokenMode::Ordinary);
            if !self.peek_is(TokenType::Spread) {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }
            self.bump();
            let value =
                self.parse_expression(ExpressionPosition::Value, ExpressionStop::default())?;
            self.eat_tree_expression_close_brace(TokenMode::TreeTag, NodeType::TreeAttribute)?;
            self.extend_node_region_range(
                value,
                NodeSpanRegion::TreeContainer,
                self.range_since(&container_start),
            );
            let attribute_id =
                self.insert_node(TreeAttribute::Spread { value }, self.range_since(&start));
            Ok(attribute_id)
        }
        // named attribute
        else {
            let (name, name_range) = self.eat_tree_literal_identifier_with_range()?;

            // explicit assignment, including newline wrapped forms
            let has_value_separator = self.peek_tree_attribute_assign_separator();

            // named attribute with value
            let value = if has_value_separator {
                self.bump_with_mode(TokenMode::TreeAttributeValue);

                // tree expression container: attr={expr}
                if self.peek_is(TokenType::OpenBrace) {
                    let container_start = self.mark_parse_start();
                    self.bump_with_mode(TokenMode::Ordinary);
                    let value = self
                        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())?;
                    self.eat_tree_expression_close_brace(
                        TokenMode::TreeTag,
                        NodeType::TreeAttribute,
                    )?;
                    self.extend_node_region_range(
                        value,
                        NodeSpanRegion::TreeContainer,
                        self.range_since(&container_start),
                    );
                    Some(TreeAttributeValue::Expression(value))
                }
                // string literal attribute
                else if self.peek_string_literal_start() {
                    let string = self.eat_tree_attribute_string_literal(TokenMode::TreeTag)?;
                    Some(TreeAttributeValue::String(string))
                }
                // unexpected attribute value
                else {
                    let is_missing_value = self.peek_is(TokenType::Identifier)
                        || self.peek_is(TokenType::Divide)
                        || self.peek_tree_tag_close()
                        || self.peek_is(TokenType::End);
                    if is_missing_value {
                        let error = ParserError::unexpected(self.peek_token_span())
                            .in_node(NodeType::TreeAttribute);
                        self.report_error(error);

                        return Ok(self.insert_node(TreeAttribute::Error, self.range_since(&start)));
                    }

                    return Err(ParserError::unexpected(self.peek_token_span()));
                }
            }
            // implicit boolean true
            else {
                None
            };

            let attribute_id = self.insert_node(
                TreeAttribute::Named {
                    name: Name::Identifier(name),
                    value,
                },
                self.range_since(&start),
            );
            self.tree.set_main_range(attribute_id, name_range);

            Ok(attribute_id)
        }
    }

    /// Parse a quoted tree attribute string literal.
    ///
    /// Examples:
    /// ```tspp
    /// title="Hello"
    /// title='Hello'
    /// title="A&nbsp;B"
    /// title="A&#160;&#xA0;B"
    /// ```
    fn eat_tree_attribute_string_literal(
        &mut self,
        follow_mode: TokenMode,
    ) -> ParserResult<StringId> {
        let token = self.peek_string_literal()?;

        // decode valid entities before interning the attribute value
        let decoded = decode_html_entities(self.string_literal_str(token));
        let string_id = if let Some(decoded) = decoded {
            self.strings.intern(&decoded)
        } else {
            let content_range = ByteRange {
                start: token.start() + 1,
                end: token.end() - 1,
            };
            self.intern_range(content_range)
        };

        self.bump_with_mode(follow_mode);

        Ok(string_id)
    }

    /// Eat a tree expression-container close and advance in the requested mode.
    fn eat_tree_expression_close_brace(
        &mut self,
        follow_mode: TokenMode,
        node_type: NodeType,
    ) -> ParserResult<()> {
        if self.peek_is(TokenType::CloseBrace) {
            self.bump_with_mode(follow_mode);

            return Ok(());
        }

        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, node_type)
    }

    /// Return true when the current tree attribute has an explicit assignment.
    pub(crate) fn peek_tree_attribute_assign_separator(&self) -> bool {
        self.peek_is(TokenType::Assign)
    }

    /// Skip whitespace-only tree content in the requested tokenization mode.
    pub(crate) fn skip_tree_whitespace(&mut self, follow_mode: TokenMode) -> bool {
        let mut skipped = false;
        loop {
            let token = self.peek_token_span();

            // skip non-meaningful whitespace-only tree strings
            if token.token.ty() == TokenType::Literal
                && token.token.literal() == Some(TokenLiteral::TreeString)
            {
                let content = self.span_str(token.span);
                if Self::is_ignored_tree_whitespace(content) {
                    self.bump_with_mode(follow_mode);
                    skipped = true;
                    continue;
                }
            }

            break;
        }

        skipped
    }

    /// Return true when a tree text token is ignored whitespace.
    fn is_ignored_tree_whitespace(content: &str) -> bool {
        let mut has_newline = false;

        for byte in content.bytes() {
            match byte {
                b'\n' | b'\r' => has_newline = true,
                b' ' | b'\t' => {}
                _ => return false,
            }
        }

        has_newline
    }

    /// Parse a tree literal (including the `<` and `>` tokens).
    ///
    /// Examples:
    /// ```tspp
    /// <Entity />
    /// <Entity name="Alfred" active />
    /// <Level difficulty={3}>
    ///     some text
    ///     <Entity name="Alfred" />
    ///     {children.map(child => <Entity name={child.name} />)}
    /// </Level>
    /// ```
    pub(crate) fn parse_tree_literal(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        self.parse_tree_literal_in_mode(TokenMode::Ordinary)
    }

    /// Parse a tree literal and advance in the requested mode after it closes.
    pub(crate) fn parse_tree_literal_in_mode(
        &mut self,
        follow_mode: TokenMode,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let (first, is_self_closing) = self.parse_tree_literal_open(follow_mode)?;
        if is_self_closing {
            return self.insert_tree_literal_expression(first, false, None);
        }
        let mut stack = vec![first];

        loop {
            self.skip_tree_whitespace(TokenMode::TreeChild);

            // preserve the open tree stack when closing tags are missing at EOF
            if self.peek_is(TokenType::End) {
                self.report_unexpected_here(NodeType::Expression);
                return self.recover_unclosed_tree_literals(stack);
            }

            // consume one closing tag and resolve it against the open stack
            if self.peek_tree_literal_close() {
                let closing_start = self.mark_parse_start();
                let closing = match self.parse_tree_literal_close(|path| {
                    stack
                        .iter()
                        .rposition(|tree_literal| tree_literal.path == *path)
                        .map_or(TokenMode::TreeChild, |index| stack[index].close_follow_mode)
                }) {
                    Ok(closing) => closing,
                    Err(error) => {
                        let child = self.recover_tree_child(&closing_start, error);
                        let parent = stack
                            .last_mut()
                            .ok_or_else(|| ParserError::unexpected(self.peek_token().range()))?;
                        parent.children.push(child);

                        continue;
                    }
                };
                let Some(matching_index) = stack
                    .iter()
                    .rposition(|tree_literal| tree_literal.path == closing.path)
                else {
                    let closing_span = closing.range;
                    let error = ParserError::unexpected(closing.range);
                    let child = self.insert_tree_error_child(closing_span, error);
                    let parent = stack
                        .last_mut()
                        .ok_or_else(|| ParserError::unexpected(self.peek_token().range()))?;
                    parent.children.push(child);

                    continue;
                };

                // recover every unclosed child before the matching ancestor
                while stack.len() > matching_index + 1 {
                    self.recover_unclosed_tree_literal(&mut stack, closing.range)?;
                }

                let tree_literal = stack
                    .pop()
                    .ok_or_else(|| ParserError::unexpected(closing.range))?;
                let body_range = ByteRange {
                    start: tree_literal.body_start,
                    end: closing.range.start,
                };
                let expression =
                    self.insert_tree_literal_expression(tree_literal, true, Some(body_range))?;
                if let Some(parent) = stack.last_mut() {
                    let child = self.insert_tree_literal_child(expression);
                    parent.children.push(child);

                    continue;
                }

                return Ok(expression);
            }

            if self.peek_tree_literal_start() {
                let child_start = self.mark_parse_start();
                let child = self.parse_tree_literal_open(TokenMode::TreeChild);
                let child = match child {
                    Ok(child) => child,
                    Err(error) => {
                        let child = self.recover_tree_child(&child_start, error);
                        let Some(parent) = stack.last_mut() else {
                            return Err(ParserError::unexpected(self.peek_token().range()));
                        };
                        parent.children.push(child);
                        continue;
                    }
                };

                let (tree_literal, is_self_closing) = child;
                if is_self_closing {
                    let expression_id =
                        self.insert_tree_literal_expression(tree_literal, false, None)?;
                    let child = self.insert_tree_literal_child(expression_id);
                    let Some(parent) = stack.last_mut() else {
                        return Err(ParserError::unexpected(self.peek_token().range()));
                    };
                    parent.children.push(child);
                } else {
                    stack.push(tree_literal);
                }
                continue;
            }

            let child_start = self.mark_parse_start();
            let child = self.parse_tree_child(TokenMode::TreeChild);

            let child = match child {
                Ok(child) => child,
                Err(error) => self.recover_tree_child(&child_start, error),
            };
            let Some(parent) = stack.last_mut() else {
                return Err(ParserError::unexpected(self.peek_token().range()));
            };
            parent.children.push(child);
        }
    }

    /// Recover open tree literals at EOF while preserving their children.
    fn recover_unclosed_tree_literals(
        &mut self,
        mut stack: Vec<TreeFrame>,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let body_end = self.peek_token().range().start;

        loop {
            let Some(tree_literal) = stack.pop() else {
                return Err(ParserError::unexpected(self.peek_token().range()));
            };
            let body_range = ByteRange {
                start: tree_literal.body_start,
                end: body_end,
            };
            let expression_id =
                self.insert_tree_literal_expression(tree_literal, true, Some(body_range))?;

            // attach each completed nested tree to its open parent
            if let Some(parent) = stack.last_mut() {
                let child = self.insert_tree_literal_child(expression_id);
                parent.children.push(child);
                continue;
            }

            return Ok(expression_id);
        }
    }

    /// Finish one unclosed nested tree before an ancestor closing tag.
    fn recover_unclosed_tree_literal(
        &mut self,
        stack: &mut Vec<TreeFrame>,
        closing_range: ByteRange,
    ) -> ParserResult<()> {
        let error = ParserError::unexpected(closing_range).in_node(NodeType::Expression);
        self.report_error(error);

        let Some(tree_literal) = stack.pop() else {
            return Err(ParserError::unexpected(self.peek_token().range()));
        };
        let body_range = ByteRange {
            start: tree_literal.body_start,
            end: closing_range.start,
        };
        let expression_id =
            self.insert_tree_literal_expression(tree_literal, true, Some(body_range))?;

        let Some(parent) = stack.last_mut() else {
            return Err(ParserError::unexpected(self.peek_token().range()));
        };
        let child = self.insert_tree_literal_child(expression_id);
        parent.children.push(child);

        Ok(())
    }

    /// Parse one tree literal opening.
    fn parse_tree_literal_open(
        &mut self,
        follow_mode: TokenMode,
    ) -> ParserResult<(TreeFrame, bool)> {
        let start = self.mark_parse_start();
        self.eat_tree_opening_angle()?;

        // parse the tag path
        let mut path_segment_ranges = None;
        let path: Option<Path> = if self.peek_is(TokenType::Identifier) {
            let RangedPath {
                path,
                segment_ranges,
            } = self.parse_tree_path()?;

            // tree namespace names cannot be followed by member access
            if self.has_tree_literal_namespace_member(&path) {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }

            path_segment_ranges = Some(segment_ranges);
            Some(path)
        } else {
            None
        };

        // generic arguments on the tag: typed and tree-tag components
        let generic_arguments = if path.is_some()
            && (self.peek_is(TokenType::LessThan) || self.peek_is(TokenType::ShiftLeft))
        {
            self.parse_generic_argument_list(ExpressionPosition::Tree)?
        } else {
            Vec::new()
        };

        // parse tag attributes
        let attributes = self.parse_tree_literal_attributes()?;

        // close self-closing tags immediately
        if self.peek_is(TokenType::Divide) {
            self.bump();
            self.eat_tree_tag_close(follow_mode)?;
            let opening_range = self.range_since(&start);
            let tree_literal = TreeFrame {
                start,
                path,
                path_segment_ranges,
                generic_arguments,
                attributes,
                children: Vec::new(),
                opening_range,
                body_start: opening_range.end,
                close_follow_mode: follow_mode,
            };
            return Ok((tree_literal, true));
        }

        // enter tree child tokenization after an opening tag
        self.eat_tree_tag_close(TokenMode::TreeChild)?;
        let opening_range = self.range_since(&start);
        let body_start = opening_range.end;
        self.skip_tree_whitespace(TokenMode::TreeChild);

        Ok((
            TreeFrame {
                start,
                path,
                path_segment_ranges,
                generic_arguments,
                attributes,
                children: Vec::new(),
                opening_range,
                body_start,
                close_follow_mode: follow_mode,
            },
            false,
        ))
    }

    /// Parse tree literal header attributes.
    fn parse_tree_literal_attributes(
        &mut self,
    ) -> ParserResult<Option<Vec<LocalNodeId<TreeAttribute>>>> {
        self.skip_tree_whitespace(TokenMode::Ordinary);
        if self.peek_is(TokenType::Divide) || self.peek_tree_tag_close() {
            return Ok(None);
        }

        let mut attributes = Vec::new();
        while self.has_more_tokens() {
            self.skip_tree_whitespace(TokenMode::Ordinary);

            // leave enclosing closing tags for the open tree stack
            if self.peek_tree_literal_close() {
                return Err(ParserError::expected(
                    self.peek_token_span(),
                    TokenType::GreaterThan,
                ));
            }

            if self.peek_is(TokenType::Divide) || self.peek_tree_tag_close() {
                break;
            }

            let attribute_start = self.mark_parse_start();
            let attribute = self.parse_tree_attribute();

            let attribute = match attribute {
                Ok(attribute) => attribute,
                Err(error)
                    if Self::is_any_stop_token(self.peek_token_type())
                        || self.peek_tree_literal_close() =>
                {
                    return Err(error);
                }
                Err(error) => self.recover_tree_attribute(&attribute_start, error),
            };
            attributes.push(attribute);
        }

        Ok(Some(attributes))
    }

    /// Parse one tree literal closing tag.
    fn parse_tree_literal_close(
        &mut self,
        follow_mode: impl FnOnce(&Option<Path>) -> TokenMode,
    ) -> ParserResult<TreeLiteralClose> {
        if !self.peek_tree_literal_close() {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        let start = self.mark_parse_start();
        self.bump_with_mode(TokenMode::TreeTag);
        self.eat_token(TokenType::Divide)?;

        let path = if self.peek_tree_tag_close() {
            None
        } else {
            let path = self.parse_tree_path()?.path;
            if self.has_tree_literal_namespace_member(&path) {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }

            Some(path)
        };

        self.skip_tree_whitespace(TokenMode::Ordinary);
        let follow_mode = follow_mode(&path);
        self.eat_tree_tag_close(follow_mode)?;
        let range = self.range_since(&start);

        Ok(TreeLiteralClose { path, range })
    }

    /// Insert one tree literal expression from an open tree literal.
    fn insert_tree_literal_expression(
        &mut self,
        tree_literal: TreeFrame,
        has_children: bool,
        body_range: Option<ByteRange>,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let TreeFrame {
            start,
            path,
            path_segment_ranges,
            generic_arguments,
            attributes,
            children,
            opening_range,
            body_start: _,
            close_follow_mode: _,
        } = tree_literal;

        let left = if let Some(path) = path {
            let Some(segment_ranges) = path_segment_ranges.as_deref() else {
                return Err(ParserError::unexpected(opening_range));
            };
            if segment_ranges.is_empty() {
                return Err(ParserError::unexpected(opening_range));
            }

            Some(self.insert_member_chain(&path.segments, segment_ranges)?)
        } else {
            None
        };

        let children = has_children.then_some(children);
        let expression = Expression::TreeExpression {
            left,
            generic_arguments,
            attributes,
            children,
        };
        let expression_id = self.insert_node(expression, self.range_since(&start));
        self.tree.set_side_range(
            expression_id,
            NodeSpanType::Region(NodeSpanRegion::Opening),
            opening_range,
        );
        if let Some(body_range) = body_range {
            self.tree.set_side_range(
                expression_id,
                NodeSpanType::Region(NodeSpanRegion::Body),
                body_range,
            );
        }

        Ok(expression_id)
    }

    /// Insert one tree child for a tree literal expression.
    fn insert_tree_literal_child(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<TreeChild> {
        self.insert_node(
            TreeChild::Tree {
                value: expression_id,
            },
            self.tree.get_range(expression_id),
        )
    }

    /// Return true when a tree literal path combines namespace and member syntax.
    pub(crate) fn has_tree_literal_namespace_member(&self, path: &Path) -> bool {
        if path.segments.len() <= 1 {
            return false;
        }

        path.segments
            .iter()
            .any(|segment| self.strings.get(*segment).contains(':'))
    }
}
