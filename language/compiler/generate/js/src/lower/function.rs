use {destack_dir as dir, destack_js as js};

use crate::{CodegenJsError, CodegenJsResult, ModuleLowerer};

impl ModuleLowerer<'_> {
    /// Lower asynchrony from DIR into JS AST.
    pub fn lower_asynchrony(&self, asynchrony: dir::Asynchrony) -> js::Asynchrony {
        match asynchrony {
            dir::Asynchrony::Sync => js::Asynchrony::Sync,
            dir::Asynchrony::Async => js::Asynchrony::Async,
        }
    }

    /// Lower a function abstraction from DIR into JS AST.
    pub fn lower_function_abstraction(
        &self,
        abstraction: dir::FunctionAbstraction,
    ) -> js::FunctionAbstraction {
        match abstraction {
            dir::FunctionAbstraction::Abstract => js::FunctionAbstraction::Abstract,
            dir::FunctionAbstraction::AbstractOverride => js::FunctionAbstraction::AbstractOverride,
            dir::FunctionAbstraction::ConcreteOverride => js::FunctionAbstraction::ConcreteOverride,
            dir::FunctionAbstraction::Concrete => js::FunctionAbstraction::Concrete,
        }
    }

    /// Lower function cardinality from DIR into JS AST.
    pub fn lower_function_cardinality(
        &self,
        cardinality: dir::FunctionCardinality,
    ) -> js::FunctionCardinality {
        match cardinality {
            dir::FunctionCardinality::Scalar => js::FunctionCardinality::Scalar,
            dir::FunctionCardinality::Generator => js::FunctionCardinality::Generator,
        }
    }

    /// Lower function mode from DIR into JS AST.
    pub fn lower_function_mode(&self, mode: dir::FunctionMode) -> js::FunctionMode {
        match mode {
            dir::FunctionMode::Getter => js::FunctionMode::Getter,
            dir::FunctionMode::Setter => js::FunctionMode::Setter,
            dir::FunctionMode::Constructor => js::FunctionMode::Constructor,
            dir::FunctionMode::New => js::FunctionMode::New,
            dir::FunctionMode::Call => js::FunctionMode::Call,
        }
    }

    /// Lower function kind from DIR into JS AST.
    pub fn lower_function_kind(&self, kind: dir::FunctionKind) -> js::FunctionKind {
        match kind {
            dir::FunctionKind::Function => js::FunctionKind::Function,
            dir::FunctionKind::Lambda => js::FunctionKind::Lambda,
        }
    }

    /// Lower a function signature from DIR into JS AST.
    pub fn lower_function_signature(
        &mut self,
        function_signature: &dir::FunctionSignature,
    ) -> CodegenJsResult<js::FunctionSignature> {
        let abstraction = self.lower_function_abstraction(function_signature.abstraction);
        let asynchrony = self.lower_asynchrony(function_signature.asynchrony);
        let cardinality = self.lower_function_cardinality(function_signature.cardinality);
        let mode = function_signature
            .mode
            .map(|mode| self.lower_function_mode(mode));
        let kind = self.lower_function_kind(function_signature.kind);
        let generics = function_signature
            .generics
            .as_ref()
            .map(|generics| self.lower_generics(generics))
            .transpose()?;
        let this_parameter = function_signature
            .this_parameter
            .map(|parameter| self.lower_parameter(parameter))
            .transpose()?;
        let dynamic_parameters = function_signature
            .dynamic_parameters
            .iter()
            .map(|parameter| self.lower_parameter(*parameter))
            .collect::<Result<Vec<_>, CodegenJsError>>()?;
        let return_type = function_signature
            .return_type
            .map(|return_type| self.lower_type_annotation_expression(return_type))
            .transpose()?;
        Ok(js::FunctionSignature {
            abstraction,
            asynchrony,
            cardinality,
            mode,
            kind,
            generics,
            this_parameter,
            dynamic_parameters,
            return_type,
        })
    }
}
