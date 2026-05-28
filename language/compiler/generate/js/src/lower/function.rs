use destack_dir as dir;
use destack_js as js;

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

    /// Lower function role from DIR into JS AST.
    pub fn lower_function_role(&self, role: dir::FunctionRole) -> js::FunctionRole {
        match role {
            dir::FunctionRole::Getter => js::FunctionRole::Getter,
            dir::FunctionRole::Setter => js::FunctionRole::Setter,
            dir::FunctionRole::Constructor => js::FunctionRole::Constructor,
            dir::FunctionRole::New | dir::FunctionRole::Call => {
                panic!("type-space function modes must lower through js type nodes")
            }
        }
    }

    /// Lower function form from DIR into JS AST.
    pub fn lower_function_form(&self, form: dir::FunctionForm) -> js::FunctionForm {
        match form {
            dir::FunctionForm::Function => js::FunctionForm::Function,
            dir::FunctionForm::Lambda => js::FunctionForm::Lambda,
        }
    }

    /// Lower a function signature from DIR into JS AST.
    pub fn lower_function_signature(
        &mut self,
        function_signature: &dir::FunctionSignature,
    ) -> CodegenJsResult<js::FunctionSignature> {
        let asynchrony = self.lower_asynchrony(function_signature.asynchrony);
        let role = function_signature
            .role
            .map(|role| self.lower_function_role(role));
        let form = self.lower_function_form(function_signature.form);
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
            asynchrony,
            role,
            form,
            generic_parameters,
            this_parameter,
            parameters,
            return_type,
            is_abstract: function_signature.is_abstract,
            is_override: function_signature.is_override,
            is_generator: function_signature.is_generator,
        })
    }
}
