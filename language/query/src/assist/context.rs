use destack_source::{EnclosingSpan, FileId, ModuleId};
use destack_workspace::{Repository, Revision};
use {destack_ast as ast, destack_dir as dir};

use crate::ast::{
    ExpressionSlotPosition, block_statement_position, enclosing_missing_expression,
    enclosing_spans_with_previous, expression_slot_position, get_module_by_file_id,
    missing_declarator_value_at_cursor, offset_is_in_ast_type_side_span,
    previous_significant_token, token_span_at_cursor_offset, token_text,
};
use crate::core::{AstQueryContext, DirQueryContext, query_context};
use crate::dir::{
    ExpectedParameterHint, ObjectLiteralCursorContext, ScopeAtOffset,
    is_inside_object_literal_expression, object_literal_cursor_context, scope_at_offset,
    scope_from_block_span,
};

use super::call::{detect_call_argument_context, detect_new_expression_context};
use super::import::detect_import_context;
use super::member::detect_member_access_context;

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
        receiver_type: Option<dir::LocalTypeId>,
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
    /// Object literal key context inside `Type { ... }`.
    ObjectLiteral {
        /// The object expression node id.
        object_node: dir::LocalNodeIdAny,
        /// The contextual type when one is available.
        contextual_type: Option<dir::LocalTypeId>,
        /// Field names already present in the literal.
        existing_fields: Vec<String>,
        /// The scope id if one is available.
        scope_id: Option<dir::LocalScopeId>,
        /// The scope mark for ordering visible symbols.
        scope_mark: Option<dir::LocalScopeMark>,
    },
    /// Object literal value context inside `Type { key: value }`.
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

