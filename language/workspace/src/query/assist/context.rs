use destack_dir as dir;
use destack_source::{FileId, ModuleId};

use crate::Session;
use crate::query::common::get_module_by_file_id;

/// The kind of completion context at a cursor position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionContext {
    /// Completing after a `.` - member access on a type.
    MemberAccess {
        /// The expression to the left of the dot.
        receiver_node: dir::LocalNodeIdAny,
        /// The symbol of the receiver, if resolved.
        receiver_symbol: Option<dir::GlobalSymbolId>,
    },
    /// Completing in a type position (after `:`, in generics, etc.).
    TypePosition,
    /// Completing in a value/expression position.
    ValuePosition {
        /// The scope at the cursor position.
        scope_id: Option<dir::LocalScopeId>,
    },
    /// Completing after `import ... from `.
    ImportPath,
    /// Completing inside an import clause (e.g., `import { | } from "foo"`).
    ImportClause {
        /// The module being imported from.
        target_module: Option<ModuleId>,
    },
    /// Unknown context - provide general completions.
    Unknown,
}

/// Information about the token at the cursor position.
#[derive(Debug, Clone)]
pub struct TokenAtCursor {
    /// The token text (partial identifier being typed).
    pub text: String,
    /// Start offset of the token.
    pub start: u32,
    /// End offset of the token.
    pub end: u32,
}

/// Result of context detection.
#[derive(Debug, Clone)]
pub struct ContextResult {
    /// The detected completion context.
    pub context: CompletionContext,
    /// Token at cursor (partial identifier being typed).
    pub token: Option<TokenAtCursor>,
}

/// Detect the completion context at a given offset.
pub fn detect_completion_context(session: &Session, file_id: FileId, offset: u32) -> ContextResult {
    // get module AST/DIR
    let Some(module) = get_module_by_file_id(session, file_id) else {
        return ContextResult {
            context: CompletionContext::Unknown,
            token: None,
        };
    };
    let module = module.read();
    let Some(ast) = &module.ast else {
        return ContextResult {
            context: CompletionContext::Unknown,
            token: None,
        };
    };
    let profile = session.default_profile_for_module(module.id);
    let Some(dir) = module.dir_maybe(profile) else {
        return ContextResult {
            context: CompletionContext::Unknown,
            token: None,
        };
    };

    // get the source text from the file
    let source_file = session.files.get(file_id);
    let source = source_file.text();

    // detect if we're immediately after a dot
    let after_dot = if offset > 0 {
        source
            .as_bytes()
            .get(offset as usize - 1)
            .map(|&c| c == b'.')
            .unwrap_or(false)
    } else {
        false
    };

    // find partial identifier at cursor
    let token = detect_partial_identifier(source, offset);

    // if after dot, check if there's an expression to the left
    if after_dot {
        // find enclosing AST nodes at the position before the dot
        let before_dot = offset.saturating_sub(1);
        let enclosing = ast
            .tree
            .source_map
            .get_enclosing_spans(before_dot, before_dot);

        // sort by length (smallest first) to get most specific node
        let mut enclosing = enclosing;
        enclosing.sort_by_key(|e| e.length);

        let dir_tree = dir.tree.read();

        // look for a member expression or an expression to the left
        for enc in &enclosing {
            let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enc.idx) else {
                continue;
            };

            if dir_node_id.ty == dir::NodeType::Expression {
                let Ok(expr_id) = dir_node_id.try_into() else {
                    continue;
                };
                let expr = dir_tree.get::<dir::Expression>(expr_id);

                // check if this is a member expression and the cursor is right after the dot
                if let dir::Expression::Member { left, .. } = expr {
                    return ContextResult {
                        context: CompletionContext::MemberAccess {
                            receiver_node: (*left).into(),
                            receiver_symbol: get_expression_symbol(&dir_tree, *left),
                        },
                        token,
                    };
                }

                // otherwise, the expression before the dot is the receiver
                return ContextResult {
                    context: CompletionContext::MemberAccess {
                        receiver_node: dir_node_id,
                        receiver_symbol: expr.target_symbol(),
                    },
                    token,
                };
            }
        }
    }

    // check for type position by looking at characters before cursor
    let in_type_position = detect_type_position(source, offset);

    if in_type_position {
        return ContextResult {
            context: CompletionContext::TypePosition,
            token,
        };
    }

    // check for import context
    if let Some(import_context) = detect_import_context(source, offset) {
        return ContextResult {
            context: import_context,
            token,
        };
    }

    // default to value position
    let scope_id = find_scope_at_offset(ast, dir, offset);

    ContextResult {
        context: CompletionContext::ValuePosition { scope_id },
        token,
    }
}

