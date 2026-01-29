use std::collections::HashMap;
use std::str::FromStr;

use destack_ast::Keyword;
use destack_dir as dir;
use destack_source::{BatchEdit, Edit, FileEdit, FileId, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::Session;
use crate::query::common::{
    QueryContext, ReferenceCollectionOptions, SymbolAtOffset, collect_symbol_references_in_context,
    find_symbol_at_offset, get_canonical_symbol, get_symbol_definition_span, is_simple_identifier,
    member_key_name, resolve_symbol_name, sort_and_dedup_spans, token_at_offset,
};

/// Result of a prepare rename query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepareRenameResult {
    /// The range of the symbol to rename.
    pub range: Span,
    /// The current name (placeholder for rename dialog).
    pub placeholder: String,
}

/// Result of a rename query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenameResult {
    /// All edits to apply.
    pub edits: BatchEdit,
}

impl RenameResult {
    /// Create an empty rename result.
    pub fn empty() -> Self {
        Self {
            edits: BatchEdit::new(),
        }
    }

    /// Create a rename result from a batch edit.
    pub fn from_edits(edits: BatchEdit) -> Self {
        Self { edits }
    }

    /// Whether there are any edits.
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// Total number of edits.
    pub fn edit_count(&self) -> usize {
        self.edits.total_edits()
    }

    /// Number of files affected.
    pub fn file_count(&self) -> usize {
        self.edits.file_count()
    }
}

/// Request prepare rename at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepareRenameRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
}

/// Response payload for prepare rename queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepareRenameResponse {
    /// Prepare rename result, if available.
    pub result: Option<PrepareRenameResult>,
}

/// Request rename edits at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenameRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
    /// The new name for the symbol.
    pub new_name: String,
}

/// Response payload for rename queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenameResponse {
    /// Rename result, if available.
    pub result: Option<RenameResult>,
}

/// Check if the symbol at the given position can be renamed.
///
/// Returns the range and current name if renameable.
pub fn prepare_rename(session: &Session, file: FileId, offset: u32) -> Option<PrepareRenameResult> {
    // find the symbol at offset
    let symbol_at = find_symbol_at_offset(session, file, offset)?;

    // reject non modifier keywords at the cursor
    let token = token_at_offset(session, file, offset);
    if token.is_some_and(|token| is_keyword(&token) && !is_modifier_keyword(&token)) {
        return None;
    }

    // get canonical symbol
    let canonical_id = get_canonical_symbol(session, symbol_at.symbol_id);

    // resolve the rename placeholder name
    let name = resolve_rename_name(session, canonical_id, &symbol_at)?;

    // return the range and current name
    Some(PrepareRenameResult {
        range: symbol_at.span,
        placeholder: name,
    })
}

/// Rename the symbol at the given position.
///
/// Returns edits for all files that need to be modified.
pub fn rename(
    session: &Session,
    file: FileId,
    offset: u32,
    new_name: &str,
) -> Option<RenameResult> {
    // find the symbol at offset
    let symbol_at = find_symbol_at_offset(session, file, offset)?;

    // reject non modifier keywords at the cursor
    let token = token_at_offset(session, file, offset);
    if token.is_some_and(|token| is_keyword(&token) && !is_modifier_keyword(&token)) {
        return None;
    }

    // validate new_name is a valid identifier
    if !is_simple_identifier(new_name) {
        return None;
    }

    // get canonical symbol
    let canonical_id = get_canonical_symbol(session, symbol_at.symbol_id);

    // resolve the existing symbol name for span targeting
    let old_name = resolve_rename_name(session, canonical_id, &symbol_at)?;

    // collect all spans to rename, grouped by file
    let mut edits_by_file: HashMap<FileId, Vec<Span>> = HashMap::new();

    // add the definition
    if let Some(definition_span) = get_symbol_definition_span(session, canonical_id) {
        edits_by_file
            .entry(definition_span.file)
            .or_default()
            .push(definition_span);
    }

    // configure reference collection for rename behavior
    let reference_options = ReferenceCollectionOptions {
        include_expressions: true,
        include_members: true,
        include_dependencies: true,
        include_namespace_members: true,
        skip_dependency_aliases: true,
        use_dependency_name_spans: true,
        target_name: Some(&old_name),
        limit_to_file: None,
    };

    // collect references across all modules
    for module in session.modules.iter() {
        let module = module.read();
        let Some(ctx) = session.query_context(&module) else {
            continue;
        };

        // collect spans for this module and group them by file
        let spans =
            collect_symbol_references_in_context(session, &ctx, canonical_id, reference_options);
        for span in spans {
            edits_by_file.entry(span.file).or_default().push(span);
        }
    }

    // normalize span ordering and remove duplicates per file
    for spans in edits_by_file.values_mut() {
        sort_and_dedup_spans(spans);
    }

    // create BatchEdit from collected spans
    let mut batch_edit = BatchEdit::new();
    for (file_id, spans) in edits_by_file {
        let edits: Vec<Edit> = spans
            .into_iter()
            .map(|span| Edit::replace(span, new_name.to_string()))
            .collect();
        batch_edit.push(FileEdit::with_edits(file_id, edits));
    }

    Some(RenameResult::from_edits(batch_edit))
}

