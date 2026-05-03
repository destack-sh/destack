use crate::{Compiler, CompilerResult, DiagnosticAnchor, ImportError};
use destack_core::{StringId, StringPool};
use destack_dir::{self as dir, Expression, SymbolTable, Tree};
use destack_workspace::{Module, ProviderContext};

/// State for validating one imported module.
pub(in crate::import) struct ImportState<'a> {
    /// The compiler driving validation.
    pub(in crate::import) compiler: &'a Compiler,
    /// The provider attempt that receives diagnostics.
    pub(in crate::import) context: &'a dyn ProviderContext,
    /// The module being validated.
    pub(in crate::import) module: &'a Module,
    /// The DIR tree being validated.
    pub(in crate::import) tree: &'a Tree,
    /// The strings referenced by the DIR being validated.
    pub(in crate::import) strings: &'a StringPool,
    /// The symbol table being validated.
    pub(in crate::import) symbols: &'a SymbolTable,
    /// The module roots being validated.
    pub(in crate::import) roots: &'a [dir::LocalNodeId<Expression>],
    /// The global augmentation scope for local conflict checks.
    pub(in crate::import) global_augmentation_scope: dir::LocalScopeId,
}

impl<'a> ImportState<'a> {
    /// Create import validation state.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::import) fn new(
        compiler: &'a Compiler,
        context: &'a dyn ProviderContext,
        module: &'a Module,
        tree: &'a Tree,
        strings: &'a StringPool,
        symbols: &'a SymbolTable,
        roots: &'a [dir::LocalNodeId<Expression>],
        global_augmentation_scope: dir::LocalScopeId,
    ) -> Self {
        Self {
            compiler,
            context,
            module,
            tree,
            strings,
            symbols,
            roots,
            global_augmentation_scope,
        }
    }

    /// Return the source anchor for one DIR node built from the current AST.
    pub(in crate::import) fn anchor(&self, node: dir::GlobalNodeIdAny) -> DiagnosticAnchor {
        assert_eq!(
            self.module.id, node.module_id,
            "import diagnostic node must belong to the module being validated"
        );

        let span = self
            .tree
            .get_span_by_id(node.local_id.id)
            .expect("import diagnostic node is missing a source span");

        DiagnosticAnchor::Span(span)
    }

    /// Return the source text for one interned string.
    pub(in crate::import) fn string(&self, string: StringId) -> String {
        self.strings.get(string).to_string()
    }

    /// Return diagnostic text for one static key.
    pub(in crate::import) fn static_key(&self, key: dir::StaticKey) -> String {
        key.debug_string(self.strings)
    }

    /// Emit one import diagnostic.
    pub(in crate::import) fn emit(&self, error: ImportError) -> CompilerResult<()> {
        self.compiler.emit_diagnostic(self.context, error)
    }
}
