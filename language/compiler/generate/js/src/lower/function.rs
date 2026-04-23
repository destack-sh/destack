use {destack_dir as dir, destack_js as js};

use crate::{CodegenJsError, CodegenJsResult, ModuleLowerer};

#[allow(clippy::too_many_arguments)]
impl ModuleLowerer<'_> {
    /// Lower asynchrony from DIR into JS AST.
    pub fn lower_asynchrony(&self, asynchrony: dir::Asynchrony) -> js::Asynchrony {
        match asynchrony {
            dir::Asynchrony::Sync => js::Asynchrony::Sync,
            dir::Asynchrony::Async => js::Asynchrony::Async,
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
            dir::FunctionMode::New | dir::FunctionMode::Call => {
                panic!("type-space function modes must lower through js type nodes")
            }
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
        let asynchrony = self.lower_asynchrony(function_signature.asynchrony);
        let cardinality = self.lower_function_cardinality(function_signature.cardinality);
        let mode = function_signature
            .mode
            .map(|mode| self.lower_function_mode(mode));
        let kind = self.lower_function_kind(function_signature.kind);
        let generic_parameters =
            self.lower_generic_parameters(&function_signature.generic_parameters)?;
        let this_parameter = function_signature
            .this_parameter
            .map(|parameter| self.lower_parameter(parameter))
            .transpose()?;
        let parameters = function_signature
            .parameters
            .iter()
            .map(|parameter| self.lower_parameter(*parameter))
            .collect::<Result<Vec<_>, CodegenJsError>>()?;
        let return_type = function_signature
            .return_type
            .map(|return_type| self.lower_type_annotation_expression(return_type))
            .transpose()?;

        Ok(js::FunctionSignature {
            is_abstract: function_signature.is_abstract,
            is_override: function_signature.is_override,
            asynchrony,
            cardinality,
            mode,
            kind,
            generic_parameters,
            this_parameter,
            parameters,
            return_type,
        })
    }
}
