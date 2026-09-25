use crate::EmitError;
use tspp_dir as dir;
use tspp_js as js;

use crate::emit::js::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower asynchrony from DIR into JavaScript.
    pub(crate) fn lower_asynchrony(&self, asynchrony: dir::Asynchrony) -> js::Asynchrony {
        match asynchrony {
            dir::Asynchrony::Sync => js::Asynchrony::Sync,
            dir::Asynchrony::Async => js::Asynchrony::Async,
        }
    }

    /// Lower one accessor role from DIR into JavaScript.
    pub(crate) fn lower_function_role(
        &self,
        role: dir::FunctionRole,
    ) -> Result<js::FunctionRole, EmitError> {
        match role {
            dir::FunctionRole::Getter => Ok(js::FunctionRole::Getter),
            dir::FunctionRole::Setter => Ok(js::FunctionRole::Setter),
            dir::FunctionRole::Constructor | dir::FunctionRole::New | dir::FunctionRole::Call => {
                Err(self.internal_error(
                    "non-accessor function role reached JavaScript method lowering".to_string(),
                ))
            }
        }
    }

    /// Lower a function signature from DIR into JavaScript.
    pub(crate) fn lower_function_signature(
        &mut self,
        function_signature: &dir::FunctionSignature,
    ) -> Result<js::FunctionSignature, EmitError> {
        let asynchrony = self.lower_asynchrony(function_signature.asynchrony);
        let parameters = function_signature
            .parameters
            .iter()
            .map(|parameter| self.lower_parameter(*parameter))
            .collect::<Result<Vec<_>, EmitError>>()?;
        Ok(js::FunctionSignature {
            asynchrony,
            parameters,
            is_generator: function_signature.is_generator,
        })
    }

    /// Lower mutability from DIR into JavaScript binding syntax.
    pub(crate) fn lower_mutability(&self, mutability: dir::Mutability) -> js::Mutability {
        match mutability {
            dir::Mutability::Immutable => js::Mutability::Immutable,
            dir::Mutability::Mutable => js::Mutability::Mutable,
        }
    }

    /// Lower one for-each binding keyword from DIR into JavaScript.
    pub(crate) fn lower_for_each_keyword(
        &self,
        keyword: dir::BindingKeyword,
    ) -> js::BindingKeyword {
        match keyword {
            dir::BindingKeyword::Let => js::BindingKeyword::Let,
            dir::BindingKeyword::Const => js::BindingKeyword::Const,
        }
    }
}
