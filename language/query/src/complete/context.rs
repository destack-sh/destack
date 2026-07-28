use destack_dir as dir;
use destack_source::{EnclosingSpan, FileId, ModuleId, NodeSpanRegion};

use crate::source::token_text;
use crate::{ModuleQueryContext, QueryError, QueryResult, ScopeAtOffset, SymbolUse};

/// Describes the context for a completion request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompletionContext {
    /// Member access context such as `value.`.
    MemberAccess {
        /// The selected receiver.
        receiver: CompletionReceiver,
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
    /// Object literal key position.
    ObjectLiteralKey {
        /// Field names already present in the literal.
        existing_fields: Vec<String>,
        /// The scope used for visible symbols.
        scope: ScopeAtOffset,
    },
    /// Object literal value position.
    ObjectLiteralValue {
        /// The scope used for visible symbols.
        scope: ScopeAtOffset,
    },
    /// Call argument context inside `call(...)`.
    CallArgument {
        /// The scope used for visible symbols.
        scope: ScopeAtOffset,
        /// The selected parameter type.
        expected_type: Option<dir::GlobalTypeId>,
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
}

/// The receiver for one member completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompletionReceiver {
    /// A typed value or type receiver.
    Type {
        /// The receiver type.
        type_id: dir::GlobalTypeId,
        /// Whether the access uses optional chaining.
        is_optional: bool,
    },
    /// An imported module namespace receiver.
    Namespace {
        /// The imported module.
        module_id: ModuleId,
    },
}

/// A token extracted at the cursor for prefix matching.
#[derive(Debug, Clone)]
pub(crate) struct CursorToken {
    /// The token text.
    pub(crate) text: String,
    /// The start offset of the token.
    pub(crate) start: u32,
    /// The end offset of the token.
    pub(crate) end: u32,
}

/// One completion context and its optional identifier prefix.
#[derive(Debug, Clone)]
pub(crate) struct CompletionCursor {
    /// The completion context.
    pub(crate) context: CompletionContext,
    /// The token at the cursor when available.
    pub(crate) token: Option<CursorToken>,
}

/// Auto import search selected by one completion context.
pub(crate) struct AutoImportSearch {
    /// The symbol namespace accepted by this position.
    pub(crate) symbol_use: SymbolUse,
    /// The lexical scope used to exclude visible names.
    pub(crate) scope: ScopeAtOffset,
    /// Whether candidates must be constructable.
    pub(crate) is_constructable_only: bool,
}

impl ModuleQueryContext<'_> {
    /// Classify completion at one offset.
    pub(crate) fn classify_completion(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<CompletionCursor>> {
        let file = self.read_file(file_id)?;
        let source = file.text();

        // resolve token prefix at the cursor
        let token = self.partial_identifier(file_id, source, offset)?;

        // import strings have their own completion language
        if let Some(context) = self.classify_import(file_id, source, offset)? {
            return Ok(Some(CompletionCursor { context, token }));
        }

        // ordinary comments and literals never contain language completions
        if self.is_lexically_suppressed(file_id, offset)? {
            return Ok(None);
        }

        // member access stays first because it is the most specific value position context
        if let Some(context) = self.classify_member_access(file_id, &token, offset)? {
            return Ok(Some(CompletionCursor { context, token }));
        }

        // check object literal context before general expression positions
        if let Some(context) = self.classify_object_literal(file_id, offset)? {
            return Ok(Some(CompletionCursor { context, token }));
        }

        // check for explicit constructor typing before call arguments
        if let Some(context) = self.classify_new_expression(file_id, offset)? {
            return Ok(Some(CompletionCursor { context, token }));
        }

        // check for call argument context
        if let Some(context) = self.classify_call_argument(file_id, offset)? {
            return Ok(Some(CompletionCursor { context, token }));
        }

        // check for type position via source spans
        if self.is_type_position(file_id, offset) {
            let Some(scope) = self.scope_at_offset(file_id, offset)? else {
                return Ok(None);
            };

            return Ok(Some(CompletionCursor {
                context: CompletionContext::TypePosition { scope },
                token,
            }));
        }

        // treat structurally classified missing expression slots as real completion positions
        if self.is_expression_slot_at_offset(file_id, offset)? {
            let Some(scope) = self.scope_at_offset(file_id, offset)? else {
                return Ok(None);
            };
            let completion_context = CompletionContext::ValuePosition { scope };

            return Ok(Some(CompletionCursor {
                context: completion_context,
                token,
            }));
        }

        // check for statement position
        if let Some(context) = self.classify_statement(file_id, offset)? {
            return Ok(Some(CompletionCursor { context, token }));
        }

        // suppress unclaimed expression holes
        if self.is_suppressed_completion_position(file_id, &token, offset)? {
            return Ok(None);
        }

        let Some(scope) = self.scope_at_offset(file_id, offset)? else {
            return Ok(None);
        };

        Ok(Some(CompletionCursor {
            context: CompletionContext::ValuePosition { scope },
            token,
        }))
    }
}

