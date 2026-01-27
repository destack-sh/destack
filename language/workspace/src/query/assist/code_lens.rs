use destack_dir::{self as dir, Expression, GlobalSymbolId, SymbolType};
use destack_source::{FileId, NodeSpanType, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::Session;
use crate::query::common::{get_canonical_symbol, get_module_by_file_id};

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
pub fn code_lenses(session: &Session, file: FileId) -> Vec<CodeLens> {
    let Some(module) = get_module_by_file_id(session, file) else {
        return Vec::new();
    };
    let module = module.read();
    let Some(ctx) = session.query_context(&module) else {
        return Vec::new();
    };
    let module_id = ctx.module_id;
    let dir_tree = ctx.tree();
    let symbols = ctx.symbols();

    let mut lenses = Vec::new();

    // collect declarations and their info
    let declarations: Vec<_> = dir_tree
        .iter_nodes_of_type::<dir::Declaration>()
        .map(
            |(decl_id, decl): (dir::LocalNodeId<dir::Declaration>, &dir::Declaration)| {
                let symbol_id = decl.symbol();
                let ast_node_id = dir_tree.get_source(decl_id.id);
                let main_span = ctx
                    .ast
                    .tree
                    .get_side_span_by_id(ast_node_id, NodeSpanType::Main);
                let name = symbols
                    .get_symbol(symbol_id)
                    .name()
                    .map(|id| ctx.ast.strings.get(id).to_string());
                let symbol_type = symbols.get_symbol(symbol_id).ty;
                (
                    decl.clone(),
                    GlobalSymbolId {
                        module_id,
                        local_id: symbol_id,
                    },
                    main_span,
                    name,
                    symbol_type,
                )
            },
        )
        .collect();

    drop(symbols);
    drop(dir_tree);
    drop(module);

    for (declaration, global_symbol_id, main_span, name, symbol_type) in declarations {
        let Some(span) = main_span else {
            continue;
        };

        // count references for functions/methods
        if matches!(declaration, dir::Declaration::Function { .. }) {
            let ref_count = count_references(session, global_symbol_id);
            if ref_count > 0 {
                lenses.push(CodeLens::references(span, ref_count));
            }

            // check if it's a test function
            if let Some(ref fn_name) = name
                && is_test_function(fn_name)
            {
                lenses.push(CodeLens::run_test(span, fn_name.clone()));
            }
        }

        // count implementations for interfaces
        if symbol_type == SymbolType::Interface {
            let impl_count = count_implementations(session, global_symbol_id);
            if impl_count > 0 {
                lenses.push(CodeLens::implementations(span, impl_count));
            }
        }

        // count subclasses for classes
        if symbol_type == SymbolType::Class {
            let subclass_count = count_subclasses(session, global_symbol_id);
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
fn count_references(session: &Session, symbol_id: GlobalSymbolId) -> usize {
    let canonical_id = get_canonical_symbol(session, symbol_id);
    let mut count = 0;

    for module in session.modules.iter() {
        let module = module.read();
        let Some(ctx) = session.query_context(&module) else {
            continue;
        };
        let dir_tree = ctx.tree();

        for (_, expr) in dir_tree.iter_nodes_of_type::<Expression>() {
            if let Some(target) = expr.target_symbol() {
                let target_canonical = get_canonical_symbol(session, target);
                if target_canonical == canonical_id {
                    count += 1;
                }
            }
        }
    }

    count
}

/// Count implementations of an interface across all modules.
fn count_implementations(session: &Session, symbol_id: GlobalSymbolId) -> usize {
    let canonical_id = get_canonical_symbol(session, symbol_id);
    let mut count = 0;

    for module in session.modules.iter() {
        let module = module.read();
        let Some(ctx) = session.query_context(&module) else {
            continue;
        };
        let types = ctx.types();

        for (_, lineage) in types.iter_lineages() {
            if lineage.directly_implements(canonical_id) {
                count += 1;
            }
        }
    }

    count
}

/// Count subclasses of a class across all modules.
fn count_subclasses(session: &Session, symbol_id: GlobalSymbolId) -> usize {
    let canonical_id = get_canonical_symbol(session, symbol_id);
    let mut count = 0;

    for module in session.modules.iter() {
        let module = module.read();
        let Some(ctx) = session.query_context(&module) else {
            continue;
        };
        let types = ctx.types();

        for (_, lineage) in types.iter_lineages() {
            if lineage.directly_extends(canonical_id) {
                count += 1;
            }
        }
    }

    count
}

/// Check if a function name indicates it's a test.
fn is_test_function(name: &str) -> bool {
    name.starts_with("test_") || name.starts_with("test") || name == "test"
}

/// Resolve a code lens (compute its command if deferred).
///
/// Some lenses defer computation until the user hovers/clicks.
pub fn resolve_code_lens(_session: &Session, lens: &CodeLens) -> CodeLens {
    // lenses are resolved eagerly for now, keep this hook for deferred work
    lens.clone()
}
