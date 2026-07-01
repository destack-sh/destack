use crate::parse::scope::TypeScope;
use crate::{Parser, ParserResult, ParserSpanStart};

use destack_core::StringId;
use destack_dir::{
    Expression, GenericArgument, LocalNodeId, NodeType, Path, PostfixPosition, TokenType,
    TypeExpression,
};
use destack_source::Span;

/// Type head that can receive generic arguments.
enum TypeGenericHead {
    /// Reference type head.
    Reference(Path),
    /// Member type head.
    Member {
        /// The type before the member name.
        left: LocalNodeId<TypeExpression>,
        /// The member name.
        name: StringId,
    },
}

impl Parser {
    /// Eat type postfix operators.
    ///
    /// Examples:
    /// ```ds
    /// T[]
    /// T["key"]
    /// Namespace.Type<T>!
    /// ```
    pub(super) fn eat_type_postfix(
        &mut self,
        start: &ParserSpanStart,
        mut left: LocalNodeId<TypeExpression>,
        scope: TypeScope,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        loop {
            let token_type = self.peek_token_type();
            let is_on_new_line = self.current_token_is_on_new_line();

            // static type close
            if scope.is_static && Self::starts_type_angle_close(token_type) {
                break;
            }

            match token_type {
                TokenType::OpenBracket => {
                    left = self.eat_type_index_postfix(left)?;
                }
                TokenType::OpenParenthesis => {
                    if scope.is_new_receiver {
                        break;
                    }

                    let Some(expression_id) = self.eat_type_static_value_call(start, left)? else {
                        break;
                    };
                    left = expression_id;
                }
                TokenType::Dot => {
                    left = self.eat_type_dot_postfix(start, left)?;
                }
                TokenType::Not if !is_on_new_line => {
                    left = self.eat_type_must_postfix(start, left);
                }
                TokenType::LessThan | TokenType::ShiftLeft => {
                    let Some(expression_id) = self.eat_type_generic_postfix(start, left)? else {
                        break;
                    };
                    left = expression_id;
                }
                _ => break,
            }
        }

        Ok(left)
    }

    /// Parse a value call in type syntax.
    ///
    /// Examples:
    /// ```ds
    /// sizeOf<Header>()
    /// namespace.value<T>(argument)
    /// ```
    fn eat_type_static_value_call(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<TypeExpression>,
    ) -> ParserResult<Option<LocalNodeId<TypeExpression>>> {
        let Some((callee, generic_arguments)) = self.type_head_as_value(left) else {
            return Ok(None);
        };
        let expression = self.eat_call(callee, Some(generic_arguments), PostfixPosition::Direct)?;

        Ok(Some(self.insert_node(
            TypeExpression::StaticValue { expression },
            self.get_span_from(start),
        )))
    }

    /// Return the value expression represented by one type head.
    fn type_head_as_value(
        &mut self,
        ty: LocalNodeId<TypeExpression>,
    ) -> Option<(LocalNodeId<Expression>, Vec<LocalNodeId<GenericArgument>>)> {
        match self.tree.get(ty).clone() {
            TypeExpression::Reference {
                path,
                generic_arguments,
            } => {
                let callee = self.type_path_as_value(&path, self.tree.get_span(ty))?;

                Some((callee, generic_arguments))
            }
            TypeExpression::Member {
                left,
                name,
                generic_arguments,
            } => {
                let owner = self.type_head_as_instantiated_value(left)?;
                let callee = self.insert_node(
                    Expression::Member {
                        left: owner,
                        name: Some(name),
                    },
                    self.tree.get_span(ty),
                );

                Some((callee, generic_arguments))
            }
            _ => None,
        }
    }

    /// Return the instantiated value expression represented by one type head.
    fn type_head_as_instantiated_value(
        &mut self,
        ty: LocalNodeId<TypeExpression>,
    ) -> Option<LocalNodeId<Expression>> {
        let (callee, generic_arguments) = self.type_head_as_value(ty)?;
        if generic_arguments.is_empty() {
            return Some(callee);
        }

        Some(self.insert_node(
            Expression::Instantiation {
                left: callee,
                generic_arguments,
            },
            self.tree.get_span(ty),
        ))
    }