impl ModuleQueryContext<'_> {
    /// Return whether one cursor lies inside a comment or literal token.
    fn is_lexically_suppressed(&self, file_id: FileId, offset: u32) -> QueryResult<bool> {
        if self
            .comments(file_id)?
            .iter()
            .any(|comment| comment.span.contains(offset))
        {
            return Ok(true);
        }

        let is_literal = self
            .token_span_at_offset(file_id, offset)?
            .is_some_and(|token| token.token.ty() == dir::TokenType::Literal);

        Ok(is_literal)
    }

    /// Detect whether completion should stay suppressed at the cursor.
    fn is_suppressed_completion_position(
        &self,
        file_id: FileId,
        token: &Option<CursorToken>,
        offset: u32,
    ) -> QueryResult<bool> {
        if token.is_some() {
            return Ok(false);
        }

        let has_expression_hole = self.expression_hole_at_offset(file_id, offset)?.is_some();
        let is_expression_slot = self.is_expression_slot_at_offset(file_id, offset)?;

        Ok(has_expression_hole && !is_expression_slot)
    }

    /// Return the partial identifier at the cursor position.
    fn partial_identifier(
        &self,
        file_id: FileId,
        source: &str,
        offset: u32,
    ) -> QueryResult<Option<CursorToken>> {
        let token = match self.token_span_at_offset(file_id, offset)? {
            Some(token) if token.token.ty() == dir::TokenType::Identifier => token,
            _ => {
                let Some(token) = self.previous_significant_token(file_id, offset)? else {
                    return Ok(None);
                };
                if token.token.ty() != dir::TokenType::Identifier {
                    return Ok(None);
                }

                if token.span.end != offset {
                    return Ok(None);
                }

                token
            }
        };

        let span = token.span;
        let prefix_end = offset
            .checked_sub(span.start)
            .ok_or(QueryError::invalid(format!(
                "completion cursor: {span:?}, {offset:?}"
            )))?;
        let prefix_end = usize::try_from(prefix_end)
            .map_err(|_| QueryError::invalid(format!("completion cursor: {span:?}, {offset:?}")))?;
        let text = token_text(source, span)?;
        if prefix_end > text.len() {
            return Err(QueryError::invalid(format!(
                "completion cursor: {span:?}, {offset:?}"
            )));
        }

        Ok(Some(CursorToken {
            text: text[..prefix_end].to_string(),
            start: span.start,
            end: span.end,
        }))
    }
}

// ================================================================================
// statement contexts
// ================================================================================

impl ModuleQueryContext<'_> {
    /// Classify statement completion at one offset.
    fn classify_statement(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<CompletionContext>> {
        // treat the start of the file as a statement position
        if offset == 0 {
            let scope = self.module_start_scope();

            return Ok(Some(CompletionContext::StatementPosition { scope }));
        }

        // skip declarator initializer holes
        if self.is_declarator_value_hole(file_id, offset)? {
            return Ok(None);
        }

        // check block based statement gaps first
        if let Some(scope) = self.statement_scope_in_block(file_id, offset)? {
            return Ok(Some(CompletionContext::StatementPosition { scope }));
        }

        // use token based statement boundaries
        if let Some(token) = self.previous_significant_token(file_id, offset)? {
            let opens_statement = matches!(
                token.token.ty(),
                dir::TokenType::Semicolon | dir::TokenType::OpenBrace | dir::TokenType::CloseBrace
            );
            if opens_statement {
                let Some(scope) = self.scope_at_offset(file_id, offset)? else {
                    return Ok(None);
                };

                return Ok(Some(CompletionContext::StatementPosition { scope }));
            }
        }

        Ok(None)
    }

    /// Return whether the cursor is in a type position.
    pub(crate) fn is_type_position(&self, file_id: FileId, offset: u32) -> bool {
        let enclosing = self.enclosing_spans_at_cursor(file_id, offset);
        let view = self.view();

        // check for type side spans that contain the cursor
        if self.region_owns_cursor(file_id, offset, NodeSpanRegion::Type) {
            return true;
        }

        // check enclosing expressions that are known type expressions
        for enclosing_span in &enclosing {
            let Some(node_id) = view.get_node_id_by_source_id(enclosing_span.source_id) else {
                continue;
            };
            if node_id.ty != dir::NodeType::Expression {
                continue;
            }
            let expression_id = dir::LocalNodeId::<dir::Expression>::new(node_id.id);
            let expression = view.get(expression_id);

            if matches!(expression, dir::Expression::Type { .. }) {
                return true;
            }
        }

        // check type declarations for their value expression spans
        if self.is_type_declaration_value_position(&enclosing, offset) {
            return true;
        }

        false
    }

    /// Resolve a statement position inside a block expression.
    fn statement_scope_in_block(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<ScopeAtOffset>> {
        let mut enclosing = self.enclosing_spans_at_cursor(file_id, offset);
        enclosing.sort_by_key(|enclosing_span| enclosing_span.length);

        // bail out when there are no spans
        if enclosing.is_empty() {
            return Ok(None);
        }

        // scan for the nearest block that opens one statement position
        for enclosing_span in &enclosing {
            if self.block_owns_statement_cursor(enclosing_span, offset)? {
                return self.scope_at_offset(file_id, offset);
            }
        }

        Ok(None)
    }

    /// Check whether the cursor is inside a type declaration value expression.
    fn is_type_declaration_value_position(&self, enclosing: &[EnclosingSpan], offset: u32) -> bool {
        let previous_offset = offset.checked_sub(1);
        let view = self.view();

        // scan enclosing declarations for type and value declarations
        for enclosing_span in enclosing {
            let Some(node_id) = view.get_node_id_by_source_id(enclosing_span.source_id) else {
                continue;
            };
            if node_id.ty != dir::NodeType::Declaration {
                continue;
            }
            let declaration_id = dir::LocalNodeId::<dir::Declaration>::new(node_id.id);
            let declaration = view.get(declaration_id);

            let dir::Declaration::Type(declaration) = declaration else {
                continue;
            };

            let span = view.get_span(declaration.value);
            if span.contains(offset)
                || previous_offset.is_some_and(|previous_offset| span.contains(previous_offset))
            {
                return true;
            }
        }

        false
    }
}
