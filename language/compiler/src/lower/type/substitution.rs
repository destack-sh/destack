use destack_core::FxIndexMap;
use destack_dir as dir;

use crate::lower::ModuleLowerer;
use crate::{CompilerError, CompilerResult};

/// The concrete receiver replacing contextual `this` references.
#[derive(Clone, Copy)]
pub(in crate::lower) enum ReceiverBinding {
    /// A nominal owner applied to its type arguments.
    Application(dir::GenericApplication),
    /// A receiver type, as extension targets declare.
    Type(dir::GlobalTypeId),
}

/// Contextual type substitutions selecting one concrete representation.
#[derive(Clone, Default)]
pub(in crate::lower) struct TypeSubstitution {
    /// The type argument substituted for each generic parameter.
    bindings: FxIndexMap<dir::GlobalGenericParameterId, dir::GlobalTypeId>,
    /// The receiver replacing contextual `this` references.
    receiver: Option<ReceiverBinding>,
}

impl TypeSubstitution {
    /// Return this substitution with its contextual receiver.
    pub(in crate::lower) fn with_receiver(mut self, receiver: ReceiverBinding) -> Self {
        self.receiver = Some(receiver);

        self
    }

    /// Return the contextual receiver, when bound.
    pub(in crate::lower) fn receiver(&self) -> Option<ReceiverBinding> {
        self.receiver
    }

    /// Bind one template's type parameters to positional arguments.
    pub(in crate::lower) fn bind(
        lowerer: &ModuleLowerer<'_>,
        template: dir::GlobalGenericTemplateId,
        arguments: &[dir::GlobalTypeId],
        enclosing: &Self,
    ) -> CompilerResult<Self> {
        let generics = &lowerer.state(template.module_id)?.generics;
        let template_row = generics.get_template(template.local_id);
        let parameters: Vec<_> = template_row
            .parameters
            .iter()
            .filter_map(|parameter| {
                let binding = generics.get_parameter(*parameter);

                matches!(binding.kind, dir::GenericParameterKind::Type)
                    .then_some(parameter.into_global(template.module_id))
            })
            .collect();
        if parameters.len() != arguments.len() {
            return Err(CompilerError::Internal {
                message: format!(
                    "{} type arguments for the {} type parameters of {:?}",
                    arguments.len(),
                    parameters.len(),
                    template
                ),
            });
        }

        // resolve outer parameters before binding the nested template
        let mut substitution = enclosing.clone();
        for (parameter, argument) in parameters.into_iter().zip(arguments) {
            let argument = enclosing.resolve(lowerer, *argument)?;
            substitution.bindings.insert(parameter, argument);
        }

        Ok(substitution)
    }

    /// Bind one generic parameter to its selected argument.
    pub(in crate::lower) fn insert(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
        argument: dir::GlobalTypeId,
    ) {
        self.bindings.insert(parameter, argument);
    }

    /// Return the bindings as one resolved-type cache key component.
    ///
    /// Bodies of different instances share dir type ids; the bindings
    /// disambiguate the representations recorded for each instance.
    pub(in crate::lower) fn bindings_key(
        &self,
    ) -> Vec<(dir::GlobalGenericParameterId, dir::GlobalTypeId)> {
        self.bindings
            .iter()
            .map(|(parameter, argument)| (*parameter, *argument))
            .collect()
    }

    /// Return the bound argument behind one generic parameter.
    pub(in crate::lower) fn binding(
        &self,
        parameter: dir::GlobalGenericParameterId,
    ) -> Option<dir::GlobalTypeId> {
        self.bindings.get(&parameter).copied()
    }

    /// Resolve one type through every enclosing parameter binding.
    pub(in crate::lower) fn resolve(
        &self,
        lowerer: &ModuleLowerer<'_>,
        mut ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        loop {
            let dir::Type::Parameter(parameter) = lowerer.ty(ty)? else {
                return Ok(ty);
            };
            let Some(argument) = self.bindings.get(&parameter).copied() else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "type parameter {parameter:?} unbound with bindings for {:?}",
                        self.bindings.keys().collect::<Vec<_>>()
                    ),
                });
            };
            ty = argument;
        }
    }
}
