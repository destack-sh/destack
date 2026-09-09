use destack_dir as dir;
use destack_mir as mir;

use crate::lower::module::callable::Receiver;
use crate::lower::{FunctionDeclaration, GenericInstanceKey, GenericScope, ModuleLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

impl ModuleLowerer<'_> {
    /// Return the polymorphic function of one template, importing a foreign one.
    pub(in crate::lower) fn template_function(
        &mut self,
        tree: &mut mir::Tree,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<mir::FunctionId> {
        // declare a foreign template or an own requirement without a body on first reach
        let key = GenericInstanceKey::non_generic(symbol);
        if !self.functions.contains_key(&key)
            && let Some(definition) = self.declare_callable(tree, &key, &GenericScope::default())?
        {
            self.pending.push(definition);
        }

        // return the declared function, an undeclarable template failing here
        match self.functions.get(&key) {
            Some(FunctionDeclaration::Declared(function)) => Ok(*function),
            Some(FunctionDeclaration::Failed) => Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "an instance of an undeclared template".to_string(),
            }
            .into()),
            None => Err(CompilerError::Internal {
                message: format!("an undeclared template '{}'", self.symbol_path(symbol)?),
            }),
        }
    }

    /// Return the instance parameters one callable's own signature template declares.
    pub(in crate::lower) fn own_parameters(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<dir::GlobalGenericParameterId>> {
        // answer an empty parameter list for a signature outside a template
        let ty = self.symbol_type(symbol)?;
        let (signature, module) = self.signature(ty)?;
        let Some(template) = self.types(module)?.signature(signature).template else {
            return Ok(Vec::new());
        };

        // keep the template parameters that stand for instances
        let generics = &self.state(template.module_id)?.generics;
        let parameters = generics
            .get_template(template.local_id)
            .parameters
            .iter()
            .filter(|parameter| generics.get_parameter(**parameter).is_instance_parameter())
            .map(|parameter| parameter.into_global(template.module_id))
            .collect();

        Ok(parameters)
    }

    /// Return the template parameters one symbol lowers under.
    pub(in crate::lower) fn symbol_scope(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<GenericScope> {
        let member = self.imported_member(symbol)?;
        let owner = member.as_ref().map(|member| member.owner);
        let is_static = member.as_ref().is_some_and(|member| member.is_static);
        let mut scope = self.callable_scope(symbol, owner, is_static)?;

        // a constructor borrows its constructed storage at a slot of its own
        if matches!(
            self.callable_header(symbol)?.receiver,
            Receiver::Constructs(_)
        ) {
            scope.push_receiver_slot();
        }

        Ok(scope)
    }

    /// Return the scope one class's synthesized constructor lowers under: the class's own
    /// parameters and the slot its constructed storage is borrowed at.
    pub(in crate::lower) fn class_constructor_scope(
        &mut self,
        class: dir::GlobalSymbolId,
    ) -> CompilerResult<GenericScope> {
        let template = self
            .definition(class)?
            .and_then(|definition| definition.template())
            .map(|template| template.into_global(class.module_id));
        let mut scope = match template {
            Some(template) => GenericScope::from_templates(self, Some(template), None)?,
            None => GenericScope::default().erased(),
        };
        scope.push_receiver_slot();

        Ok(scope)
    }
}
