use destack_dir as dir;
use destack_source::{EnclosingSpan, ModuleId};

use crate::core::{DirQueryContext, ModuleQueryContext};
use crate::dir::{ExpectedParameterHint, ObjectLiteralCursorContext, ScopeAtOffset};
use crate::source::{ExpressionSlotPosition, token_text};

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
    /// Type position context such as annotations or type expressions.
    TypePosition {
        /// The scope id if one is available.
        scope_id: Option<dir::LocalScopeId>,
        /// The scope mark for ordering visible symbols.
        scope_mark: Option<dir::LocalScopeMark>,
    },
    /// Value position context such as expressions and statements.
    ValuePosition {
        /// The scope id if one is available.
        scope_id: Option<dir::LocalScopeId>,
        /// The scope mark for ordering visible symbols.
        scope_mark: Option<dir::LocalScopeMark>,
    },
    /// Statement position context such as the start of a line in a block.
    StatementPosition {
        /// The scope id if one is available.
        scope_id: Option<dir::LocalScopeId>,
        /// The scope mark for ordering visible symbols.
        scope_mark: Option<dir::LocalScopeMark>,
    },
    /// Object literal key context inside `dir::Type { ... }`.
    ObjectLiteral {
        /// The object expression node id.
        object_node: dir::LocalNodeIdAny,
        /// The contextual type when one is available.
        contextual_type: Option<dir::GlobalTypeId>,
        /// Field names already present in the literal.
        existing_fields: Vec<String>,
        /// The scope id if one is available.
        scope_id: Option<dir::LocalScopeId>,
        /// The scope mark for ordering visible symbols.
        scope_mark: Option<dir::LocalScopeMark>,
    },
    /// Object literal value context inside `dir::Type { key: value }`.
    ObjectLiteralValue {
        /// The scope id if one is available.
        scope_id: Option<dir::LocalScopeId>,
        /// The scope mark for ordering visible symbols.
        scope_mark: Option<dir::LocalScopeMark>,
    },
    /// Call argument context inside `call(...)`.
    CallArgument {
        /// The scope id if one is available.
        scope_id: Option<dir::LocalScopeId>,
        /// The scope mark for ordering visible symbols.
        scope_mark: Option<dir::LocalScopeMark>,
        /// The active parameter hint when one is available.
        expected_parameter: Option<ExpectedParameterHint>,
    },
    /// New expression context inside `new ...`.
    NewExpression {
        /// The scope id if one is available.
        scope_id: Option<dir::LocalScopeId>,
        /// The scope mark for ordering visible symbols.
        scope_mark: Option<dir::LocalScopeMark>,
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
        /// Optional namespace filter for the clause.
        space_filter: Option<dir::SymbolSpace>,
    },
    /// Explicitly suppressed completion context.
    Suppressed,
    /// Fallback context when no other context matches.
    Unknown,
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
pub(crate) struct CompletionInput {
    /// The completion context.
    pub(crate) context: CompletionContext,
    /// The token at the cursor when available.
    pub(crate) token: Option<CursorToken>,
}

impl CompletionInput {
    /// Create an unknown completion input.
    pub(crate) fn unknown() -> Self {
        Self {
            context: CompletionContext::Unknown,
            token: None,
        }
    }
}

/// Build a value position context from an optional scope.
fn value_context_from_scope(scope: Option<ScopeAtOffset>) -> CompletionContext {
    CompletionContext::ValuePosition {
        scope_id: scope.map(|scope| scope.scope_id),
        scope_mark: scope.map(|scope| scope.scope_mark),
    }
}

/// Build a type position context from an optional scope.
fn type_context_from_scope(scope: Option<ScopeAtOffset>) -> CompletionContext {
    CompletionContext::TypePosition {
        scope_id: scope.map(|scope| scope.scope_id),
        scope_mark: scope.map(|scope| scope.scope_mark),
    }
}

/// Build a completion context from a structural expression slot position.
fn context_from_expression_slot_position(
    position: ExpressionSlotPosition,
    scope: Option<ScopeAtOffset>,
) -> CompletionContext {
    match position {
        ExpressionSlotPosition::Constructor => CompletionContext::NewExpression {
            scope_id: scope.map(|scope| scope.scope_id),
            scope_mark: scope.map(|scope| scope.scope_mark),
        },
        ExpressionSlotPosition::Type => type_context_from_scope(scope),
        ExpressionSlotPosition::Value => value_context_from_scope(scope),
    }
}

