use destack_dir as dir;
use destack_source::{FileId, Span};
use serde::{Deserialize, Serialize};

use crate::common::{
    QueryContext, SymbolKind, container_name_for_node, declaration_display_name,
    declaration_symbol_kind, is_synthetic_function_keyword_field, member_key_name,
    member_symbol_kind, score_completion, try_span_for_dir_node,
};
use destack_workspace::{ModuleSource, Session};

/// A symbol in the workspace (flat list for workspace symbol search).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceSymbol {
    /// The symbol's name.
    pub name: String,
    /// The kind of symbol.
    pub kind: SymbolKind,
    /// The file containing the symbol.
    pub file: FileId,
    /// The location of the symbol.
    pub range: Span,
    /// Container name (e.g., class name for methods).
    pub container: Option<String>,
}

/// Request workspace symbols for a query string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceSymbolsRequest {
    /// The search query string.
    pub query: String,
    /// The maximum number of results.
    pub max_results: u32,
}

/// Response payload for workspace symbols queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceSymbolsResponse {
    /// Workspace symbols.
    pub symbols: Vec<WorkspaceSymbol>,
}

/// Search for symbols across the workspace.
///
/// Returns symbols whose names contain the query string (case insensitive).
pub fn workspace_symbols(
    session: &Session,
    query: &str,
    max_results: usize,
) -> Vec<WorkspaceSymbol> {
    // prepare the scored symbol buffer
    let mut scored_symbols: Vec<(u32, WorkspaceSymbol)> = Vec::new();

    // normalize query input
    let query = query.trim();

    // search all modules
    for module in session.modules.iter() {
        let module = module.as_ref();

        // skip builtin modules
        if module.source != ModuleSource::User {
            continue;
        }

        // resolve query context for the module
        let Some(ctx) = crate::query_context(session, module) else {
            continue;
        };

        let dir_tree = ctx.tree();

        // iterate through all declarations
        for (declaration_id, declaration) in dir_tree.iter_nodes_of_type::<dir::Declaration>() {
            // get the declaration name
            let name = declaration_display_name(&session.strings, declaration);

            // get the declaration kind
            let kind = declaration_symbol_kind(declaration);

            // resolve container by walking up the parent chain
            let container = container_name_for_node(dir_tree, &session.strings, declaration_id.id);

            // resolve the declaration span
            let Some(range) = workspace_symbol_range(&ctx, dir_tree, declaration_id.id) else {
                continue;
            };

            // score declaration match
            if let Some(score) = score_workspace_symbol(&name, container.as_deref(), query) {
                scored_symbols.push((
                    score,
                    WorkspaceSymbol {
                        name: name.clone(),
                        kind,
                        file: ctx.file_id,
                        range,
                        container: container.clone(),
                    },
                ));
            }

            // collect member symbols with the declaration as container
            if let Some(member_ids) = declaration.member_ids() {
                for member_id in member_ids {
                    let Some(member_symbol) =
                        member_to_workspace_symbol(session, &ctx, dir_tree, *member_id, &name)
                    else {
                        continue;
                    };

                    if let Some(score) =
                        score_workspace_symbol(&member_symbol.name, Some(&name), query)
                    {
                        scored_symbols.push((score, member_symbol));
                    }
                }
            }

            // collect enum field symbols with the enum as container
            if let dir::Declaration::Enum { fields, .. } = declaration {
                for field_id in fields {
                    let Some(field_symbol) =
                        enum_field_to_workspace_symbol(session, &ctx, dir_tree, *field_id, &name)
                    else {
                        continue;
                    };

                    if let Some(score) =
                        score_workspace_symbol(&field_symbol.name, Some(&name), query)
                    {
                        scored_symbols.push((score, field_symbol));
                    }
                }
            }
        }
    }

    // sort by score descending, then name and location for deterministic results
    scored_symbols.sort_by(|left, right| {
        let left_key = (
            std::cmp::Reverse(left.0),
            left.1.name.len(),
            left.1.name.to_lowercase(),
            left.1.file.0,
            left.1.range.start,
            left.1.range.end,
        );
        let right_key = (
            std::cmp::Reverse(right.0),
            right.1.name.len(),
            right.1.name.to_lowercase(),
            right.1.file.0,
            right.1.range.start,
            right.1.range.end,
        );
        left_key.cmp(&right_key)
    });

    // drop duplicate symbol locations
    let mut symbols = Vec::new();
    for (_, symbol) in scored_symbols {
        let is_duplicate = symbols.iter().any(|existing: &WorkspaceSymbol| {
            existing.name == symbol.name
                && existing.kind == symbol.kind
                && existing.file == symbol.file
                && existing.range.start == symbol.range.start
                && existing.range.end == symbol.range.end
        });
        if !is_duplicate {
            symbols.push(symbol);
        }
    }

    // enforce the maximum result limit
    if symbols.len() > max_results {
        symbols.truncate(max_results);
    }

    // return the final symbol list
    symbols
}

/// Score a workspace symbol against the query.
fn score_workspace_symbol(name: &str, _container: Option<&str>, query: &str) -> Option<u32> {
    // match the symbol name against the query
    score_completion(name, query).map(|matched| matched.score)
}

/// Convert a member to a workspace symbol.
fn member_to_workspace_symbol(
    session: &Session,
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    member_id: dir::LocalNodeId<dir::Member>,
    container_name: &str,
) -> Option<WorkspaceSymbol> {
    // resolve the member node
    let member = dir_tree.get::<dir::Member>(member_id);

    // resolve the member name from its key
    let key = member.key()?;
    let name = member_key_name(session, key)?;

    // resolve the member kind
    let kind = member_symbol_kind(member)?;

    // resolve the member span
    let range = workspace_symbol_range(ctx, dir_tree, member_id.id)?;

    // skip synthetic function keyword fields for methods
    if is_synthetic_function_keyword_field(member, &name, range) {
        return None;
    }

    Some(WorkspaceSymbol {
        name,
        kind,
        file: ctx.file_id,
        range,
        container: Some(container_name.to_string()),
    })
}

/// Convert an enum field to a workspace symbol.
fn enum_field_to_workspace_symbol(
    session: &Session,
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    field_id: dir::LocalNodeId<dir::EnumField>,
    container_name: &str,
) -> Option<WorkspaceSymbol> {
    // resolve the enum field node
    let field = dir_tree.get::<dir::EnumField>(field_id);

    // resolve the field name
    let name = session.strings.get(field.name).to_string();

    // resolve the field span
    let range = workspace_symbol_range(ctx, dir_tree, field_id.id)?;

    // return the enum field symbol
    Some(WorkspaceSymbol {
        name,
        kind: SymbolKind::EnumMember,
        file: ctx.file_id,
        range,
        container: Some(container_name.to_string()),
    })
}

/// Resolve one workspace symbol range without failing the whole query on bad source ids.
fn workspace_symbol_range(
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    node_id: u32,
) -> Option<Span> {
    let node_id = dir::LocalNodeIdAny::new(node_id, dir_tree.get_node_type(node_id));
    try_span_for_dir_node(ctx, dir_tree, node_id)
}
