use destack_dir::{self as dir, Expression, GlobalSymbolId, SymbolType};
use destack_source::{FileId, NodeSpanType, Span};

use crate::Session;
use crate::query::common::{get_canonical_symbol, get_module_by_file_id};

/// A code lens (inline annotation with optional command).
#[derive(Debug, Clone)]
pub struct CodeLens {
    /// The range this lens applies to.
    pub range: Span,
    /// The lens data.
    pub data: CodeLensData,
}

/// The data/command for a code lens.
#[derive(Debug, Clone)]
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

    let mut lenses = Vec::new();
    let module_guard = module.read();
    let module_id = module_guard.id;
    let dir_tree = module_guard.dir.tree.read();
    let symbols = module_guard.dir.symbols.read();

    // collect declarations and their info
    let declarations: Vec<_> = dir_tree
        .iter_nodes_of_type::<dir::Declaration>()
        .map(|(decl_id, decl)| {
            let symbol_id = decl.symbol();
            let ast_node_id = dir_tree.get_source(decl_id.id);
            let main_span = module_guard
                .ast
                .tree
                .get_side_span_by_id(ast_node_id, NodeSpanType::Main);
            let name = symbols
                .get_symbol(symbol_id)
                .name()
                .map(|id| module_guard.ast.strings.get(id).to_string());
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
        })
        .collect();

    drop(symbols);
    drop(dir_tree);
    drop(module_guard);

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

    // sort lenses by position
    lenses.sort_by_key(|l| l.range.start);
    lenses
}

/// Count references to a symbol across all modules.
fn count_references(session: &Session, symbol_id: GlobalSymbolId) -> usize {
    let canonical_id = get_canonical_symbol(session, symbol_id);
    let mut count = 0;

    for module in session.modules.iter() {
        let module_guard = module.read();
        let dir_tree = module_guard.dir.tree.read();

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
        let module_guard = module.read();
        let types = module_guard.dir.types.read();

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
        let module_guard = module.read();
        let types = module_guard.dir.types.read();

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
    // For now, lenses are fully resolved on creation
    // This hook exists for expensive computations that should be deferred
    lens.clone()
}
