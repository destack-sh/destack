use crate::parse::{ExpressionPosition, TypePosition, TypeStop};
use crate::{ParseStart, Parser, ParserResult};
use tspp_core::StringId;
use tspp_dir::{
    Expression, GenericArgument, LocalNodeId, NodeType, Path, PostfixPosition, TokenType,
    TypeExpression,
};
use tspp_source::{ByteRange, NodeSpanList, NodeSpanType};

/// One type head that can receive generic arguments.
enum TypeGenericHead {
    /// One reference type head.
    Reference(Path),
    /// One member type head.
    Member {
        /// The type before the member name.
        left: LocalNodeId<TypeExpression>,
        /// The member name.
        name: StringId,
    },
}

/// One static type head promoted into value space.
struct TypeValueHead {
    /// The promoted value expression.
    expression: LocalNodeId<Expression>,
    /// The generic arguments retained for a call or instantiation.
    generic_arguments: Vec<LocalNodeId<GenericArgument>>,
}

impl Parser {
    /// Parse all postfix operations owned by one type operand.
    pub(in crate::parse::r#type) fn parse_type_postfix(
        &mut self,
        start: &ParseStart,
        mut left: LocalNodeId<TypeExpression>,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        loop {
            // stop before tokens that cannot continue one type operand
            let token_type = self.peek_token_type();
            let is_postfix_candidate = matches!(
                token_type,
                TokenType::OpenBracket
                    | TokenType::OpenParenthesis
                    | TokenType::Dot
                    | TokenType::Not
                    | TokenType::LessThan
                    | TokenType::ShiftLeft
            );
            if !is_postfix_candidate {
                break;
            }

            // parse one postfix operation without recursive descent
            let is_on_new_line = self.peek_is_on_new_line();
            let next = match token_type {
                TokenType::OpenBracket => Some(self.parse_type_index_postfix(left, stop)?),
                TokenType::OpenParenthesis => self.parse_type_static_call_postfix(start, left)?,
                TokenType::Dot => Some(self.parse_type_member_postfix(start, left)?),
                TokenType::Not if !is_on_new_line => {
                    Some(self.parse_type_must_postfix(start, left))
                }
                TokenType::LessThan | TokenType::ShiftLeft if !is_on_new_line => {
                    self.parse_type_generic_postfix(start, left)?
                }
                _ => None,
            };
            let Some(next) = next else {
                break;
            };

            // continue from the newly wrapped type
            left = next;
        }

        Ok(left)
    }

    /// Parse one value call written from a static type head.
    fn parse_type_static_call_postfix(
        &mut self,
        start: &ParseStart,
        left: LocalNodeId<TypeExpression>,
    ) -> ParserResult<Option<LocalNodeId<TypeExpression>>> {
        let Some(head) = self.promote_type_value_head(left) else {
            return Ok(None);
        };
        let expression = self.parse_call(
            head.expression,
            head.generic_arguments,
            PostfixPosition::Direct,
            ExpressionPosition::Value,
            false,
        )?;
        let ty = self.insert_node(
            TypeExpression::StaticValue { expression },
            self.range_since(start),
        );

        Ok(Some(ty))
    }

    /// Promote one static type head into value space.
    fn promote_type_value_head(
        &mut self,
        ty: LocalNodeId<TypeExpression>,
    ) -> Option<TypeValueHead> {
        let mark = self.tree.mark();
        let Some(head) = self.build_type_value_head(ty) else {
            self.tree.restore_to_mark(mark);

            return None;
        };

        Some(head)
    }

    /// Build one value head.
    fn build_type_value_head(&mut self, ty: LocalNodeId<TypeExpression>) -> Option<TypeValueHead> {
        // capture fields from promotable type heads
        let (head, generic_arguments) = match self.tree.get(ty) {
            TypeExpression::Reference {
                path,
                generic_arguments,
            } => (
                TypeGenericHead::Reference(path.clone()),
                generic_arguments.clone(),
            ),
            TypeExpression::Member {
                left,
                name,
                generic_arguments,
            } => (
                TypeGenericHead::Member {
                    left: *left,
                    name: *name,
                },
                generic_arguments.clone(),
            ),
            _ => return None,
        };
        let range = self.tree.get_range(ty);

        // promote the reference-shaped head into value space
        let expression = match head {
            TypeGenericHead::Reference(path) => self.insert_path_expression(ty, &path, range)?,
            TypeGenericHead::Member { left, name } => {
                let owner = self.build_type_value(left)?;
                let expression = self.insert_node(
                    Expression::Member {
                        left: owner,
                        name: Some(name),
                        is_optional: false,
                    },
                    range,
                );
                let main_range = self.tree.get_main_range(ty)?;
                self.tree.set_main_range(expression, main_range);

                expression
            }
        };

        Some(TypeValueHead {
            expression,
            generic_arguments,
        })
    }

    /// Build one complete static type head in value space.
    fn build_type_value(
        &mut self,
        ty: LocalNodeId<TypeExpression>,
    ) -> Option<LocalNodeId<Expression>> {
        let head = self.build_type_value_head(ty)?;
        if head.generic_arguments.is_empty() {
            return Some(head.expression);
        }

        Some(self.insert_node(
            Expression::Instantiation {
                left: head.expression,
                generic_arguments: head.generic_arguments,
            },
            self.tree.get_range(ty),
        ))
    }

