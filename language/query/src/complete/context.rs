use destack_dir as dir;
use destack_source::{EnclosingSpan, FileId, ModuleId, NodeSpanRegion};

use crate::source::token_text;
use crate::{CompletionItemKind, ModuleQueryContext, QueryError, QueryResult, SymbolUse};

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
        scope: dir::LocalScope,
    },
    /// Value position context such as expressions and statements.
    ValuePosition {
        /// The scope used for visible symbols.
        scope: dir::LocalScope,
    },
    /// Statement position context such as the start of a line in a block.
    StatementPosition {
        /// The scope used for visible symbols.
        scope: dir::LocalScope,
    },
    /// Object literal key position.
    ObjectLiteralKey {
        /// The object literal expression node.
        literal: dir::LocalNodeId<dir::Expression>,
        /// The scope used for visible symbols.
        scope: dir::LocalScope,
    },
    /// Object literal value position.
    ObjectLiteralValue {
        /// The scope used for visible symbols.
        scope: dir::LocalScope,
    },
    /// Call argument context inside `call(...)`.
    CallArgument {
        /// The scope used for visible symbols.
        scope: dir::LocalScope,
        /// The selected parameter type.
        expected_type: Option<dir::GlobalTypeId>,
    },
    /// New expression context inside `new ...`.
    NewExpression {
        /// The scope used for visible symbols.
        scope: dir::LocalScope,
    },
    /// Control label position after `break` or `continue`.
    ControlLabel {
        /// The enclosing labels ordered nearest first.
        labels: Vec<dir::StringId>,
    },
    /// Import path context inside string literals.
    ImportPath {
        /// The partial path typed so far.
        path: PartialImportPath,
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
    /// A source member access.
    Access {
        /// The exact DIR member lookup site.
        site: dir::MemberSite,
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

/// One partially authored import path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartialImportPath {
    /// The authored path before the cursor.
    text: String,
    /// The byte offset of the editable path segment.
    segment_start: usize,
}

/// One completion context and its optional identifier prefix.
#[derive(Debug, Clone)]
pub(crate) struct CompletionCursor {
    /// The completion context.
    pub(crate) context: CompletionContext,
    /// The token at the cursor when available.
    pub(crate) token: Option<CursorToken>,
}

/// The constraints for auto-import completion.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AutoImportContext {
    /// The symbol namespace accepted by this position.
    pub(crate) symbol_use: SymbolUse,
    /// The lexical scope used to exclude visible names.
    pub(crate) scope: dir::LocalScope,
    /// Whether candidates must be constructable.
    pub(crate) is_constructable_only: bool,
}

impl CompletionContext {
    /// Return the expected value type when this position has one.
    pub(crate) fn expected_type(&self) -> Option<dir::GlobalTypeId> {
        match self {
            Self::CallArgument { expected_type, .. } => *expected_type,
            _ => None,
        }
    }

    /// Return the auto-import constraints for this completion context.
    pub(crate) fn auto_import_context(&self) -> Option<AutoImportContext> {
        match self {
            Self::ValuePosition { scope }
            | Self::StatementPosition { scope }
            | Self::ObjectLiteralValue { scope }
            | Self::CallArgument { scope, .. } => Some(AutoImportContext {
                symbol_use: SymbolUse::Value,
                scope: *scope,
                is_constructable_only: false,
            }),
            Self::TypePosition { scope } => Some(AutoImportContext {
                symbol_use: SymbolUse::Type,
                scope: *scope,
                is_constructable_only: false,
            }),
            Self::NewExpression { scope } => Some(AutoImportContext {
                symbol_use: SymbolUse::Value,
                scope: *scope,
                is_constructable_only: true,
            }),
            _ => None,
        }
    }
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
            let token = match &context {
                CompletionContext::ImportPath { path } => Some(path.cursor_token(offset)?),
                _ => token,
            };

