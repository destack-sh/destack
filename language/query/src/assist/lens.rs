use destack_ast as ast;
use destack_dir::{self as dir, DeclarationForm, GlobalSymbolId};
use destack_source::{FileId, NodeSpanType, Span, Uri};
use destack_workspace::{Repository, Revision};
use serde::{Deserialize, Serialize};

use crate::ast::get_module_by_file_id;
use crate::core::{
    AstQueryContext, NominalRelation, modules_referencing_symbol, nominal_relations_for_target,
    query_context,
};
use crate::dir::{
    ReferenceCollectionOptions, collect_symbol_references_in_context, get_canonical_symbol,
    resolve_symbol_name,
};

/// A code lens (inline annotation with optional command).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeLens {
    /// The range this lens applies to.
    pub range: Span,
    /// The lens data.
    pub data: CodeLensData,
}

/// The data/command for a code lens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CodeLensData {
    /// Show reference count: "N references"
    References {
        /// Number of references (excluding declaration).
        count: usize,
    },
    /// Show implementation count: "N implementations"
    Implementations {
        /// Number of implementations.
        count: usize,
    },
    /// Run test action.
    RunTest {
        /// The test name.
        test_name: String,
    },
    /// Debug test action.
    DebugTest {
        /// The test name.
        test_name: String,
    },
    /// Custom lens with title and command.
    Custom {
        /// The title to display.
        title: String,
        /// The command identifier.
        command: String,
        /// Command arguments (JSON-serializable).
        arguments: Vec<String>,
    },
}

/// Request code lenses for a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeLensesRequest {
    /// The document URI.
    pub uri: Uri,
}

/// Request to resolve a code lens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolveCodeLensRequest {
    /// The code lens to resolve.
    pub lens: CodeLens,
}

/// Response payload for code lenses queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeLensesResponse {
    /// Code lenses.
    pub lenses: Vec<CodeLens>,
}

/// Response payload for code lens resolve queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolveCodeLensResponse {
    /// The resolved code lens.
    pub lens: CodeLens,
}

impl CodeLens {
    /// Create a references code lens.
    pub fn references(range: Span, count: usize) -> Self {
        Self {
            range,
            data: CodeLensData::References { count },
        }
    }

    /// Create an implementations code lens.
    pub fn implementations(range: Span, count: usize) -> Self {
        Self {
            range,
            data: CodeLensData::Implementations { count },
        }
    }

    /// Create a run test code lens.
    pub fn run_test(range: Span, test_name: impl Into<String>) -> Self {
        Self {
            range,
            data: CodeLensData::RunTest {
                test_name: test_name.into(),
            },
        }
    }

    /// Get the display title for this lens.
    pub fn title(&self) -> String {
        match &self.data {
            CodeLensData::References { count } => {
                if *count == 1 {
                    "1 reference".to_string()
                } else {
                    format!("{count} references")
                }
            }
            CodeLensData::Implementations { count } => {
                if *count == 1 {
                    "1 implementation".to_string()
                } else {
                    format!("{count} implementations")
                }
            }
            CodeLensData::RunTest { test_name } => format!("▶ Run {test_name}"),
            CodeLensData::DebugTest { test_name } => format!("🐛 Debug {test_name}"),
            CodeLensData::Custom { title, .. } => title.clone(),
        }
    }
}

/// Get code lenses for a file.
///
/// Code lenses appear as inline annotations above functions, classes, etc.
/// Common uses: reference counts, "Run Test" buttons, implementation counts.
pub fn code_lenses(repository: &Repository, revision: Revision, file: FileId) -> Vec<CodeLens> {
    let Some(module) = get_module_by_file_id(repository, revision, file) else {
        return Vec::new();
    };
    let Some(ctx) = query_context(repository, revision, module.id) else {
        return Vec::new();
    };
    let ast = ctx.ast();
    let module_id = ctx.module_id();
    let mut lenses = Vec::new();

    // collect declarations and their info
    let declarations: Vec<_> = {
        let dir_tree = ctx.dir().tree();
        let symbols = ctx.dir().symbols();

        dir_tree
            .iter_nodes_of_type::<dir::Declaration>()
            .map(
                |(decl_id, decl): (dir::LocalNodeId<dir::Declaration>, &dir::Declaration)| {
                    let symbol_id = decl.symbol();
                    let global_symbol_id = GlobalSymbolId {
                        module_id,
                        local_id: symbol_id,
                    };
                    let ast_node_id = dir_tree.get_source(decl_id.id);
                    let main_span = ctx
                        .ast()
                        .tree()
                        .get_side_span_by_id(ast_node_id, NodeSpanType::Main);
                    let name = resolve_symbol_name(repository, revision, global_symbol_id);
                    let is_test = has_decorator_named(ast, ast_node_id, "test");
                    let declaration_form = symbols.get_symbol(symbol_id).form;
                    (
                        decl.clone(),
                        global_symbol_id,
                        main_span,
                        is_test,
                        name,
                        declaration_form,
                    )
                },
            )
            .collect()
    };

    for (declaration, global_symbol_id, main_span, is_test, name, declaration_form) in declarations
    {
        let Some(span) = main_span else {
            continue;
        };

        // count references for functions/methods
        if matches!(declaration, dir::Declaration::Function { .. }) {
            let ref_count = count_references(repository, revision, global_symbol_id);
            if ref_count > 0 {
                lenses.push(CodeLens::references(span, ref_count));
            }

            // check if it's a test function
            if let Some(ref fn_name) = name
                && is_test
            {
                lenses.push(CodeLens::run_test(span, fn_name.clone()));
            }
        }

        // count implementations for interfaces
        if declaration_form == DeclarationForm::Interface {
            let impl_count = count_implementations(repository, revision, global_symbol_id);
            if impl_count > 0 {
                lenses.push(CodeLens::implementations(span, impl_count));
            }
        }

        // count subclasses for classes
        if declaration_form == DeclarationForm::Class {
            let subclass_count = count_subclasses(repository, revision, global_symbol_id);
            if subclass_count > 0 {
                lenses.push(CodeLens::implementations(span, subclass_count));
            }
        }
    }

    // sort lenses deterministically by range, kind, and title
    lenses.sort_by_cached_key(code_lens_key);

    // drop identical lenses after sorting
    lenses.dedup_by(|left, right| code_lens_key(left) == code_lens_key(right));

    lenses
}

