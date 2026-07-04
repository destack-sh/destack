use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::{NodeSpanType, Span};
use serde::{Deserialize, Serialize};

use crate::{Module, ModuleQueryContext, ProgramQueryContext, ReferenceFilter};

/// A code lens (inline annotation with optional command).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CodeLens {
    /// The range this lens applies to.
    pub range: Span,
    /// The lens action.
    pub action: CodeLensAction,
}

/// The action for a code lens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum CodeLensAction {
    /// Show reference count.
    References {
        /// Number of references (excluding declaration).
        count: usize,
    },
    /// Show implementation count.
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CodeLensesRequest {
    /// The queried module.
    pub module: Module,
}

/// Request to resolve a code lens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ResolveCodeLensRequest {
    /// The code lens to resolve.
    pub lens: CodeLens,
}

/// Response payload for code lenses queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CodeLensesResponse {
    /// Code lenses.
    pub lenses: Vec<CodeLens>,
}

/// Response payload for code lens resolve queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ResolveCodeLensResponse {
    /// The resolved code lens.
    pub lens: CodeLens,
}

impl CodeLens {
    /// Create a references code lens.
    pub fn references(range: Span, count: usize) -> Self {
        Self {
            range,
            action: CodeLensAction::References { count },
        }
    }

    /// Create an implementations code lens.
    pub fn implementations(range: Span, count: usize) -> Self {
        Self {
            range,
            action: CodeLensAction::Implementations { count },
        }
    }

    /// Create a run test code lens.
    pub fn run_test(range: Span, test_name: impl Into<String>) -> Self {
        Self {
            range,
            action: CodeLensAction::RunTest {
                test_name: test_name.into(),
            },
        }
    }

    /// Resolve this code lens.
    pub fn resolve(&self) -> Self {
        self.clone()
    }

    /// Get the display title for this lens.
    pub fn title(&self) -> String {
        match &self.action {
            CodeLensAction::References { count } => {
                if *count == 1 {
                    "1 reference".to_string()
                } else {
                    format!("{count} references")
                }
            }
            CodeLensAction::Implementations { count } => {
                if *count == 1 {
                    "1 implementation".to_string()
                } else {
                    format!("{count} implementations")
                }
            }
            CodeLensAction::RunTest { test_name } => format!("Run {test_name}"),
            CodeLensAction::DebugTest { test_name } => format!("Debug {test_name}"),
            CodeLensAction::Custom { title, .. } => title.clone(),
        }
    }

    /// Return the stable protocol ordering for this lens.
    fn order(&self) -> CodeLensOrder {
        CodeLensOrder {
            start: self.range.start,
            end: self.range.end,
            action: self.action.kind(),
            title: self.title(),
        }
    }
}

impl CodeLensAction {
    /// Return the stable action family for protocol ordering.
    fn kind(&self) -> CodeLensActionKind {
        match self {
            CodeLensAction::References { .. } => CodeLensActionKind::References,
            CodeLensAction::Implementations { .. } => CodeLensActionKind::Implementations,
            CodeLensAction::RunTest { .. } => CodeLensActionKind::RunTest,
            CodeLensAction::DebugTest { .. } => CodeLensActionKind::DebugTest,
            CodeLensAction::Custom { .. } => CodeLensActionKind::Custom,
        }
    }
}

/// Stable protocol ordering for one code lens.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CodeLensOrder {
    /// The lens start offset.
    start: u32,
    /// The lens end offset.
    end: u32,
    /// The action family.
    action: CodeLensActionKind,
    /// The rendered title.
    title: String,
}

/// Stable code lens action family order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CodeLensActionKind {
    /// Reference count lenses.
    References,
    /// Implementation count lenses.
    Implementations,
    /// Run test lenses.
    RunTest,
    /// Debug test lenses.
    DebugTest,
    /// Custom command lenses.
    Custom,
}

/// Declaration facts needed to emit code lenses.
#[derive(Debug, Clone, PartialEq)]
struct CodeLensDeclaration {
    /// The declaration symbol.
    symbol_id: dir::GlobalSymbolId,
    /// The declaration name.
    name: Option<String>,
    /// The declaration kind.
    symbol_kind: dir::SymbolKind,
    /// The declaration main span.
    span: Span,
    /// Whether this is a function declaration.
    is_function: bool,
    /// Whether this declaration is marked as a test.
    is_test: bool,
}

impl CodeLensDeclaration {
    /// Build declaration facts for code lens emission.
    fn new(
        module: &ModuleQueryContext<'_>,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) -> Option<Self> {
        let view = module.view();
        let symbols = module.symbols();
        let local_symbol_id = module.node_symbol(declaration_id.into())?;
        let symbol_id = dir::GlobalSymbolId::new(module.module_id(), local_symbol_id);
        let source_node_id = view.get_source(declaration_id);
        let span = module
            .tree()
            .get_side_span_by_id(source_node_id, NodeSpanType::Main)?;

        Some(Self {
            symbol_id,
            name: module.symbol_name(symbol_id),
            symbol_kind: symbols.get_symbol(local_symbol_id).kind,
            span,
            is_function: matches!(declaration, dir::Declaration::Function { .. }),
            is_test: module.has_decorator_named(source_node_id, "test"),
        })
    }