/// Get the target symbol of an expression if it resolves to one.
fn get_expression_symbol(
    dir_tree: &dir::NodeTree,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::GlobalSymbolId> {
    let expr = dir_tree.get::<dir::Expression>(expr_id);
    expr.target_symbol()
}

/// Detect partial identifier at cursor position.
fn detect_partial_identifier(source: &str, offset: u32) -> Option<TokenAtCursor> {
    let bytes = source.as_bytes();
    let offset = offset as usize;

    // find start of identifier (scan backwards)
    let mut start = offset;
    while start > 0 {
        let c = bytes[start - 1];
        if c.is_ascii_alphanumeric() || c == b'_' {
            start -= 1;
        } else {
            break;
        }
    }

    // find end of identifier (scan forwards)
    let mut end = offset;
    while end < bytes.len() {
        let c = bytes[end];
        if c.is_ascii_alphanumeric() || c == b'_' {
            end += 1;
        } else {
            break;
        }
    }

    if start < end {
        Some(TokenAtCursor {
            text: String::from_utf8_lossy(&bytes[start..end]).into_owned(),
            start: start as u32,
            end: end as u32,
        })
    } else {
        None
    }
}

/// Detect if the cursor is in a type position.
fn detect_type_position(source: &str, offset: u32) -> bool {
    let bytes = source.as_bytes();
    let mut pos = offset as usize;

    // skip whitespace backwards
    while pos > 0 && bytes[pos - 1].is_ascii_whitespace() {
        pos -= 1;
    }

    // check for type annotation indicators
    if pos > 0 {
        let c = bytes[pos - 1];
        // after `:` or `<` or `extends` or `implements`
        if c == b':' || c == b'<' || c == b',' {
            return true;
        }
    }

    // check for keyword patterns (extends, implements, is)
    let prefix = &source[..pos.min(source.len())];
    let trimmed = prefix.trim_end();
    if trimmed.ends_with("extends")
        || trimmed.ends_with("implements")
        || trimmed.ends_with("is")
        || trimmed.ends_with("as")
    {
        return true;
    }

    false
}

/// Detect import-related context.
fn detect_import_context(source: &str, offset: u32) -> Option<CompletionContext> {
    let bytes = source.as_bytes();

    // find the start of the current line
    let line_start = (0..offset as usize)
        .rev()
        .find(|&i| bytes[i] == b'\n')
        .map(|i| i + 1)
        .unwrap_or(0);

    let line_content = &source[line_start..offset as usize];

    // check if we're in an import statement
    if line_content.trim_start().starts_with("import") {
        // check if we're after "from" - import path context
        if line_content.contains("from") {
            return Some(CompletionContext::ImportPath);
        }

        // check if we're inside braces - import clause context
        let has_open_brace = line_content.contains('{');
        let has_close_brace = line_content.contains('}');
        if has_open_brace && !has_close_brace {
            // we're inside { ... } - check if there's a resolved module
            // for now, return ImportClause without target
            return Some(CompletionContext::ImportClause {
                target_module: None,
            });
        }
    }

    None
}

/// Find the scope at a given offset.
fn find_scope_at_offset(
    ast: &crate::ModuleAst,
    dir: &crate::ModuleDir,
    offset: u32,
) -> Option<dir::LocalScopeId> {
    // find enclosing AST nodes
    let enclosing = ast.tree.source_map.get_enclosing_spans(offset, offset);
    if enclosing.is_empty() {
        return None;
    }

    // sort by length (smallest first)
    let mut enclosing = enclosing;
    enclosing.sort_by_key(|e| e.length);

    let dir_tree = dir.tree.read();

    // find the innermost node - we need to get the scope for the node
    for enclosing in &enclosing {
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enclosing.idx) else {
            continue;
        };

        // for Expression nodes, use get_scope
        if dir_node_id.ty == dir::NodeType::Expression
            && let Ok(expr_id) = dir_node_id.try_into()
        {
            let (scope_id, _) = dir_tree.get_scope::<dir::Expression>(expr_id);
            return Some(scope_id);
        }
    }

    None
}
