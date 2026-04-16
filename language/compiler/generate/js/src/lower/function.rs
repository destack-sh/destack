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

    /// Lower function abstraction flags from DIR into JS AST.
    pub fn lower_function_abstraction(
        &self,
        is_abstract: bool,
        is_override: bool,
    ) -> js::FunctionAbstraction {
        match (is_abstract, is_override) {
            (true, true) => js::FunctionAbstraction::AbstractOverride,
            (true, false) => js::FunctionAbstraction::Abstract,
            (false, true) => js::FunctionAbstraction::ConcreteOverride,
            (false, false) => js::FunctionAbstraction::Concrete,
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
        let abstraction = self.lower_function_abstraction(
            function_signature.is_abstract,
            function_signature.is_override,
        );
        let asynchrony = self.lower_asynchrony(function_signature.asynchrony);
        let cardinality = self.lower_function_cardinality(function_signature.cardinality);
        let mode = function_signature
            .mode
            .map(|mode| self.lower_function_mode(mode));
        let kind = self.lower_function_kind(function_signature.kind);
        let generics = self.lower_generic_parameters(&function_signature.generic_parameters)?;
        let this_parameter = function_signature
            .this_parameter
            .map(|parameter| self.lower_parameter(parameter))
            .transpose()?;
        let dynamic_parameters = function_signature
            .parameters
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
            generics: generics.map(|static_parameters| js::Generics {
                static_parameters: Some(static_parameters),
            }),
            this_parameter,
            dynamic_parameters,
            return_type,
        })
    }
}