/// Build a fallback completion input from raw source text only.
fn fallback_completion_input(source: &str, offset: u32) -> CompletionInput {
    let token = detect_partial_identifier_from_source(source, offset);
    let context = if token.is_some() {
        CompletionContext::ValuePosition {
            scope_id: None,
            scope_mark: None,
        }
    } else {
        CompletionContext::Unknown
    };

    CompletionInput { context, token }
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

/// Detect whether completion should stay suppressed at the cursor.
fn is_suppressed_completion_position(
    ast: AstQueryContext<'_>,
    source: &str,
    token: &Option<CursorToken>,
    offset: u32,
) -> bool {
    if token.is_some() {
        return false;
    }

    enclosing_missing_expression(ast, offset).is_some()
        && expression_slot_position(ast, source, offset).is_none()
}

/// Detect the completion context at a given offset.
pub(crate) fn completion_input_at_offset(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
    offset: u32,
) -> CompletionInput {
    let Some(source_file) = repository.file(revision, file_id).ok().flatten() else {
        return fallback_completion_input("", offset);
    };
    let source = source_file.text();

    // resolve the module for this file
    let Some(module) = get_module_by_file_id(repository, revision, file_id) else {
        return fallback_completion_input(source, offset);
    };

    // resolve the query context from the module
    let Some(ctx) = query_context(repository, revision, module.id) else {
        return fallback_completion_input(source, offset);
    };
    let ast = ctx.ast();
    let dir = ctx.dir();

    // resolve token prefix at the cursor
    let token = detect_partial_identifier(ast, source, offset);

    // member access stays first because it is the most specific value-position context
    if let Some(context) = detect_member_access_context(ast, dir, &token, offset) {
        return CompletionInput { context, token };
    }

    // check object literal context before type position to avoid comma misclassification
    if let Some(object_context) = object_literal_cursor_context(ast, dir, offset, repository) {
        let context = match object_context {
            ObjectLiteralCursorContext::Key(object_context) => CompletionContext::ObjectLiteral {
                object_node: object_context.object_node,
                contextual_type: object_context.contextual_type,
                existing_fields: object_context.existing_fields,
                scope_id: Some(object_context.scope.scope_id),
                scope_mark: Some(object_context.scope.scope_mark),
            },
            ObjectLiteralCursorContext::Value(scope) => CompletionContext::ObjectLiteralValue {
                scope_id: Some(scope.scope_id),
                scope_mark: Some(scope.scope_mark),
            },
        };

        return CompletionInput { context, token };
    }

    // check for explicit constructor typing before call arguments
    if let Some(new_context) = detect_new_expression_context(ast, dir, offset) {
        return CompletionInput {
            context: new_context,
            token,
        };
    }

    // check for call argument context
    if let Some(call_context) = detect_call_argument_context(repository, ast, dir, offset) {
        return CompletionInput {
            context: call_context,
            token,
        };
    }

    // check for import context
    if let Some(import_context) = detect_import_context(repository, ast, dir, source, offset) {
        return CompletionInput {
            context: import_context,
            token,
        };
    }

    // check for type position via ast spans
    if detect_type_position(ast, source, offset) {
        let scope = scope_at_offset(ast, dir, offset);
        return CompletionInput {
            context: type_context_from_scope(scope),
            token,
        };
    }

    // treat structurally classified missing expression slots as real completion positions
    if let Some(position) = expression_slot_position(ast, source, offset) {
        let scope = scope_at_offset(ast, dir, offset);
        let context = context_from_expression_slot_position(position, scope);

        return CompletionInput { context, token };
    }

    // check for statement position
    if let Some(statement_context) = detect_statement_position(ast, dir, source, offset) {
        return CompletionInput {
            context: statement_context,
            token,
        };
    }

    // keep naked missing expression holes suppressed until a richer context claims them
    if is_suppressed_completion_position(ast, source, &token, offset) {
        return CompletionInput {
            context: CompletionContext::Suppressed,
            token,
        };
    }

    let scope = scope_at_offset(ast, dir, offset);
    CompletionInput {
        context: value_context_from_scope(scope),
        token,
    }
}

/// Detect partial identifier at the cursor position.
fn detect_partial_identifier(
    ast: AstQueryContext<'_>,
    source: &str,
    offset: u32,
) -> Option<CursorToken> {
    // find the token under the cursor or immediately before it
    let token = match token_span_at_cursor_offset(ast, offset) {
        Some(token) if token.token.ty == ast::TokenType::Identifier => token,
        _ => {
            let token = previous_significant_token(ast, offset)?;
            if token.token.ty != ast::TokenType::Identifier {
                return None;
            }

            if token.span.end != offset {
                return None;
            }

            token
        }
    };

    // resolve the token span and prefix end
    let span = token.span;
    let prefix_end = offset.saturating_sub(span.start) as usize;

    // slice the token text from the source
    let text = token_text(source, span)?;
    let prefix_end = prefix_end.min(text.len());

    Some(CursorToken {
        text: text[..prefix_end].to_string(),
        start: span.start,
    })
}

/// Detect one partial identifier directly from raw source text.
fn detect_partial_identifier_from_source(source: &str, offset: u32) -> Option<CursorToken> {
    let offset = usize::try_from(offset).ok()?.min(source.len());
    if !source.is_char_boundary(offset) {
        return None;
    }

    let start = source[..offset]
        .char_indices()
        .rev()
        .find_map(|(index, character)| {
            (!is_identifier_character(character)).then_some(index + character.len_utf8())
        })
        .unwrap_or(0);
    let text = &source[start..offset];
    if text.is_empty() {
        return None;
    }

    let first = text.chars().next()?;
    if !(first == '_' || first.is_alphabetic()) {
        return None;
    }
    if !text.chars().all(is_identifier_character) {
        return None;
    }

    Some(CursorToken {
        text: text.to_string(),
        start: u32::try_from(start).ok()?,
    })
}

/// Return true when one character can appear in an identifier.
fn is_identifier_character(character: char) -> bool {
    character == '_' || character.is_alphanumeric()
}

// ================================================================================
// statement contexts
// ================================================================================

/// Detect whether the cursor is at a statement position.
fn detect_statement_position(
    ast: AstQueryContext<'_>,
    dir: DirQueryContext<'_>,
    source: &str,
    offset: u32,
) -> Option<CompletionContext> {
    // treat the start of the file as a statement position
    if offset == 0 {
        let scope = scope_at_offset(ast, dir, offset);
        return Some(statement_context_from_scope(scope));
    }

    // missing declarator initializer holes are not statement gaps
    if missing_declarator_value_at_cursor(ast, source, offset) {
        return None;
    }

    // check block based statement gaps first
    if let Some(scope) = statement_position_from_block(ast, dir, offset) {
        return Some(statement_context_from_scope(Some(scope)));
    }

    // fall back to token-based statement boundaries
    if let Some(token) = previous_significant_token(ast, offset)
        && matches!(
            token.token.ty,
            ast::TokenType::Semicolon | ast::TokenType::OpenBrace | ast::TokenType::CloseBrace
        )
    {
        let scope = scope_at_offset(ast, dir, offset);
        return Some(statement_context_from_scope(scope));
    }

    None
}

/// Build a statement position context from an optional scope.
fn statement_context_from_scope(scope: Option<ScopeAtOffset>) -> CompletionContext {
    CompletionContext::StatementPosition {
        scope_id: scope.map(|scope| scope.scope_id),
        scope_mark: scope.map(|scope| scope.scope_mark),
    }
}

/// Resolve a statement position inside a block expression.
fn statement_position_from_block(
    ast: AstQueryContext<'_>,
    dir: DirQueryContext<'_>,
    offset: u32,
) -> Option<ScopeAtOffset> {
    // resolve enclosing spans at the cursor boundary
    let mut enclosing = enclosing_spans_with_previous(ast, offset);
    enclosing.sort_by_key(|enc| enc.length);

    // bail out when there are no spans
    if enclosing.is_empty() {
        return None;
    }

    // scan for the nearest block that opens one statement position
    for enc in &enclosing {
        if block_statement_position(ast, enc, offset).is_some() {
            return scope_from_block_span(ast, dir, enc.idx, offset);
        }
    }

    None
}

// ================================================================================
// type contexts
// ================================================================================

/// Detect whether the cursor is in a type position.
fn detect_type_position(ast: AstQueryContext<'_>, source: &str, offset: u32) -> bool {
    // resolve enclosing spans from innermost to outermost
    let enclosing = enclosing_spans_with_previous(ast, offset);

    // check for type side spans that contain the cursor
    if offset_is_in_ast_type_side_span(ast, offset) {
        return true;
    }

    // check enclosing expressions that are known type expressions
    for enc in &enclosing {
        if ast.tree().get_node_type(enc.idx) != ast::NodeType::Expression {
            continue;
        }

        let expr_id = ast::LocalNodeId::<ast::Expression>::new(enc.idx);
        let expr = ast.tree().get(expr_id);

        if is_type_expression(expr) {
            return true;
        }
    }

    // check type declarations for their value expression spans
    if is_type_declaration_value_position(ast, &enclosing, offset) {
        return true;
    }

    // avoid treating object literal values as type positions
    if is_inside_object_literal_expression(ast, offset) {
        return false;
    }

    token_suggests_type_position(ast, source, offset)
}

/// Check whether the preceding token still suggests a type position.
fn token_suggests_type_position(ast: AstQueryContext<'_>, source: &str, offset: u32) -> bool {
    let Some(token) = previous_significant_token(ast, offset) else {
        return false;
    };

    // delimiters often introduce one following type position
    if matches!(
        token.token.ty,
        ast::TokenType::Colon | ast::TokenType::Comma | ast::TokenType::LessThan
    ) {
        return true;
    }

    // type relation keywords also introduce one following type position
    token.token.ty == ast::TokenType::Identifier
        && matches!(
            token_text(source, token.span),
            Some("extends") | Some("implements") | Some("is") | Some("as")
        )
}

/// Check whether the cursor is inside a type declaration value expression.
fn is_type_declaration_value_position(
    ast: AstQueryContext<'_>,
    enclosing: &[EnclosingSpan],
    offset: u32,
) -> bool {
    // resolve the previous offset for span checks
    let previous_offset = offset.saturating_sub(1);

    // scan enclosing declarations for type and value declarations
    for enc in enclosing {
        if ast.tree().get_node_type(enc.idx) != ast::NodeType::Declaration {
            continue;
        }

        let declaration_id = ast::LocalNodeId::<ast::Declaration>::new(enc.idx);
        let declaration = ast.tree().get(declaration_id);

        let ast::Declaration::Type(declaration) = declaration else {
            continue;
        };

        let span = ast.tree().source_map.get(declaration.value.id);
        if span.contains(offset) || span.contains(previous_offset) {
            return true;
        }
    }

    false
}

/// Check whether the expression is one known type expression form.
fn is_type_expression(expr: &ast::Expression) -> bool {
    matches!(expr, ast::Expression::Type { .. })
}
