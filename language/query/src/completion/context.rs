use destack_dir as dir;
use destack_source::{EnclosingSpan, ModuleId, NodeSpanRegion};

use crate::source::token_text;
use crate::{
    ExpectedParameterHint, ExpressionSlotPosition, ModuleQueryContext, ObjectLiteralCursorContext,
    ScopeAtOffset, SymbolUse,
};

/// Describes the context for a completion request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompletionContext {
    /// Member access context such as `value.`.
    MemberAccess {
        /// The receiver node id.
        receiver_node: dir::LocalNodeIdAny,
        /// The resolved receiver symbol when available.
        receiver_symbol: Option<dir::GlobalSymbolId>,
        /// The resolved receiver type when available.
        receiver_type: Option<dir::GlobalTypeId>,
    },
    /// Type position context such as decorations or type expressions.
    TypePosition {
        /// The scope used for visible symbols.
        scope: ScopeAtOffset,
    },
    /// Value position context such as expressions and statements.
    ValuePosition {
        /// The scope used for visible symbols.
        scope: ScopeAtOffset,
    },
    /// Statement position context such as the start of a line in a block.
    StatementPosition {
        /// The scope used for visible symbols.
        scope: ScopeAtOffset,
    },
    /// Object literal key context inside `dir::Type { ... }`.
    ObjectLiteral {
        /// The object expression node id.
        object_node: dir::LocalNodeIdAny,
        /// The contextual type when one is available.
        contextual_type: Option<dir::GlobalTypeId>,
        /// Field names already present in the literal.
        existing_fields: Vec<String>,
        /// The scope used for visible symbols.
        scope: ScopeAtOffset,
    },
    /// Object literal value context inside `dir::Type { key: value }`.
    ObjectLiteralValue {
        /// The scope used for visible symbols.
        scope: ScopeAtOffset,
    },
    /// Call argument context inside `call(...)`.
    CallArgument {
        /// The scope used for visible symbols.
        scope: ScopeAtOffset,
        /// The active parameter hint when one is available.
        expected_parameter: Option<ExpectedParameterHint>,
    },
    /// New expression context inside `new ...`.
    NewExpression {
        /// The scope used for visible symbols.
        scope: ScopeAtOffset,
    },
    /// Import path context inside string literals.
    ImportPath {
        /// The partial path typed so far.
        partial_path: String,
    },
    /// Import clause context inside `{ ... }` import lists.
    ImportClause {
        /// The target module id when one is available.
        target_module: Option<ModuleId>,
        /// Names already present in the import clause.
        existing_names: Vec<String>,
        /// Optional use filter for the clause.
        use_filter: Option<SymbolUse>,
    },
    /// Explicitly suppressed completion context.
    Suppressed,
}

impl CompletionContext {
    /// Create a type-position context.
    fn type_position(scope: ScopeAtOffset) -> Self {
        Self::TypePosition { scope }
    }

    /// Create a value-position context.
    fn value_position(scope: ScopeAtOffset) -> Self {
        Self::ValuePosition { scope }
    }

    /// Create a statement-position context.
    fn statement_position(scope: ScopeAtOffset) -> Self {
        Self::StatementPosition { scope }
    }

    /// Create a context from one expression slot position.
    fn expression_slot(position: ExpressionSlotPosition, scope: ScopeAtOffset) -> Self {
        match position {
            ExpressionSlotPosition::Constructor => Self::NewExpression { scope },
            ExpressionSlotPosition::Type => Self::type_position(scope),
            ExpressionSlotPosition::Value => Self::value_position(scope),
        }
    }
}

/// Completion-specific expression predicates.
trait CompletionExpression {
    /// Return whether this expression is a known type expression form.
    fn is_type_expression(&self) -> bool;
}

impl CompletionExpression for dir::Expression {
    fn is_type_expression(&self) -> bool {
        matches!(self, dir::Expression::Type { .. })
    }
}

/// A token extracted at the cursor for prefix matching.
#[derive(Debug, Clone)]
pub(crate) struct CursorToken {
    /// The token text.
    pub(crate) text: String,
    /// The start offset of the token.
    pub(crate) start: u32,
}

/// The detected completion context plus token data.
#[derive(Debug, Clone)]
pub(crate) struct CompletionCursor {
    /// The completion context.
    pub(crate) context: CompletionContext,
    /// The token at the cursor when available.
    pub(crate) token: Option<CursorToken>,
}

