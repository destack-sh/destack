use destack_artifact::{DirBound, DirParsed};
use destack_core::StringPool;
use destack_dir as dir;
use destack_js as js;
use destack_repository::Module;

use crate::emit::js::ScriptSymbolId;
use crate::{DiagnosticAnchor, EmitError};

/// Context for lowering a DIR module to JS AST.
#[derive(Debug)]
pub(crate) struct ModuleLowerer<'a> {
    /// The source module.
    pub(crate) module: &'a Module,
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
    /// The module table.
    pub(crate) modules: dir::ModuleTable<'static>,

    /// The output JS AST tree.
    pub(crate) tree: js::Tree,
    /// Root nodes in the output.
    pub(crate) roots: Vec<js::LocalNodeIdAny>,
    /// String pool for the output.
    pub(crate) strings: StringPool,
    /// Collected non-fatal errors.
    pub(crate) errors: Vec<EmitError>,
}

impl<'a> ModuleLowerer<'a> {
    // TODO #Cleanup: not entirely sure if guarding js::ModuleLowerer only to local operands is right?

    /// Return one checked type visible to this lowering context.
    pub(crate) fn require_type(&self, type_id: dir::GlobalTypeId) -> Result<dir::Type, EmitError> {
        if type_id.module_id != self.module.id {
            return Err(self.internal_error(format!(
                "JS lowering cannot read foreign DIR type {type_id:?}"
            )));
        }

        Ok(self.types.get_type(type_id.local_id))
    }

    /// Return one checked static value visible to this lowering context.
    pub(crate) fn require_static(
        &self,
        static_id: dir::GlobalStaticId,
    ) -> Result<&dir::StaticTerm, EmitError> {
        if static_id.module_id != self.module.id {
            return Err(self.internal_error(format!(
                "JS lowering cannot read foreign DIR static {static_id:?}"
            )));
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
    pub(crate) fn new(
        module: &'a Module,
        parsed: &'a DirParsed,
        source_strings: &'a StringPool,
        bound: &'a DirBound,
        symbols: dir::BindingTable<'static>,
        types: &'a dir::TypeTable<'static>,
        statics: &'a dir::StaticTable<'static>,
        generics: &'a dir::GenericTable<'static>,
        modules: dir::ModuleTable<'static>,
    ) -> Self {
        let strings = StringPool::new();
        strings.ensure_all_from(source_strings);

        Self {
            module,
            source_strings,
            dir_tree: &parsed.tree,
            dir_roots: bound.roots.as_ref(),
            symbols,
            types,
            statics,
            generics,
            modules,
            tree: js::Tree::new(),
            roots: Vec::new(),
            strings,
            errors: Vec::new(),
        }
    }

    /// Record a non-fatal error (allows lowering to continue).
    pub(crate) fn error(&mut self, error: EmitError) {
        self.errors.push(error);
    }

    /// Build one source span anchor from a DIR node.
    pub(crate) fn anchor(&self, node: dir::GlobalNodeIdAny) -> DiagnosticAnchor {
        assert_eq!(
            self.module.id, node.module_id,
            "JS diagnostic node belongs to a different module"
        );

        let span = self
            .dir_tree
            .get_span_by_id(node.local_id.id)
            .expect("JS diagnostic node is missing a source span");

        DiagnosticAnchor::Span(span)
    }

    /// Build one unsupported construct error.
    pub(crate) fn unsupported_construct(
        &self,
        node: dir::GlobalNodeIdAny,
        message: Option<String>,
    ) -> EmitError {
        EmitError::UnsupportedConstruct {
            anchor: self.anchor(node),
            module: self.module.id,
            message: message.unwrap_or_else(|| format!("unsupported {}", node.local_id.ty.name())),
        }
    }

    /// Build one unexpected lowered node error.
    pub(crate) fn unexpected_node(
        &self,
        node: dir::GlobalNodeIdAny,
        wanted: js::NodeType,
        message: Option<String>,
    ) -> EmitError {
        EmitError::UnexpectedConstruct {
            anchor: self.anchor(node),
            module: self.module.id,
            message: message.unwrap_or_else(|| {
                format!(
                    "unexpected {} (wanted {})",
                    node.local_id.ty.name(),
                    wanted.name()
                )
            }),
        }
    }

    /// Return one lowered node as the expected JS node type.
    pub(crate) fn expect_node<T>(
        &self,
        node_id: js::LocalNodeIdAny,
        source_id: dir::GlobalNodeIdAny,
    ) -> Result<js::LocalNodeId<T>, EmitError>
    where
        T: js::Node,
    {
        if node_id.ty == T::TYPE {
            Ok(js::LocalNodeId::<T>::new(node_id.id))
        } else {
            let source_kind = if source_id.local_id.ty == dir::NodeType::Expression {
                " for expression"
            } else {
                ""
            };

            Err(self.unexpected_node(
                source_id,
                T::TYPE,
                Some(format!(
                    "lowered to unexpected {} (wanted {}){source_kind}",
                    node_id.ty.name(),
                    T::TYPE.name()
                )),
            ))
        }
    }

    /// Lower one DIR expression and anchor shape errors at the given source node.
    pub(crate) fn lower_expression_as_anchored<T>(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        anchor: dir::GlobalNodeIdAny,
    ) -> Result<js::LocalNodeId<T>, EmitError>
    where
        T: js::Node,
    {
        let node_id = self.lower_expression(expression_id)?;

        self.expect_node::<T>(node_id, anchor)
    }

    /// Lower one DIR expression and return it as the expected JS node type.
    pub(crate) fn lower_expression_as<T>(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::LocalNodeId<T>, EmitError>
    where
        T: js::Node,
    {
        self.lower_expression_as_anchored::<T>(
            expression_id,
            expression_id.into_global_any(self.module.id),
        )
    }

    /// Build one missing type error.
    pub(crate) fn missing_type(&self, node: dir::GlobalNodeIdAny) -> EmitError {
        EmitError::MissingType {
            anchor: self.anchor(node),
            module: self.module.id,
        }
    }

    /// Build one internal JS emit error.
    pub(crate) fn internal_error(&self, message: String) -> EmitError {
        EmitError::Internal {
            anchor: self.module.id.into(),
            module: self.module.id,
            message,
        }
    }

    /// Lower the module to JS AST.
    pub(crate) fn lower_module(&mut self) -> Result<(), EmitError> {
        for expression_id in self.dir_roots.iter() {
            match self.lower_expression(*expression_id) {
                Ok(root_id) => self.roots.push(root_id),
                Err(error) => self.error(error),
            }
        }
        Ok(())
    }
}
