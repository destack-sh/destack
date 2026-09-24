use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{FunctionDeclaration, GenericInstanceKey, GenericScope, ModuleLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

impl ModuleLowerer<'_> {
    /// Return the polymorphic function of one template, importing a foreign one.
    pub(in crate::lower) fn template_function(
        &mut self,
        tree: &mut mir::Tree,
        symbol: dir::GlobalSymbolId,
        receiver: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<mir::FunctionId> {
        // declare a foreign template or an own requirement without a body on first reach
        let key = self.template_key(tree, symbol, receiver)?;
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

    /// Return the key one template declares under, a derived member keyed by its receiver.
    fn template_key(
        &mut self,
        tree: &mut mir::Tree,
        symbol: dir::GlobalSymbolId,
        receiver: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<GenericInstanceKey> {
        if !self.is_derived_member(symbol)? {
            return Ok(GenericInstanceKey::non_generic(symbol));
        }
        let Some(receiver) = receiver else {
            return Err(CompilerError::Internal {
                message: format!(
                    "a derived member '{}' reached without its receiver",
                    self.symbol_path(symbol)?
                ),
            });
        };

        self.type_lowerer(tree, &GenericScope::default().erased())
            .generic_instance_key(symbol, Some(receiver), &[])
    }

    /// Return whether one symbol is a member the compiler derived for one receiver's witness.
    pub(in crate::lower) fn is_derived_member(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        Ok(self
            .state(symbol.module_id)?
            .derived_functions
            .contains(&symbol))
    }

    /// Return the template parameters one symbol lowers under.
    pub(in crate::lower) fn symbol_scope(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<GenericScope> {
        self.nested_symbol_scope(symbol, None)
    }

    /// Return the template parameters one callable lowers under beneath an enclosing callable.
    pub(in crate::lower) fn nested_symbol_scope(
        &mut self,
        symbol: dir::GlobalSymbolId,
        enclosing: Option<&GenericScope>,
    ) -> CompilerResult<GenericScope> {
        let member = self.imported_member(symbol)?;
        let owner = member.as_ref().map(|member| member.owner);
        let is_static = member.as_ref().is_some_and(|member| member.is_static);

        self.callable_scope(symbol, owner, is_static, enclosing)
    }

    /// Return the scope one dispatch slot indexes: the interface's parameters, then the member's.
    pub(in crate::lower) fn dispatch_slot_scope(
        &mut self,
        tree: &mir::Tree,
        interface: &GenericScope,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<GenericScope> {
        // index the member's parameters after the interface's, its signature's template first
        let declared = self.symbol_type(symbol)?;
        let signature_template = match self.ty(declared)? {
            dir::Type::FunctionSignature(signature) => {
                self.types(declared.module_id)?
                    .signature(signature)
                    .template
            }
            _ => None,
        };
        let template = match signature_template {
            Some(template) => Some(template),
            None => self
                .state(symbol.module_id)?
                .generics
                .template_by_symbol(symbol)
                .map(|template| template.into_global(symbol.module_id)),
        };

        GenericScope::for_signature(self, template, Some(symbol))?
            .with_parameters_of(interface, self, tree)
    }

    /// Return the scope one class's synthesized constructor lowers under.
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
