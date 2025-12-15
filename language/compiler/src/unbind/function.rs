use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use crate::Compiler;

impl Compiler {
    /// Unbind a DIR function kind to an AST function kind.
    #[inline]
    pub(super) fn unbind_function_kind(&self, kind: dir::FunctionKind) -> ast::FunctionKind {
        match kind {
            dir::FunctionKind::Function => ast::FunctionKind::Function,
            dir::FunctionKind::Lambda => ast::FunctionKind::Lambda,
        }
    }

    /// Unbind a DIR function cardinality to an AST function cardinality.
    #[inline]
    pub(super) fn unbind_function_cardinality(
        &self,
        cardinality: dir::FunctionCardinality,
    ) -> ast::FunctionCardinality {
        match cardinality {
            dir::FunctionCardinality::Scalar => ast::FunctionCardinality::Scalar,
            dir::FunctionCardinality::Generator => ast::FunctionCardinality::Generator,
        }
    }

    /// Unbind a DIR function mode to an AST function mode.
    #[inline]
    pub(super) fn unbind_function_mode(&self, mode: dir::FunctionMode) -> ast::FunctionMode {
        match mode {
            dir::FunctionMode::Getter => ast::FunctionMode::Getter,
            dir::FunctionMode::Setter => ast::FunctionMode::Setter,
            dir::FunctionMode::Constructor => ast::FunctionMode::Constructor,
            dir::FunctionMode::New => ast::FunctionMode::New,
            dir::FunctionMode::Call => ast::FunctionMode::Call,
        }
    }

    /// Unbind a DIR function abstraction to an AST function abstraction.
    #[inline]
    pub(super) fn unbind_function_abstraction(
        &self,
        abstraction: dir::FunctionAbstraction,
    ) -> ast::FunctionAbstraction {
        match abstraction {
            dir::FunctionAbstraction::Abstract => ast::FunctionAbstraction::Abstract,
            dir::FunctionAbstraction::AbstractOverride => ast::FunctionAbstraction::AbstractOverride,
            dir::FunctionAbstraction::ConcreteOverride => ast::FunctionAbstraction::ConcreteOverride,
            dir::FunctionAbstraction::Concrete => ast::FunctionAbstraction::Concrete,
        }
    }

    /// Unbind a DIR function signature to an AST function signature.
    pub(super) fn unbind_function_signature(
        &self,
        module: &Module,
        signature: &dir::FunctionSignature,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
    ) -> ast::FunctionSignature {
        let abstraction = self.unbind_function_abstraction(signature.abstraction);
        let asynchrony = self.unbind_asynchrony(signature.asynchrony);
        let cardinality = self.unbind_function_cardinality(signature.cardinality);
        let mode = signature.mode.map(|mode| self.unbind_function_mode(mode));
        let kind = self.unbind_function_kind(signature.kind);
        let generics = signature.generics.as_ref().map(|generics| {
            self.unbind_generics(module, generics, tree, symbols, ast_tree, ast_strings)
        });
        let dynamic_parameters = signature
            .dynamic_parameters
            .iter()
            .map(|parameter| {
                self.unbind_parameter(module, *parameter, tree, symbols, ast_tree, ast_strings)
            })
            .collect();
        let return_type = signature.return_type.map(|return_type| {
            self.unbind_expression(module, return_type, tree, symbols, ast_tree, ast_strings)
        });
        ast::FunctionSignature {
            abstraction,
            asynchrony,
            cardinality,
            mode,
            kind,
            generics,
            dynamic_parameters,
            return_type,
        }
    }
}