impl CompletionCursor {
    /// Create a suppressed completion cursor.
    pub(crate) fn suppressed() -> Self {
        Self {
            context: CompletionContext::Suppressed,
            token: None,
        }
    }
}

impl ModuleQueryContext<'_> {
    /// Return the completion context at a given offset.
    pub(crate) fn completion_cursor(&self, offset: u32) -> CompletionCursor {
        let source_file = self.source_file();
        let source = source_file.text();

        // resolve token prefix at the cursor
        let token = self.partial_identifier(source, offset);

        // member access stays first because it is the most specific value position context
        if let Some(context) = self.member_access_context(&token, offset) {
            return CompletionCursor { context, token };
        }

        // check object literal context before type position to avoid comma misclassification
        if let Some(object_context) = self.object_literal_context_at_offset(offset) {
            let completion_context = match object_context {
                ObjectLiteralCursorContext::Key(object_context) => {
                    CompletionContext::ObjectLiteral {
                        object_node: object_context.object_node,
                        contextual_type: object_context.contextual_type,
                        existing_fields: object_context.existing_fields,
                        scope: object_context.scope,
                    }
                }
                ObjectLiteralCursorContext::Value(scope) => {
                    CompletionContext::ObjectLiteralValue { scope }
                }
            };

            return CompletionCursor {
                context: completion_context,
                token,
            };
        }

        // check for explicit constructor typing before call arguments
        if let Some(new_context) = self.new_expression_cursor_context(offset) {
            return CompletionCursor {
                context: new_context,
                token,
            };
        }

        // check for call argument context
        if let Some(call_context) = self.call_argument_cursor_context(offset) {
            return CompletionCursor {
                context: call_context,
                token,
            };
        }

        // check for import context
        if let Some(import_context) = self.import_context(source, offset) {
            return CompletionCursor {
                context: import_context,
                token,
            };
        }

        // check for type position via source spans
        if self.is_type_position(source, offset) {
            let Some(scope) = self.scope_at_offset(offset) else {
                return CompletionCursor::suppressed();
            };

            return CompletionCursor {
                context: CompletionContext::type_position(scope),
                token,
            };
        }

        // treat structurally classified missing expression slots as real completion positions
        if let Some(position) = self.expression_slot_position(source, offset) {
            let Some(scope) = self.scope_at_offset(offset) else {
                return CompletionCursor::suppressed();
            };
            let completion_context = CompletionContext::expression_slot(position, scope);

            return CompletionCursor {
                context: completion_context,
                token,
            };
        }

        // check for statement position
        if let Some(statement_context) = self.statement_position(source, offset) {
            return CompletionCursor {
                context: statement_context,
                token,
            };
        }

        // suppress unclaimed expression holes
        if self.is_suppressed_completion_position(source, &token, offset) {
            return CompletionCursor {
                context: CompletionContext::Suppressed,
                token,
            };
        }

        let Some(scope) = self.scope_at_offset(offset) else {
            return CompletionCursor::suppressed();
        };

        CompletionCursor {
            context: CompletionContext::value_position(scope),
            token,
        }
    }
}

impl ModuleQueryContext<'_> {
    /// Detect whether completion should stay suppressed at the cursor.
    fn is_suppressed_completion_position(
        &self,
        source: &str,
        token: &Option<CursorToken>,
        offset: u32,
    ) -> bool {
        if token.is_some() {
            return false;
        }

        self.expression_hole_at_offset(offset).is_some()
            && self.expression_slot_position(source, offset).is_none()
    }

    /// Return the partial identifier at the cursor position.
    fn partial_identifier(&self, source: &str, offset: u32) -> Option<CursorToken> {
        let token = match self.token_span_at_offset(offset) {
            Some(token) if token.token.ty() == dir::TokenType::Identifier => token,
            _ => {
                let token = self.previous_significant_token(offset)?;
                if token.token.ty() != dir::TokenType::Identifier {
                    return None;
                }

                if token.span.end != offset {
                    return None;
                }

                token
            }
        };

        let span = token.span;
        let prefix_end = offset.saturating_sub(span.start) as usize;
        let text = token_text(source, span)
            .unwrap_or_else(|| panic!("invalid completion token source range: {span:?}"));
        let prefix_end = prefix_end.min(text.len());

        Some(CursorToken {
            text: text[..prefix_end].to_string(),
            start: span.start,
        })
    }
}