fn resolve_rename_name(
    session: &Session,
    canonical_id: dir::GlobalSymbolId,
    symbol_at: &SymbolAtOffset,
) -> Option<String> {
    // prefer the canonical symbol name when available
    if let Some(name) = resolve_symbol_name(session, canonical_id) {
        return Some(name);
    }

    // fall back to the local node name in the current module
    let module = session.modules.get(symbol_at.symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;
    resolve_name_from_node(session, &ctx, symbol_at.node_id)
}

fn resolve_name_from_node(
    session: &Session,
    ctx: &QueryContext<'_>,
    node_id: dir::LocalNodeIdAny,
) -> Option<String> {
    // resolve the dir tree for node lookup
    let dir_tree = ctx.tree();

    match node_id.ty {
        dir::NodeType::Expression => {
            let Ok(expr_id) = node_id.try_into() else {
                return None;
            };
            let expr = dir_tree.get::<dir::Expression>(expr_id);
            match expr {
                dir::Expression::Member { name, .. } => {
                    Some(session.strings.get(*name).to_string())
                }
                _ => None,
            }
        }
        dir::NodeType::Member => {
            let Ok(member_id) = node_id.try_into() else {
                return None;
            };
            let member = dir_tree.get::<dir::Member>(member_id);
            let key = member.key()?;
            member_key_name(session, key)
        }
        dir::NodeType::EnumField => {
            let Ok(field_id) = node_id.try_into() else {
                return None;
            };
            let field = dir_tree.get::<dir::EnumField>(field_id);
            Some(session.strings.get(field.name).to_string())
        }
        dir::NodeType::Parameter => {
            let Ok(param_id) = node_id.try_into() else {
                return None;
            };
            let param = dir_tree.get::<dir::Parameter>(param_id);
            match param {
                dir::Parameter::Named { name, .. } => Some(session.strings.get(*name).to_string()),
                dir::Parameter::Variadic { name, .. } => {
                    Some(session.strings.get(*name).to_string())
                }
                dir::Parameter::Pattern { .. } => None,
            }
        }
        dir::NodeType::Declaration => {
            let Ok(decl_id) = node_id.try_into() else {
                return None;
            };
            let declaration = dir_tree.get::<dir::Declaration>(decl_id);
            let descriptor = declaration.descriptor();
            descriptor
                .name
                .map(|name| session.strings.get(name.string()).to_string())
        }
        _ => None,
    }
}

/// Check whether a keyword is a declaration modifier.
fn is_modifier_keyword(token: &str) -> bool {
    let Ok(keyword) = Keyword::from_str(token) else {
        return false;
    };

    matches!(
        keyword,
        Keyword::Export
            | Keyword::Declare
            | Keyword::Abstract
            | Keyword::Async
            | Keyword::Static
            | Keyword::Public
            | Keyword::Protected
            | Keyword::Private
            | Keyword::Readonly
            | Keyword::Mut
            | Keyword::Final
            | Keyword::Accessor
            | Keyword::Default
            | Keyword::Override
    )
}

/// Check whether a token is a keyword.
fn is_keyword(token: &str) -> bool {
    Keyword::from_str(token).is_ok()
}