    /// Insert one source-backed type path as a value expression path.
    fn insert_path_expression(
        &mut self,
        ty: LocalNodeId<TypeExpression>,
        path: &Path,
        range: ByteRange,
    ) -> Option<LocalNodeId<Expression>> {
        let first = path.segments.first().copied()?;
        let mut value = self.insert_node(Expression::Identifier { name: first }, range);
        let first_range = self.type_path_segment_range(ty, path, 0)?;
        self.tree.set_main_range(value, first_range);

        for (index, segment) in path.segments.iter().copied().enumerate().skip(1) {
            value = self.insert_node(
                Expression::Member {
                    left: value,
                    name: Some(segment),
                    is_optional: false,
                },
                range,
            );
            let segment_range = self.type_path_segment_range(ty, path, index)?;
            self.tree.set_main_range(value, segment_range);
        }

        Some(value)
    }

    /// Return one exact segment range from a source-backed reference type path.
    fn type_path_segment_range(
        &self,
        ty: LocalNodeId<TypeExpression>,
        path: &Path,
        index: usize,
    ) -> Option<ByteRange> {
        if path.segments.len() == 1 {
            return self.tree.get_main_range(ty);
        }

        let index = u16::try_from(index).ok()?;
        let span_type = NodeSpanType::ListItem(NodeSpanList::Segment, index);

        self.tree.get_side_range(ty, span_type)
    }

    /// Parse one type must postfix.
    fn parse_type_must_postfix(
        &mut self,
        start: &ParseStart,
        left: LocalNodeId<TypeExpression>,
    ) -> LocalNodeId<TypeExpression> {
        let operator_range = self.peek_token().range();
        self.bump();
        let ty = self.insert_node(
            TypeExpression::Must { target_type: left },
            self.range_since(start),
        );
        self.tree.set_main_range(ty, operator_range);

        ty
    }

    /// Parse generic arguments after one compatible type head.
    fn parse_type_generic_postfix(
        &mut self,
        start: &ParseStart,
        left: LocalNodeId<TypeExpression>,
    ) -> ParserResult<Option<LocalNodeId<TypeExpression>>> {
        let previous_head_range = self.tree.get_head_range(left);
        let previous_main_range = self.tree.get_main_range(left);
        let head = match self.tree.get(left) {
            TypeExpression::Reference { path, .. } => TypeGenericHead::Reference(path.clone()),
            TypeExpression::Member { left, name, .. } => TypeGenericHead::Member {
                left: *left,
                name: *name,
            },
            _ => return Ok(None),
        };

        let generic_arguments = self.parse_type_generic_arguments()?;
        let ty = match head {
            TypeGenericHead::Reference(path) => TypeExpression::Reference {
                path,
                generic_arguments,
            },
            TypeGenericHead::Member { left, name } => TypeExpression::Member {
                left,
                name,
                generic_arguments,
            },
        };
        let ty = self.insert_node(ty, self.range_since(start));
        if let Some(range) = previous_head_range {
            self.tree.set_head_range(ty, range);
        }
        if let Some(range) = previous_main_range {
            self.tree.set_main_range(ty, range);
        }

        Ok(Some(ty))
    }

    /// Parse one type member postfix.
    fn parse_type_member_postfix(
        &mut self,
        start: &ParseStart,
        left: LocalNodeId<TypeExpression>,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        self.eat_token(TokenType::Dot)?;
        let (name, name_range) = self.eat_member_name_with_range()?;
        let generic_arguments = if self.peek_type_generic_arguments() {
            self.parse_type_generic_arguments()?
        } else {
            Vec::new()
        };
        let ty = self.insert_node(
            TypeExpression::Member {
                left,
                name,
                generic_arguments,
            },
            self.range_since(start),
        );
        self.tree.set_main_range(ty, name_range);

        Ok(ty)
    }

    /// Parse one array or indexed-access type postfix.
    fn parse_type_index_postfix(
        &mut self,
        left: LocalNodeId<TypeExpression>,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let start = self.mark_parse_start();
        self.eat_token(TokenType::OpenBracket)?;

        // empty brackets form an array type
        if self.peek_is(TokenType::CloseBracket) {
            self.bump();
            let left_range = self.tree.get_range(left);
            let postfix_range = self.range_since(&start);

            return Ok(self.insert_node(
                TypeExpression::Array { element: left },
                ByteRange {
                    start: left_range.start,
                    end: postfix_range.end,
                },
            ));
        }

        // nonempty brackets form an indexed access type
        let index = if self.peek_type_expression_recovery_boundary() {
            self.recover_missing_type_expression_here(NodeType::TypeExpression)
        } else {
            self.parse_type(TypePosition::Type, stop.nest())?
        };
        self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::TypeExpression)?;
        let left_range = self.tree.get_range(left);
        let postfix_range = self.range_since(&start);

        Ok(self.insert_node(
            TypeExpression::Index { left, index },
            ByteRange {
                start: left_range.start,
                end: postfix_range.end,
            },
        ))
    }
}