    /// Collect code lenses for this declaration.
    fn collect_lenses(
        &self,
        module: &ModuleQueryContext<'_>,
        program: &ProgramQueryContext<'_>,
        lenses: &mut Vec<CodeLens>,
    ) {
        // collect function lenses
        if self.is_function {
            let reference_count = module.count_references(program, self.symbol_id);
            if reference_count > 0 {
                lenses.push(CodeLens::references(self.span, reference_count));
            }

            if self.is_test {
                if let Some(function_name) = &self.name {
                    lenses.push(CodeLens::run_test(self.span, function_name.clone()));
                }
            }
        }

        // collect interface implementation lenses
        if self.symbol_kind == dir::SymbolKind::Interface {
            let implementation_count = module.count_implementations(program, self.symbol_id);
            if implementation_count > 0 {
                lenses.push(CodeLens::implementations(self.span, implementation_count));
            }
        }

        // collect class subclass lenses
        if self.symbol_kind == dir::SymbolKind::Class {
            let subclass_count = module.count_subclasses(program, self.symbol_id);
            if subclass_count > 0 {
                lenses.push(CodeLens::implementations(self.span, subclass_count));
            }
        }
    }
}

/// Resolve a code lens (compute its command if deferred).
///
/// Some lenses defer computation until the user hovers/clicks.
pub fn resolve_code_lens(lens: &CodeLens) -> CodeLens {
    lens.resolve()
}

impl ModuleQueryContext<'_> {
    /// Check whether a node has a decorator with the given name.
    fn has_decorator_named(&self, node_id: u32, name: &str) -> bool {
        // scan decorations attached to the node
        if self.decorator_on_node(node_id, name) {
            return true;
        }

        let span = self
            .source_index()
            .get_main(node_id)
            .unwrap_or_else(|| panic!("missing main source span for decorator target {node_id}"));
        let mut enclosing = self.source_index().get_enclosing_spans(
            self.file_id(),
            span.start,
            span.end.saturating_sub(1),
        );
        enclosing.sort_by_key(|entry| entry.length);

        // use enclosing nodes for decorations attached higher up
        for entry in enclosing {
            if entry.source_id == node_id {
                continue;
            }
            if self.decorator_on_node(entry.source_id, name) {
                return true;
            }
        }

        false
    }

    /// Check whether a decorator is attached directly to a node.
    fn decorator_on_node(&self, node_id: u32, name: &str) -> bool {
        let decorators = self.tree().get_decorators(node_id);

        // scan decorators attached to the node
        for decorator_id in decorators {
            let decorator = self.tree().get::<dir::Decorator>(decorator_id);
            let Some(decorator_name_id) = self.decorator_leaf_name_id(decorator) else {
                continue;
            };

            let decorator_name = self.strings().get(decorator_name_id);
            if decorator_name == name {
                return true;
            }
        }

        false
    }
}

impl ModuleQueryContext<'_> {
    /// Return code lenses for a file.
    ///
    /// Code lenses appear as inline decorations above functions, classes, etc.
    /// Common uses: reference counts, "Run Test" buttons, implementation counts.
    pub fn code_lenses(&self, program: &ProgramQueryContext<'_>) -> Vec<CodeLens> {
        let mut lenses = Vec::new();

        // collect declaration facts
        let declarations: Vec<_> = {
            let view = self.view();

            view.iter_nodes_of_type::<dir::Declaration>()
                .into_iter()
                .filter_map(
                    |(declaration_id, declaration): (
                        dir::LocalNodeId<dir::Declaration>,
                        &dir::Declaration,
                    )| {
                        CodeLensDeclaration::new(self, declaration_id, declaration)
                    },
                )
                .collect()
        };

        for declaration in declarations {
            declaration.collect_lenses(self, program, &mut lenses);
        }

        // sort lenses deterministically by range, kind, and title
        lenses.sort_by_cached_key(CodeLens::order);

        // drop identical lenses after sorting
        lenses.dedup_by(|left, right| left.order() == right.order());

        lenses
    }

    /// Count references to a symbol across all modules.
    fn count_references(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> usize {
        let canonical_id = self.canonical_symbol(symbol_id);
        let reference_name = self.symbol_name(canonical_id);

        let reference_search = ReferenceFilter {
            include_expressions: true,
            include_members: true,
            include_dependency_items: true,
            include_namespace_receivers: true,
            skip_dependency_aliases: false,
            target_name: reference_name.as_deref(),
            requires_target_name_match: false,
            limit_file: None,
        };

        self.program_symbol_references(program, canonical_id, reference_search)
            .len()
    }

    /// Count implementations of an interface across all modules.
    fn count_implementations(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> usize {
        let canonical_id = self.canonical_symbol(symbol_id);

        program
            .base_heritage(canonical_id)
            .into_iter()
            .filter(|entry| entry.kind == dir::HeritageKind::Implements)
            .count()
    }

    /// Count subclasses of a class across all modules.
    fn count_subclasses(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> usize {
        let canonical_id = self.canonical_symbol(symbol_id);

        program
            .base_heritage(canonical_id)
            .into_iter()
            .filter(|entry| entry.kind == dir::HeritageKind::Extends)
            .count()
    }
}
