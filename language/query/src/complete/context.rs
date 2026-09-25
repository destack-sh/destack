use tspp_dir as dir;
use tspp_source::{EnclosingSpan, FileId, ModuleId, NodeSpanRegion};

use super::PartialImportPath;
use crate::cursor::Cursor;
use crate::source::token_text;
use crate::{DeclarationUse, ModuleQueryContext, QueryError, QueryResult};

/// The language construct completed at a source position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompletionPosition {
    /// A member name after a receiver.
    MemberAccess {
        /// The selected receiver.
        receiver: CompletionReceiver,
    },
    /// A type expression.
    Type {
        /// The scope used for visible symbols.
        scope: dir::LocalScope,
    },
    /// A value expression.
    Value {
        /// The scope used for visible symbols.
        scope: dir::LocalScope,
        /// The expected value type recorded by checking.
        expected_type: Option<dir::GlobalTypeId>,
    },
    /// A statement in a module or block.
    Statement {
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
    /// A constructor name after `new`.
    Constructor {
        /// The scope used for visible symbols.
        scope: dir::LocalScope,
    },
    /// Control label position after `break` or `continue`.
    ControlLabel {
        /// The enclosing labels ordered nearest first.
        labels: Vec<dir::StringId>,
    },
    /// A module specifier in an import.
    ImportPath {
        /// The partial path typed so far.
        path: PartialImportPath,
    },
    /// An imported name in a named import list.
    ImportClause {
        /// The target module id when one is available.
        target_module: Option<ModuleId>,
        /// Names already present in the import clause.
        existing_names: Vec<dir::StringId>,
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
        /// The declaration use at the member position.
        usage: DeclarationUse,
    },
}

/// A token extracted at the cursor for prefix matching.
#[derive(Debug, Clone)]
pub(crate) struct CompletionPrefix {
    /// The text before the cursor.
    pub(crate) text: String,
    /// The start offset of the token.
    pub(crate) start: u32,
    /// The end offset of the token.
    pub(crate) end: u32,
}

/// One completion position and its editable prefix.
#[derive(Debug, Clone)]
pub(crate) struct CompletionContext {
    /// The selected language construct.
    pub(crate) position: CompletionPosition,
    /// The editable prefix when present.
    pub(crate) prefix: Option<CompletionPrefix>,
}

/// The constraints for auto-import completion.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AutoImportContext {
    /// The declaration use at the cursor.
    pub(crate) usage: DeclarationUse,
    /// The lexical scope used to exclude visible names.
    pub(crate) scope: dir::LocalScope,
}

impl Cursor<'_, '_> {
    /// Classify completion at one offset.
    pub(crate) fn classify_completion(&self) -> QueryResult<Option<CompletionContext>> {
        // read the source text at the selected position
        let file_id = self.file_id;
        let offset = self.offset;
        let file = self.module.read_file(file_id)?;
        let source = file.text();

        // read the identifier prefix at the cursor
        let prefix = self.module.partial_identifier(file_id, source, offset)?;

        // classify module specifiers and imported names
        if let Some(position) = self.classify_import(source)? {
            let prefix = match &position {
                CompletionPosition::ImportPath { path } => Some(path.prefix(offset)?),
                _ => prefix,
            };

            return Ok(Some(CompletionContext { position, prefix }));
        }

        // ordinary comments and literals never contain language completions
        if self.module.is_lexically_suppressed(file_id, offset)? {
            return Ok(None);
        }

        // select the most specific completion position
        let position = if let Some(position) = self.classify_control_label(prefix.as_ref())? {
            Some(position)
        } else if let Some(position) = self.classify_member_access(prefix.as_ref())? {
            Some(position)
        } else if let Some(position) = self.classify_object_literal()? {
            Some(position)
        } else if let Some(position) = self.classify_constructor()? {
            Some(position)
        } else if let Some(position) = self.classify_call_argument()? {
            Some(position)
        } else if self.is_type_position()? {
            self.scope()?
                .map(|scope| CompletionPosition::Type { scope })
        } else if let Some(position) = self.classify_statement()? {
            Some(position)
        } else if self.is_value_position()? || self.is_expression_slot()? {
            self.scope()?.map(|scope| CompletionPosition::Value {
                scope,
                expected_type: None,
            })
        } else {
            None
        };

        Ok(position.map(|position| CompletionContext { position, prefix }))
    }

    /// Classify a control label completion position.
    fn classify_control_label(
        &self,
        prefix: Option<&CompletionPrefix>,
    ) -> QueryResult<Option<CompletionPosition>> {
        // require a control transfer keyword before the label
        let file_id = self.file_id;
        let offset = self.offset;
        let label_start = prefix.map_or(offset, |prefix| prefix.start);
        let Some(previous) = self
            .module
            .previous_significant_token(file_id, label_start)?
        else {
            return Ok(None);
        };
        if !matches!(
            previous.token.keyword(),
            Some(dir::Keyword::Break | dir::Keyword::Continue)
        ) {
            return Ok(None);
        }

        // collect distinct visible labels
        let view = self.module.view()?;
        let mut labels = Vec::new();

        // collect labels from enclosing control targets, nearest first
        for enclosing in self.enclosing() {
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

        Ok(Some(CompletionPosition::ControlLabel { labels }))
    }

    /// Return whether the cursor is in a type position.
    pub(crate) fn is_type_position(&self) -> QueryResult<bool> {
        let offset = self.offset;
        let enclosing = self.enclosing();
        let view = self.module.view()?;

        // check for type side spans that contain the cursor
        if self.is_in_region(NodeSpanRegion::Type)? {
            return Ok(true);
        }

        // accept authored type nodes and type expressions
        for enclosing_span in enclosing {
            let Some(node_id) = view.get_node_id_by_source_id(enclosing_span.source_id) else {
                continue;
            };
            if node_id.ty == dir::NodeType::TypeExpression {
                return Ok(true);
            }
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
        if self
            .module
            .is_type_declaration_value_position(enclosing, offset)?
        {
            return Ok(true);
        }

        Ok(false)
    }

    /// Return whether the cursor selects an authored value reference.
    fn is_value_position(&self) -> QueryResult<bool> {
        let view = self.module.view()?;

        // accept only identifier expressions that own the cursor
        for enclosing in self.enclosing() {
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
}

impl CompletionPosition {
    /// Return the expected value type when this position has one.
    pub(crate) fn expected_type(&self) -> Option<dir::GlobalTypeId> {
        match self {
            Self::Value { expected_type, .. } => *expected_type,
            _ => None,
        }
    }

    /// Return the auto-import constraints for this completion context.
    pub(crate) fn auto_import_context(&self) -> Option<AutoImportContext> {
        match self {
            Self::Value { scope, .. } | Self::Statement { scope } => Some(AutoImportContext {
                usage: DeclarationUse::Expression,
                scope: *scope,
            }),
            Self::Type { scope } => Some(AutoImportContext {
                usage: DeclarationUse::Type,
                scope: *scope,
            }),
            Self::Constructor { scope } => Some(AutoImportContext {
                usage: DeclarationUse::Constructor,
                scope: *scope,
            }),
            _ => None,
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
    ) -> QueryResult<Option<CompletionPrefix>> {
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

        Ok(Some(CompletionPrefix {
            text: text[..prefix_end].to_string(),
            start: span.start,
            end: span.end,
        }))
    }
}

impl ModuleQueryContext<'_> {
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