    /// Return the value expression represented by a path reference.
    fn type_path_as_value(&mut self, path: &Path, span: Span) -> Option<LocalNodeId<Expression>> {
        let mut segments = path.segments.iter().copied();
        let first = segments.next()?;
        let mut callee = self.insert_node(Expression::Identifier { name: first }, span);

        for segment in segments {
            callee = self.insert_node(
                Expression::Member {
                    left: callee,
                    name: Some(segment),
                },
                span,
            );
        }

        Some(callee)
    }

    /// Parse a type must postfix.
    ///
    /// Examples:
    /// ```ds
    /// T!
    /// Namespace.Type!
    /// Result<T>!
    /// ```
    fn eat_type_must_postfix(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<TypeExpression>,
    ) -> LocalNodeId<TypeExpression> {
        let operator_start = self.span_start();
        self.bump();
        let expression_id = self.insert_node(
            TypeExpression::Must { target_type: left },
            self.get_span_from(start),
        );
        self.tree
            .set_main_span(expression_id, self.get_span_from(&operator_start));

        expression_id
    }

    /// Parse type generic postfix arguments when present.
    ///
    /// Examples:
    /// ```ds
    /// Result<T>
    /// Map<K, V>
    /// Namespace.Type<string>
    /// ```
    fn eat_type_generic_postfix(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<TypeExpression>,
    ) -> ParserResult<Option<LocalNodeId<TypeExpression>>> {
        if self.current_token_is_on_new_line() {
            return Ok(None);
        }

        let previous_head_span = self.tree.get_head_span(left);
        let previous_main_span = self.tree.get_main_span(left);
        let head = match self.tree.get(left) {
            TypeExpression::Reference { path, .. } => TypeGenericHead::Reference(path.clone()),
            TypeExpression::Member { left, name, .. } => TypeGenericHead::Member {
                left: *left,
                name: *name,
            },
            _ => return Ok(None),
        };

        let generic_arguments = self.eat_type_generic_arguments()?;
        let expression = match head {
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

        let expression_id = self.insert_node(expression, self.get_span_from(start));
        if let Some(span) = previous_head_span {
            self.tree.set_head_span(expression_id, span);
        }
        if let Some(span) = previous_main_span {
            self.tree.set_main_span(expression_id, span);
        }

        Ok(Some(expression_id))
    }

    /// Parse a type dot postfix.
    ///
    /// Examples:
    /// ```ds
    /// Namespace.Type
    /// Namespace.Type<T>
    /// A.B.C
    /// ```
    fn eat_type_dot_postfix(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<TypeExpression>,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        self.eat_token(TokenType::Dot)?;
        let (name, _) = self.eat_member_name_with_span()?;
        let generic_arguments = if self.type_generic_arguments_start_here() {
            self.eat_type_generic_arguments()?
        } else {
            Vec::new()
        };

        Ok(self.insert_node(
            TypeExpression::Member {
                left,
                name,
                generic_arguments,
            },
            self.get_span_from(start),
        ))
    }

    /// Parse a type index postfix.
    ///
    /// Examples:
    /// ```ds
    /// T[]
    /// T[K]
    /// Tuple[0]
    /// ```
    fn eat_type_index_postfix(
        &mut self,
        left: LocalNodeId<TypeExpression>,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let start = self.span_start();
        self.eat_token(TokenType::OpenBracket)?;

        // array shorthand
        if self.peek_is(TokenType::CloseBracket) {
            self.bump();
            let left_span = self.tree.get_span(left);
            let index_span = self.get_span_from(&start);

            return Ok(self.insert_node(
                TypeExpression::Array { element: left },
                Span::new(left_span.file, left_span.start, index_span.end),
            ));
        }

        // index access
        let index = self.eat_type_expression_or_recover_missing(
            self.flags.nested().in_type(),
            NodeType::TypeExpression,
        )?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::TypeExpression)?;
        let left_span = self.tree.get_span(left);
        let index_span = self.get_span_from(&start);

        Ok(self.insert_node(
            TypeExpression::Index { left, index },
            Span::new(left_span.file, left_span.start, index_span.end),
        ))
    }
}
