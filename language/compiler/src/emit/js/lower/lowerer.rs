use tspp_core::StringPool;
use tspp_dir as dir;
use tspp_js as js;
use tspp_repository::Module;

use crate::emit::js::ScriptSymbolId;
use crate::{DiagnosticAnchor, EmitError};

/// Context for lowering a DIR module to a JavaScript tree.
#[derive(Debug)]
pub(crate) struct ModuleLowerer<'a> {
    /// The source module.
    pub(crate) module: &'a Module,

    /// The DIR roots.
    pub(crate) dir_roots: &'a [dir::LocalNodeId<dir::Expression>],
    /// The DIR tree.
    pub(crate) dir_tree: dir::View<'a>,
    /// The symbol table.
    pub(crate) symbols: dir::BindingTable<'static>,
    /// The module table.
    pub(crate) modules: dir::ModuleTable<'static>,

    /// The output JavaScript tree.
    pub(crate) tree: js::Tree,
    /// Root nodes in the output.
    pub(crate) roots: Vec<js::LocalNodeIdAny>,
    /// String pool for the output.
    pub(crate) strings: StringPool,
    /// Collected non-fatal errors.
    pub(crate) errors: Vec<EmitError>,
}

impl<'a> ModuleLowerer<'a> {
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

    /// Store one lowered script symbol id on one JavaScript node.
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

    /// Store one source-backed symbol id on one JavaScript node.
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

    /// Create a new module lowerer.
    pub(crate) fn new(
        module: &'a Module,
        tree: dir::View<'a>,
        roots: &'a [dir::LocalNodeId<dir::Expression>],
        source_strings: &'a StringPool,
        symbols: dir::BindingTable<'static>,
        modules: dir::ModuleTable<'static>,
    ) -> Self {
        let strings = StringPool::new();
        strings.ensure_all_from(source_strings);

        Self {
            module,
            dir_tree: tree,
            dir_roots: roots,
            symbols,
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
    pub(crate) fn anchor(&self, node: dir::GlobalNodeIdAny) -> Result<DiagnosticAnchor, EmitError> {
        if self.module.id != node.module_id {
            return Err(self.internal_error(
                "JavaScript diagnostic node belongs to a different module".to_string(),
            ));
        }

        let Some(span) = self.dir_tree.get_span_by_id(node.local_id.id) else {
            return Err(self.internal_error(
                "JavaScript diagnostic node is missing a source span".to_string(),
            ));
        };

        Ok(DiagnosticAnchor::Span(span))
    }

    /// Build one internal error for a valid construct that emission did not handle.
    pub(crate) fn unhandled(
        &self,
        node: dir::GlobalNodeIdAny,
        message: Option<String>,
    ) -> EmitError {
        let anchor = match self.anchor(node) {
            Ok(anchor) => anchor,
            Err(error) => return error,
        };

        EmitError::Internal {
            anchor,
            module: self.module.id,
            message: message.unwrap_or_else(|| format!("unhandled {}", node.local_id.ty.name())),
        }
    }

    /// Build one unexpected lowered node error.
    pub(crate) fn unexpected_node(
        &self,
        node: dir::GlobalNodeIdAny,
        wanted: js::NodeType,
        message: Option<String>,
    ) -> EmitError {
        let anchor = match self.anchor(node) {
            Ok(anchor) => anchor,
            Err(error) => return error,
        };

        EmitError::UnexpectedConstruct {
            anchor,
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

    /// Return one lowered node as the expected JavaScript node type.
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

    /// Lower one DIR expression and return it as the expected JavaScript node type.
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

    /// Build one internal JS emit error.
    pub(crate) fn internal_error(&self, message: String) -> EmitError {
        EmitError::Internal {
            anchor: self.module.id.into(),
            module: self.module.id,
            message,
        }
    }

    /// Lower the module to a JavaScript tree.
    pub(crate) fn lower_module(&mut self) {
        for expression_id in self.dir_roots.iter() {
            if self.expression_is_erased(*expression_id) {
                continue;
            }

            match self.lower_expression(*expression_id) {
                Ok(root_id) => self.roots.push(root_id),
                Err(error) => self.error(error),
            }
        }
    }

    /// Return whether one DIR expression has no JavaScript runtime form.
    pub(crate) fn expression_is_erased(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        match self.dir_tree.get(expression_id) {
            dir::Expression::Declaration(declaration_id) => {
                match self.dir_tree.get(*declaration_id) {
                    dir::Declaration::Global(declaration) => declaration.is_ambient,
                    dir::Declaration::Type(_) => true,
                    dir::Declaration::Struct(declaration) => declaration.is_ambient,
                    dir::Declaration::Class(declaration) => declaration.is_ambient,
                    dir::Declaration::Interface(_) => true,
                    dir::Declaration::Enum(declaration) => declaration.is_ambient,
                    dir::Declaration::Function(declaration) => declaration.body.is_none(),
                    _ => false,
                }
            }
            dir::Expression::Let { is_ambient, .. } | dir::Expression::Using { is_ambient, .. } => {
                *is_ambient
            }
            _ => false,
        }
    }
}