            return Ok(Some(CompletionCursor { context, token }));
        }

        // ordinary comments and literals never contain language completions
        if self.is_lexically_suppressed(file_id, offset)? {
            return Ok(None);
        }

        // select the label namespace after control transfer keywords
        if let Some(context) = self.classify_control_label(file_id, token.as_ref(), offset)? {
            return Ok(Some(CompletionCursor { context, token }));
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
        if self.is_type_position(file_id, offset)? {
            let Some(scope) = self.scope_at_offset(file_id, offset)? else {
                return Ok(None);
            };

            return Ok(Some(CompletionCursor {
                context: CompletionContext::TypePosition { scope },
                token,
            }));
        }

        // classify statement heads before ordinary value references
        if let Some(context) = self.classify_statement(file_id, offset)? {
            return Ok(Some(CompletionCursor { context, token }));
        }

        // classify authored value references directly
        if self.is_value_position(file_id, offset)? {
            let Some(scope) = self.scope_at_offset(file_id, offset)? else {
                return Ok(None);
            };

            return Ok(Some(CompletionCursor {
                context: CompletionContext::ValuePosition { scope },
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

        Ok(None)
    }
}

impl ModuleQueryContext<'_> {
    /// Classify a control label completion position.
    fn classify_control_label(
        &self,
        file_id: FileId,
        token: Option<&CursorToken>,
        offset: u32,
    ) -> QueryResult<Option<CompletionContext>> {
        let label_start = token.map_or(offset, |token| token.start);
        let Some(previous) = self.previous_significant_token(file_id, label_start)? else {
            return Ok(None);
        };
        if !matches!(
            previous.token.keyword(),
            Some(dir::Keyword::Break | dir::Keyword::Continue)
        ) {
            return Ok(None);
        }

        let view = self.view()?;
        let mut labels = Vec::new();

        // collect labels from enclosing control targets, nearest first
        for enclosing in self.enclosing_spans_at_cursor(file_id, offset)? {
            let Some(node) = view.get_node_id_by_source_id(enclosing.source_id) else {
                continue;
            };
            if node.ty != dir::NodeType::Expression {
                continue;
            }
            let expression = view.get(dir::LocalNodeId::<dir::Expression>::new(node.id));
            let Some(label) = expression.control_label() else {
                continue;
            };
            if !labels.contains(&label) {
                labels.push(label);
            }
        }

        Ok(Some(CompletionContext::ControlLabel { labels }))
    }
}

impl PartialImportPath {
    /// Parse one authored import path prefix.
    pub(crate) fn new(text: String) -> Self {
        let slash_count = text.bytes().filter(|byte| *byte == b'/').count();
        let root_is_incomplete = text.starts_with('@') && slash_count <= 1;
        let separator = if text.starts_with("destack:") {
            text.rfind([':', '/'])
        } else if root_is_incomplete {
            None
        } else {
            text.rfind('/')
        };
        let segment_start = separator.map_or(0, |index| index + 1);

        Self {
            text,
            segment_start,
        }
    }

    /// Build the editable path segment at one source offset.
    pub(crate) fn cursor_token(&self, offset: u32) -> QueryResult<CursorToken> {
        let text = self.text[self.segment_start..].to_string();
        let start = offset
            .checked_sub(text.len() as u32)
            .ok_or(QueryError::invalid("import path completion range"))?;

        Ok(CursorToken {
            text,
            start,
            end: offset,
        })
    }

    /// Project one addressable specifier to its next completion.
    pub(crate) fn completion(&self, specifier: &str) -> Option<(String, CompletionItemKind)> {
        // project relative paths one segment at a time
        if specifier.starts_with("./") || specifier.starts_with("../") {
            if !self.text.is_empty()
                && !self.text.starts_with("./")
                && !self.text.starts_with("../")
            {
                return None;
            }

            return self.segment_completion(specifier);
        }

        // project the builtin root before its public subpaths
        if specifier.starts_with("destack:") {
            return self.package_completion("destack:", specifier);
        }

        // project an external package root before its public subpaths
        let package_end = if specifier.starts_with('@') {
            let scope_end = specifier.find('/')?;
            specifier[scope_end + 1..]
                .find('/')
                .map_or(specifier.len(), |index| scope_end + index + 1)
        } else {
            specifier.find('/').unwrap_or(specifier.len())
        };
        let package = &specifier[..package_end];

        self.package_completion(package, specifier)
    }

    /// Project one package root or subpath.
    fn package_completion(
        &self,
        package: &str,
        specifier: &str,
    ) -> Option<(String, CompletionItemKind)> {
        // complete an unfinished package root as one lexical unit
        if package.starts_with(&self.text) {
            let is_module = package == specifier;
            let mut label = package.to_string();
            if !is_module && !label.ends_with(':') {
                label.push('/');
            }
            let kind = if is_module {
                CompletionItemKind::Module
            } else {
                CompletionItemKind::Folder
            };

            return Some((label, kind));
        }

        // complete only subpaths below the exact package root
        let subpath = self.text.strip_prefix(package)?;
        if !subpath.starts_with('/') && !(package.ends_with(':') && !subpath.is_empty()) {
            return None;
        }

        self.segment_completion(specifier)
    }

    /// Project one specifier through the current path directory.
    fn segment_completion(&self, specifier: &str) -> Option<(String, CompletionItemKind)> {
        let directory = &self.text[..self.segment_start];
        let remainder = specifier.strip_prefix(directory)?;
        if remainder.is_empty() {
            return None;
        }

        match remainder.split_once('/') {
            Some((segment, _)) => Some((format!("{segment}/"), CompletionItemKind::Folder)),
            None => Some((remainder.to_string(), CompletionItemKind::Module)),
        }
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
            let scope = self.module_start_scope()?;

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
    pub(crate) fn is_type_position(&self, file_id: FileId, offset: u32) -> QueryResult<bool> {
        let enclosing = self.enclosing_spans_at_cursor(file_id, offset)?;
        let view = self.view()?;

        // check for type side spans that contain the cursor
        if self.region_owns_cursor(file_id, offset, NodeSpanRegion::Type)? {
            return Ok(true);
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
                return Ok(true);
            }
        }

        // check type declarations for their value expression spans
        if self.is_type_declaration_value_position(&enclosing, offset)? {
            return Ok(true);
        }

        Ok(false)
    }

    /// Return whether the cursor selects an authored value reference.
    fn is_value_position(&self, file_id: FileId, offset: u32) -> QueryResult<bool> {
        let view = self.view()?;

        // accept only identifier expressions that own the cursor
        for enclosing in self.enclosing_spans_at_cursor(file_id, offset)? {
            let Some(node_id) = view.get_node_id_by_source_id(enclosing.source_id) else {
                continue;
            };
            if node_id.ty != dir::NodeType::Expression {
                continue;
            }

            let expression_id = dir::LocalNodeId::<dir::Expression>::new(node_id.id);
            if matches!(view.get(expression_id), dir::Expression::Identifier { .. }) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Resolve a statement position inside a block expression.
    fn statement_scope_in_block(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<dir::LocalScope>> {
        let mut enclosing = self.enclosing_spans_at_cursor(file_id, offset)?;
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
    fn is_type_declaration_value_position(
        &self,
        enclosing: &[EnclosingSpan],
        offset: u32,
    ) -> QueryResult<bool> {
        let previous_offset = offset.checked_sub(1);
        let view = self.view()?;

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
                return Ok(true);
            }
        }

        Ok(false)
    }
}
