use destack_artifact::{DirBound, DirParsed};
use destack_core::StringPool;
use destack_dir as dir;
use destack_js as js;
use destack_repository::{Module, Target};

use crate::generate::js::{CodegenJsError, CodegenJsResult, CodegenJsWarning, ScriptSymbolId};

/// Context for lowering a DIR module to JS AST.
#[derive(Debug)]
#[allow(dead_code)]
pub struct ModuleLowerer<'a> {
    /// The source module.
    pub(crate) module: &'a Module,
    /// The parsed DIR artifact.
    pub(crate) parsed: &'a DirParsed,
    /// The source string pool for bound DIR nodes.
    pub(crate) source_strings: &'a StringPool,

    /// The DIR roots.
    pub(crate) dir_roots: &'a Vec<dir::LocalNodeId<dir::Expression>>,
    /// The DIR tree.
    pub(crate) dir_tree: &'a dir::Tree,
    /// The symbol table.
    pub(crate) symbols: dir::BindingTable<'static>,
    /// The type table.
    pub(crate) types: &'a dir::TypeTable<'static>,
    /// The static table.
    pub(crate) statics: &'a dir::StaticTable<'static>,
    /// The generic table.
    pub(crate) generics: &'a dir::GenericTable<'static>,
    /// The resolution table.
    pub(crate) resolutions: &'a dir::ResolutionTable<'static>,
    /// The module table.
    pub(crate) modules: dir::ModuleTable<'static>,
    /// The target configuration.
    pub(crate) target: &'a Target,

    /// The output JS AST tree.
    pub(crate) tree: js::Tree,
    /// Root nodes in the output.
    pub(crate) roots: Vec<js::LocalNodeIdAny>,
    /// String pool for the output.
    pub(crate) strings: StringPool,
    /// Collected warnings.
    pub(crate) warnings: Vec<CodegenJsWarning>,
    /// Collected non-fatal errors (treated as warnings for continued processing).
    pub(crate) errors: Vec<CodegenJsError>,
}

impl<'a> ModuleLowerer<'a> {
    // TODO #Cleanup: not entirely sure if guarding js::ModuleLowerer only to local operands is right?

    /// Return one checked type visible to this lowering context.
    pub(crate) fn require_type(&self, type_id: dir::GlobalTypeId) -> CodegenJsResult<&dir::Type> {
        if type_id.module_id != self.module.id {
            return Err(CodegenJsError::Internal {
                message: format!("JS lowering cannot read foreign DIR type {type_id:?}"),
            });
        }

        Ok(self.types.get_type(type_id.local_id))
    }

    /// Return one checked type source visible to this lowering context.
    pub(crate) fn require_type_source(
        &self,
        type_id: dir::GlobalTypeId,
    ) -> CodegenJsResult<dir::LocalNodeIdAny> {
        if type_id.module_id != self.module.id {
            return Err(CodegenJsError::Internal {
                message: format!("JS lowering cannot read foreign DIR type source {type_id:?}"),
            });
        }

        Ok(self.types.get_type_source(type_id.local_id))
    }

    /// Return one checked static value visible to this lowering context.
    pub(crate) fn require_static(
        &self,
        static_id: dir::GlobalStaticId,
    ) -> CodegenJsResult<&dir::StaticTerm> {
        if static_id.module_id != self.module.id {
            return Err(CodegenJsError::Internal {
                message: format!("JS lowering cannot read foreign DIR static {static_id:?}"),
            });
        }

        Ok(self.statics.get_static(static_id.local_id))
    }

    /// Build one lowered script symbol id from one local DIR symbol.
    pub(crate) fn source_symbol_id(&self, symbol_id: dir::LocalSymbolId) -> ScriptSymbolId {
        ScriptSymbolId::Source(symbol_id.into_global(self.module.id))
    }

    /// Return the source symbol declared by one DIR node.
    pub(crate) fn source_symbol_for_node<T>(
        &self,
        node_id: dir::LocalNodeId<T>,
    ) -> Option<dir::LocalSymbolId>
    where
        T: dir::Node,
    {
        let node_id = node_id.into_global_any(self.module.id);

        self.symbols.declaration_symbol(node_id)
    }

    /// Store one lowered script symbol id on one JS AST node.
    pub(crate) fn set_node_symbol<T>(
        &mut self,
        node_id: js::LocalNodeId<T>,
        symbol_id: ScriptSymbolId,
    ) where
        T: js::Node,
        js::Tree: js::TreeImpl<T>,
    {
        self.tree.set_symbol(node_id, symbol_id);
    }

    /// Store one source-backed symbol id on one JS AST node.
    pub(crate) fn set_source_node_symbol<T>(
        &mut self,
        node_id: js::LocalNodeId<T>,
        symbol_id: dir::LocalSymbolId,
    ) where
        T: js::Node,
        js::Tree: js::TreeImpl<T>,
    {
        self.set_node_symbol(node_id, self.source_symbol_id(symbol_id));
    }

    /// Copy the source symbol declared by one DIR node when one exists.
    pub(crate) fn copy_source_node_symbol<T, U>(
        &mut self,
        node_id: js::LocalNodeId<T>,
        source_id: dir::LocalNodeId<U>,
    ) where
        T: js::Node,
        U: dir::Node,
        js::Tree: js::TreeImpl<T>,
    {
        if let Some(symbol_id) = self.source_symbol_for_node(source_id) {
            self.set_source_node_symbol(node_id, symbol_id);
        }
    }

    /// Store one global source-backed symbol id on one JS AST node.
    pub(crate) fn set_global_node_symbol<T>(
        &mut self,
        node_id: js::LocalNodeId<T>,
        symbol_id: dir::GlobalSymbolId,
    ) where
        T: js::Node,
        js::Tree: js::TreeImpl<T>,
    {
        self.set_node_symbol(node_id, ScriptSymbolId::Source(symbol_id));
    }

    /// Create a new module lowerer.
    pub fn new(
        module: &'a Module,
        parsed: &'a DirParsed,
        source_strings: &'a StringPool,
        bound: &'a DirBound,
        symbols: dir::BindingTable<'static>,
        types: &'a dir::TypeTable<'static>,
        statics: &'a dir::StaticTable<'static>,
        generics: &'a dir::GenericTable<'static>,
        resolutions: &'a dir::ResolutionTable<'static>,
        modules: dir::ModuleTable<'static>,
        target: &'a Target,
    ) -> Self {
        let strings = StringPool::new();
        strings.ensure_all_from(source_strings);

        Self {
            module,
            parsed,
            source_strings,
            dir_tree: &parsed.tree,
            dir_roots: bound.roots.as_ref(),
            symbols,
            types,
            statics,
            generics,
            resolutions,
            modules,
            target,
            tree: js::Tree::new(),
            roots: Vec::new(),
            strings,
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Record a non-fatal error (allows lowering to continue).
    pub(crate) fn error(&mut self, error: CodegenJsError) {
        self.errors.push(error);
    }

    /// Record a warning.
    pub(crate) fn warning(&mut self, warning: CodegenJsWarning) {
        self.warnings.push(warning);
    }

    /// Lower the module to JS AST.
    pub fn lower_module(&mut self) -> CodegenJsResult<()> {
        for expression_id in self.dir_roots.iter() {
            match self.lower_expression(*expression_id) {
                Ok(root_id) => self.roots.push(root_id),
                Err(error) => self.error(error),
            }
        }
        Ok(())
    }
}