/// Build a stable ordering key for a code lens.
fn code_lens_key(lens: &CodeLens) -> (u32, u32, u8, String) {
    let title = lens.title();
    (
        lens.range.start,
        lens.range.end,
        code_lens_kind_rank(&lens.data),
        title,
    )
}

/// Rank code lens kinds for stable ordering.
fn code_lens_kind_rank(data: &CodeLensData) -> u8 {
    match data {
        CodeLensData::References { .. } => 0,
        CodeLensData::Implementations { .. } => 1,
        CodeLensData::RunTest { .. } => 2,
        CodeLensData::DebugTest { .. } => 3,
        CodeLensData::Custom { .. } => 4,
    }
}

/// Count references to a symbol across all modules.
fn count_references(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
) -> usize {
    let canonical_id = get_canonical_symbol(repository, revision, symbol_id);
    let reference_name = resolve_symbol_name(repository, revision, canonical_id);

    let reference_options = ReferenceCollectionOptions {
        include_expressions: true,
        include_members: true,
        include_dependencies: true,
        include_namespace_receivers: true,
        skip_dependency_aliases: false,
        use_dependency_name_spans: true,
        target_name: reference_name.as_deref(),
        require_target_name_match: false,
        limit_to_file: None,
    };

    let mut count = 0;
    for module_id in modules_referencing_symbol(repository, revision, canonical_id) {
        let Some(ctx) = query_context(repository, revision, module_id) else {
            continue;
        };

        let spans = collect_symbol_references_in_context(
            repository,
            ctx.ast(),
            ctx.dir(),
            canonical_id,
            reference_options,
        );
        count += spans.len();
    }

    count
}

/// Count implementations of an interface across all modules.
fn count_implementations(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
) -> usize {
    let canonical_id = get_canonical_symbol(repository, revision, symbol_id);
    nominal_relations_for_target(repository, revision, canonical_id)
        .into_iter()
        .filter(|entry| entry.relation == NominalRelation::Implements)
        .count()
}

/// Count subclasses of a class across all modules.
fn count_subclasses(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
) -> usize {
    let canonical_id = get_canonical_symbol(repository, revision, symbol_id);
    nominal_relations_for_target(repository, revision, canonical_id)
        .into_iter()
        .filter(|entry| entry.relation == NominalRelation::Extends)
        .count()
}

/// Check whether a node has a decorator with the given name.
fn has_decorator_named(ast: AstQueryContext<'_>, node_id: u32, name: &str) -> bool {
    // scan annotations attached to the node
    if decorator_on_node(ast, node_id, name) {
        return true;
    }

    // fall back to enclosing nodes for annotations attached higher up
    let span = ast.source_map().get_main_or_enclosing(node_id);
    let mut enclosing = ast
        .source_map()
        .get_enclosing_spans(span.start, span.end.saturating_sub(1));
    enclosing.sort_by_key(|entry| entry.length);

    for entry in enclosing {
        if entry.idx == node_id {
            continue;
        }
        if decorator_on_node(ast, entry.idx, name) {
            return true;
        }
    }

    false
}

/// Check whether a decorator is attached directly to a node.
fn decorator_on_node(ast: AstQueryContext<'_>, node_id: u32, name: &str) -> bool {
    // scan decorators attached to the node
    let decorators = ast.tree().get_decorators(node_id);
    for decorator_id in decorators {
        let decorator = ast.tree().get::<ast::Decorator>(decorator_id);
        let Some(decorator_name_id) = decorator_name_id(ast, decorator) else {
            continue;
        };
        let decorator_name = ast.strings().get(decorator_name_id);
        if decorator_name == name {
            return true;
        }
    }

    false
}

/// Resolve the last segment of a decorator name when it is path-like.
fn decorator_name_id(
    ast: AstQueryContext<'_>,
    decorator: &ast::Decorator,
) -> Option<destack_core::StringId> {
    let mut expression_id = decorator.expression;
    loop {
        match ast.tree().get(expression_id) {
            ast::Expression::Parenthesized { expression } => {
                expression_id = *expression;
            }
            ast::Expression::Call { left, .. } => {
                expression_id = *left;
            }
            ast::Expression::QualifiedReference { path, .. } => {
                return path.segments.last().copied();
            }
            ast::Expression::Identifier { name } => return Some(*name),
            _ => return None,
        }
    }
}

/// Resolve a code lens (compute its command if deferred).
///
/// Some lenses defer computation until the user hovers/clicks.
pub fn resolve_code_lens(lens: &CodeLens) -> CodeLens {
    // lenses are resolved eagerly
    lens.clone()
}
