use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{
    Asynchrony, FunctionAbstraction, FunctionCardinality, FunctionKind, FunctionMode,
    FunctionSignature, Module, NodeTree, LocalScopeId,
};

impl<'a> Compiler<'a> {
    /// Bind function kind into a DIR function kind.
    #[inline]
    pub(super) fn bind_function_kind(&self, kind: ast::FunctionKind) -> FunctionKind {
        match kind {
            ast::FunctionKind::Function => FunctionKind::Function,
            ast::FunctionKind::Lambda => FunctionKind::Lambda,
        }
    }

    /// Bind asynchrony into a DIR asynchrony.
    #[inline]
    pub(super) fn bind_asynchrony(&self, asynchrony: ast::Asynchrony) -> Asynchrony {
        match asynchrony {
            ast::Asynchrony::Sync => Asynchrony::Sync,
            ast::Asynchrony::Async => Asynchrony::Async,
        }
    }

    /// Bind function cardinality into a DIR function cardinality.
    #[inline]
    pub(super) fn bind_function_cardinality(
        &self,
        cardinality: ast::FunctionCardinality,
    ) -> FunctionCardinality {
        match cardinality {
            ast::FunctionCardinality::Scalar => FunctionCardinality::Scalar,
            ast::FunctionCardinality::Generator => FunctionCardinality::Generator,
        }
    }

    /// Bind function mode into a DIR function mode.
    #[inline]
    pub(super) fn bind_function_mode(&self, mode: ast::FunctionMode) -> FunctionMode {
        match mode {
            ast::FunctionMode::Getter => FunctionMode::Getter,
            ast::FunctionMode::Setter => FunctionMode::Setter,
            ast::FunctionMode::Constructor => FunctionMode::Constructor,
            ast::FunctionMode::New => FunctionMode::New,
            ast::FunctionMode::Call => FunctionMode::Call,
        }
    }

    /// Bind function abstraction into a DIR function abstraction.
    #[inline]
    pub(super) fn bind_function_abstraction(
        &self,
        abstraction: ast::FunctionAbstraction,
    ) -> FunctionAbstraction {
        match abstraction {
            ast::FunctionAbstraction::Abstract => FunctionAbstraction::Abstract,
            ast::FunctionAbstraction::AbstractOverride => FunctionAbstraction::AbstractOverride,
            ast::FunctionAbstraction::ConcreteOverride => FunctionAbstraction::ConcreteOverride,
            ast::FunctionAbstraction::Concrete => FunctionAbstraction::Concrete,
        }
    }

    /// Bind function signature into a DIR function signature.
    pub(super) fn bind_function_signature(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        signature: &ast::FunctionSignature,
        tree: &mut NodeTree,
    ) -> FunctionSignature {
        let abstraction = self.bind_function_abstraction(signature.abstraction);
        let asynchrony = self.bind_asynchrony(signature.asynchrony);
        let cardinality = self.bind_function_cardinality(signature.cardinality);
        let mode = signature.mode.map(|mode| self.bind_function_mode(mode));
        let kind = self.bind_function_kind(signature.kind);
        let generics = signature
            .generics
            .as_ref()
            .map(|generics| self.bind_generics(module, scope_id, generics, tree));
        let dynamic_parameters = signature
            .dynamic_parameters
            .iter()
            .map(|parameter| self.bind_parameter(module, scope_id, *parameter, tree))
            .collect();
        let return_type = signature
            .return_type
            .map(|return_type| self.bind_expression_to_type(module, scope_id, return_type, tree));
        FunctionSignature {
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
