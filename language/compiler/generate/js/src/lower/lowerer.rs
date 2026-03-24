//! Module lowering from DIR to JS AST.
//!
//! The `ModuleLowerer` converts elaborated DIR (Destack IR) into a JavaScript AST.
//! This is the emission stage for JS and TS targets.

use destack_artifact::Ast;
use destack_core::StringPool;
use destack_dir as dir;
use destack_dir::{SymbolTable, TypeTable};
use destack_workspace::{Module, Target};

use crate::{
    CodegenJsError, CodegenJsResult, CodegenJsWarning, LocalNodeIdAny, NodeTree as JsTree,
};

/// Context for lowering a DIR module to JS AST.
#[derive(Debug)]
#[allow(dead_code)]
pub struct ModuleLowerer<'a> {
    /// The source module.
    pub(crate) module: &'a Module,
    /// The source module AST.
    pub(crate) ast: &'a Ast,
    /// The source string pool for bound DIR nodes.
    pub(crate) source_strings: &'a StringPool,

    /// The DIR roots.
    pub(crate) dir_roots: &'a Vec<dir::LocalNodeId<dir::Expression>>,
    /// The DIR tree.
    pub(crate) dir_tree: &'a dir::NodeTree,
    /// The symbol table.
    pub(crate) symbols: &'a SymbolTable,
    /// The type table.
    pub(crate) types: &'a TypeTable,
    /// The target configuration.
    pub(crate) target: &'a Target,

    /// The output JS AST tree.
    pub(crate) tree: JsTree,
    /// Root nodes in the output.
    pub(crate) roots: Vec<LocalNodeIdAny>,
    /// String pool for the output.
    pub(crate) strings: StringPool,
    /// Collected warnings.
    pub(crate) warnings: Vec<CodegenJsWarning>,
    /// Collected non-fatal errors (treated as warnings for continued processing).
    pub(crate) errors: Vec<CodegenJsError>,
}

impl<'a> ModuleLowerer<'a> {
    /// Return whether one specifier is package-like instead of local.
    pub(crate) fn is_package_dependency_specifier(specifier: &str) -> bool {
        !specifier.starts_with('.') && !specifier.starts_with('/') && !specifier.contains(':')
    }

    /// Return whether the active target externalizes a dependency specifier.
    pub(crate) fn target_externalizes_dependency_specifier(&self, specifier: &str) -> bool {
        let dependency = &self.target.bundle.dependency;

        dependency
            .external
            .iter()
            .any(|candidate| candidate == specifier)
            || dependency
                .never_bundle
                .iter()
                .any(|candidate| candidate == specifier)
    }

    /// Return whether one unresolved dependency expression should remain external.
    pub(crate) fn allows_unresolved_external_dependency_expression(
        &self,
        expression: &dir::Expression,
    ) -> bool {
        let specifier = match expression {
            dir::Expression::UnresolvedImport {
                target: dir::ImportTarget::String(target),
                ..
            }
            | dir::Expression::UnresolvedReExport { target, .. } => {
                self.source_strings.get(*target)
            }
            _ => return false,
        };

        Self::is_package_dependency_specifier(&specifier)
            && self.target_externalizes_dependency_specifier(&specifier)
    }

    /// Create a new module lowerer.
    pub fn new(
        module: &'a Module,
        ast: &'a Ast,
        source_strings: &'a StringPool,
        dir_tree: &'a dir::NodeTree,
        dir_roots: &'a Vec<dir::LocalNodeId<dir::Expression>>,
        symbols: &'a SymbolTable,
        types: &'a TypeTable,
        target: &'a Target,
    ) -> Self {
        Self {
            module,
            ast,
            source_strings,
            dir_tree,
            dir_roots,
            symbols,
            types,
            target,
            tree: JsTree::new(),
            roots: Vec::new(),
            strings: StringPool::new(),
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