// ================================================================================
// statement contexts
// ================================================================================

impl ModuleQueryContext<'_> {
    /// Return the statement position.
    fn statement_position(&self, source: &str, offset: u32) -> Option<CompletionContext> {
        // treat the start of the file as a statement position
        if offset == 0 {
            let scope = ScopeAtOffset::new(self.namespace_scope(), dir::LocalScopeMark(0));

            return Some(CompletionContext::statement_position(scope));
        }

        // skip declarator initializer holes
        if self.is_declarator_value_hole_at_cursor(source, offset) {
            return None;
        }

        // check block based statement gaps first
        if let Some(scope) = self.statement_position_in_block(offset) {
            return Some(CompletionContext::statement_position(scope));
        }

        // use token based statement boundaries
        if let Some(token) = self.previous_significant_token(offset) {
            let opens_statement = matches!(
                token.token.ty(),
                dir::TokenType::Semicolon | dir::TokenType::OpenBrace | dir::TokenType::CloseBrace
            );
            if opens_statement {
                let scope = self.scope_at_offset(offset)?;

                return Some(CompletionContext::statement_position(scope));
            }
        }

        None
    }

    /// Return whether the cursor is in a type position.
    fn is_type_position(&self, source: &str, offset: u32) -> bool {
        let enclosing = self.enclosing_spans_with_previous(offset);

        // check for type side spans that contain the cursor
        if self.region_owns_cursor(offset, NodeSpanRegion::Type) {
            return true;
        }

        // check enclosing expressions that are known type expressions
        for enc in &enclosing {
            if self.tree().get_node_type(enc.source_id) != dir::NodeType::Expression {
                continue;
            }

            let expr_id = dir::LocalNodeId::<dir::Expression>::new(enc.source_id);
            let expr = self.tree().get(expr_id);

            if expr.is_type_expression() {
                return true;
            }
        }

        // check type declarations for their value expression spans
        if self.is_type_declaration_value_position(&enclosing, offset) {
            return true;
        }

        // avoid treating object literal values as type positions
        if self.is_inside_object_literal_expression(offset) {
            return false;
        }

        self.token_suggests_type_position(source, offset)
    }

    /// Resolve a statement position inside a block expression.
    fn statement_position_in_block(&self, offset: u32) -> Option<ScopeAtOffset> {
        let mut enclosing = self.enclosing_spans_with_previous(offset);
        enclosing.sort_by_key(|enc| enc.length);

        // bail out when there are no spans
        if enclosing.is_empty() {
            return None;
        }

        // scan for the nearest block that opens one statement position
        for enc in &enclosing {
            if self.block_statement_position(enc, offset).is_some() {
                return self.source_block_scope(enc.source_id, offset);
            }
        }

        None
    }

    /// Check whether the preceding token still suggests a type position.
    fn token_suggests_type_position(&self, source: &str, offset: u32) -> bool {
        let Some(token) = self.previous_significant_token(offset) else {
            return false;
        };

        // delimiters often introduce one following type position
        if matches!(
            token.token.ty(),
            dir::TokenType::Colon | dir::TokenType::Comma | dir::TokenType::LessThan
        ) {
            return true;
        }

        // type relation keywords also introduce one following type position
        token.token.ty() == dir::TokenType::Identifier
            && matches!(
                token_text(source, token.span),
                Some("extends") | Some("implements") | Some("is") | Some("as")
            )
    }

    /// Check whether the cursor is inside a type declaration value expression.
    fn is_type_declaration_value_position(&self, enclosing: &[EnclosingSpan], offset: u32) -> bool {
        let previous_offset = offset.saturating_sub(1);

        // scan enclosing declarations for type and value declarations
        for enc in enclosing {
            if self.tree().get_node_type(enc.source_id) != dir::NodeType::Declaration {
                continue;
            }

            let declaration_id = dir::LocalNodeId::<dir::Declaration>::new(enc.source_id);
            let declaration = self.tree().get(declaration_id);

            let dir::Declaration::Type(declaration) = declaration else {
                continue;
            };

            let span = self.tree().source_index.get(declaration.value.id);
            if span.contains(offset) || span.contains(previous_offset) {
                return true;
            }
        }

        false
    }
}
