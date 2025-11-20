use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{
    Asynchrony, FunctionAbstraction, FunctionCardinality, FunctionKind, FunctionMode,
    FunctionSignature, Module, ScopeId,
};

impl<'a> Compiler<'a> {
    /// Lower function kind into a DIR function kind.
    #[inline]
    pub(super) fn lower_function_kind(&self, kind: ast::FunctionKind) -> FunctionKind {
        match kind {
            ast::FunctionKind::Function => FunctionKind::Function,
            ast::FunctionKind::Lambda => FunctionKind::Lambda,
        }
    }

    /// Lower asynchrony into a DIR asynchrony.
    #[inline]
    pub(super) fn lower_asynchrony(&self, asynchrony: ast::Asynchrony) -> Asynchrony {
        match asynchrony {
            ast::Asynchrony::Sync => Asynchrony::Sync,
            ast::Asynchrony::Async => Asynchrony::Async,
        }
    }

    /// Lower function cardinality into a DIR function cardinality.
    #[inline]
    pub(super) fn lower_function_cardinality(
        &self,
        cardinality: ast::FunctionCardinality,
    ) -> FunctionCardinality {
        match cardinality {
            ast::FunctionCardinality::Scalar => FunctionCardinality::Scalar,
            ast::FunctionCardinality::Generator => FunctionCardinality::Generator,
        }
    }

    /// Lower function mode into a DIR function mode.
    #[inline]
    pub(super) fn lower_function_mode(&self, mode: ast::FunctionMode) -> FunctionMode {
        match mode {
            ast::FunctionMode::Getter => FunctionMode::Getter,
            ast::FunctionMode::Setter => FunctionMode::Setter,
            ast::FunctionMode::Constructor => FunctionMode::Constructor,
            ast::FunctionMode::New => FunctionMode::New,
            ast::FunctionMode::Call => FunctionMode::Call,
        }
    }

    /// Lower function abstraction into a DIR function abstraction.
    #[inline]
    pub(super) fn lower_function_abstraction(
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

    /// Lower function signature into a DIR function signature.
    pub(super) fn lower_function_signature(
        &self,
        module: &Module,
        scope_id: ScopeId,
        signature: &ast::FunctionSignature,
    ) -> FunctionSignature {
        let abstraction = self.lower_function_abstraction(signature.abstraction);
        let asynchrony = self.lower_asynchrony(signature.asynchrony);
        let cardinality = self.lower_function_cardinality(signature.cardinality);
        let mode = signature.mode.map(|mode| self.lower_function_mode(mode));
        let kind = self.lower_function_kind(signature.kind);
        let generics = signature
            .generics
            .as_ref()
            .map(|generics| self.lower_generics(module, scope_id, generics));
        let dynamic_parameters = signature
            .dynamic_parameters
            .iter()
            .map(|parameter| self.lower_parameter(module, scope_id, *parameter))
            .collect();
        let return_type = signature
            .return_type
            .map(|return_type| self.lower_expression_to_type(module, scope_id, return_type));
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