impl ModuleQueryContext<'_> {
    /// Detect the completion context at a given offset.
    pub(crate) fn completion_input_at_offset(&self, offset: u32) -> CompletionInput {
        let Some(source_file) = self
            .repository()
            .file(self.revision(), self.file_id())
            .ok()
            .flatten()
        else {
            return CompletionInput::unknown();
        };
        let source = source_file.text();
        let dir = self.dir();

        // resolve token prefix at the cursor
        let token = dir.detect_partial_identifier(source, offset);

        // member access stays first because it is the most specific value position context
        if let Some(context) = dir.detect_member_access_context(&token, offset) {
            return CompletionInput { context, token };
        }

        // check object literal context before type position to avoid comma misclassification
        if let Some(object_context) = dir.object_literal_cursor_context(offset) {
            let context = match object_context {
                ObjectLiteralCursorContext::Key(object_context) => {
                    CompletionContext::ObjectLiteral {
                        object_node: object_context.object_node,
                        contextual_type: object_context.contextual_type,
                        existing_fields: object_context.existing_fields,
                        scope_id: Some(object_context.scope.scope_id),
                        scope_mark: Some(object_context.scope.scope_mark),
                    }
                }
                ObjectLiteralCursorContext::Value(scope) => CompletionContext::ObjectLiteralValue {
                    scope_id: Some(scope.scope_id),
                    scope_mark: Some(scope.scope_mark),
                },
            };

            return CompletionInput { context, token };
        }

        // check for explicit constructor typing before call arguments
        if let Some(new_context) = dir.detect_new_expression_context(offset) {
            return CompletionInput {
                context: new_context,
                token,
            };
        }

        // check for call argument context
        if let Some(call_context) = dir.detect_call_argument_context(offset) {
            return CompletionInput {
                context: call_context,
                token,
            };
        }

        // check for import context
        if let Some(import_context) = dir.detect_import_context(source, offset) {
            return CompletionInput {
                context: import_context,
                token,
            };
        }

        // check for type position via source spans
        if dir.detect_type_position(source, offset) {
            let scope = dir.scope_at_offset(offset);
            return CompletionInput {
                context: type_context_from_scope(scope),
                token,
            };
        }

        // treat structurally classified missing expression slots as real completion positions
        if let Some(position) = dir.expression_slot_position(source, offset) {
            let scope = dir.scope_at_offset(offset);
            let context = context_from_expression_slot_position(position, scope);

            return CompletionInput { context, token };
        }

        // check for statement position
        if let Some(statement_context) = dir.detect_statement_position(source, offset) {
            return CompletionInput {
                context: statement_context,
                token,
            };
        }

        // suppress unclaimed missing expression slots
        if dir.is_suppressed_completion_position(source, &token, offset) {
            return CompletionInput {
                context: CompletionContext::Suppressed,
                token,
            };
        }

        let scope = dir.scope_at_offset(offset);
        CompletionInput {
            context: value_context_from_scope(scope),
            token,
        }
    }
}

impl DirQueryContext<'_> {
    /// Detect whether completion should stay suppressed at the cursor.
    fn is_suppressed_completion_position(
        self,
        source: &str,
        token: &Option<CursorToken>,
        offset: u32,
    ) -> bool {
        if token.is_some() {
            return false;
        }

        self.enclosing_missing_expression(offset).is_some()
            && self.expression_slot_position(source, offset).is_none()
    }

    /// Detect partial identifier at the cursor position.
    fn detect_partial_identifier(self, source: &str, offset: u32) -> Option<CursorToken> {
        let token = match self.token_span_at_cursor_offset(offset) {
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
        let text = token_text(source, span)?;
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

/// Build a statement position context from an optional scope.
fn statement_context_from_scope(scope: Option<ScopeAtOffset>) -> CompletionContext {
    CompletionContext::StatementPosition {
        scope_id: scope.map(|scope| scope.scope_id),
        scope_mark: scope.map(|scope| scope.scope_mark),
    }
}

// ================================================================================
// type contexts
// ================================================================================

/// Check whether the expression is one known type expression form.
fn is_type_expression(expr: &dir::Expression) -> bool {
    matches!(expr, dir::Expression::Type { .. })
}

impl DirQueryContext<'_> {
    /// Detect whether the cursor is at a statement position.
    fn detect_statement_position(self, source: &str, offset: u32) -> Option<CompletionContext> {
        // treat the start of the file as a statement position
        if offset == 0 {
            let scope = self.scope_at_offset(offset);
            return Some(statement_context_from_scope(scope));
        }

        // skip missing declarator initializer slots
        if self.missing_declarator_value_at_cursor(source, offset) {
            return None;
        }

        // check block based statement gaps first
        if let Some(scope) = self.statement_position_from_block(offset) {
            return Some(statement_context_from_scope(Some(scope)));
        }

        // recover token based statement boundaries
        if let Some(token) = self.previous_significant_token(offset)
            && matches!(
                token.token.ty(),
                dir::TokenType::Semicolon | dir::TokenType::OpenBrace | dir::TokenType::CloseBrace
            )
        {
            let scope = self.scope_at_offset(offset);
            return Some(statement_context_from_scope(scope));
        }

        None
    }

    /// Detect whether the cursor is in a type position.
    fn detect_type_position(self, source: &str, offset: u32) -> bool {
        let enclosing = self.enclosing_spans_with_previous(offset);

        // check for type side spans that contain the cursor
        if self.offset_is_in_source_type_side_span(offset) {
            return true;
        }

        // check enclosing expressions that are known type expressions
        for enc in &enclosing {
            if self.tree().get_node_type(enc.source_id) != dir::NodeType::Expression {
                continue;
            }

            let expr_id = dir::LocalNodeId::<dir::Expression>::new(enc.source_id);
            let expr = self.tree().get(expr_id);

            if is_type_expression(expr) {
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
    fn statement_position_from_block(self, offset: u32) -> Option<ScopeAtOffset> {
        let mut enclosing = self.enclosing_spans_with_previous(offset);
        enclosing.sort_by_key(|enc| enc.length);

        // bail out when there are no spans
        if enclosing.is_empty() {
            return None;
        }

        // scan for the nearest block that opens one statement position
        for enc in &enclosing {
            if self.block_statement_position(enc, offset).is_some() {
                return self.scope_from_block_span(enc.source_id, offset);
            }
        }

        None
    }

    /// Check whether the preceding token still suggests a type position.
    fn token_suggests_type_position(self, source: &str, offset: u32) -> bool {
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
    fn is_type_declaration_value_position(self, enclosing: &[EnclosingSpan], offset: u32) -> bool {
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
