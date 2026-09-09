use destack_core::StringId;
use destack_mir as mir;
use destack_mir::Substitution;

use crate::instantiate::state::InstantiateState;
use crate::{CompilerError, CompilerResult};

/// What one call in a template body dispatches to once its receiver closes.
pub(crate) enum Dispatch {
    /// The function the call names after substitution.
    Function(mir::FunctionId),
    /// A clone the compiler implements by loading through the receiver.
    Clone,
    /// A drop the compiler answers with an empty body.
    Drop,
    /// The additive identity of the receiver's representation.
    Zero,
    /// The multiplicative identity of the receiver's representation.
    One,
}

impl InstantiateState<'_> {
    /// Return the function one direct call names, an applied template specialized on demand.
    pub(crate) fn dispatch_direct(
        &mut self,
        callee: mir::FunctionId,
        applied: &[mir::GenericArgument],
    ) -> CompilerResult<mir::FunctionId> {
        if applied.is_empty() {
            return Ok(callee);
        }

        self.specialization(callee, applied.to_vec())
    }

    /// Return what one witness call dispatches to at its closed receiver.
    pub(crate) fn dispatch_witness(
        &mut self,
        receiver: mir::TypeId,
        interface: mir::TypeId,
        requirement: mir::FunctionId,
    ) -> CompilerResult<Dispatch> {
        let Some(witness) = self.witnesses.get(&mut self.tree, receiver, interface) else {
            return Err(CompilerError::Internal {
                message: format!(
                    "a witness call to '{}' at a closed receiver without a witness",
                    self.strings.get(self.tree.get(requirement).name)
                ),
            });
        };

        // take the implementer the witness names
        let named = witness
            .functions
            .iter()
            .find(|function| function.requirement == requirement)
            .map(|function| function.function);
        if let Some(function) = named {
            return Ok(Dispatch::Function(function));
        }

        // specialize the requirement's default body at the receiver
        let symbol = self.tree.get(requirement).symbol;
        if self.templates.contains_key(&symbol) {
            let mut arguments = vec![mir::GenericArgument::Type(receiver)];
            if let mir::Type::Application {
                arguments: applied, ..
            } = self.tree.get(interface)
            {
                arguments.extend(applied.iter().cloned());
            }
            let specialization = self.specialization(requirement, arguments)?;

            return Ok(Dispatch::Function(specialization));
        }

        self.intrinsic_requirement(interface, requirement)
    }

    /// Return the global one closed receiver answers an interface's associated const with.
    pub(crate) fn witness_constant(
        &mut self,
        receiver: mir::TypeId,
        interface: mir::TypeId,
        member: StringId,
    ) -> CompilerResult<mir::GlobalId> {
        let Some(witness) = self.witnesses.get(&mut self.tree, receiver, interface) else {
            return Err(CompilerError::Internal {
                message: format!(
                    "a witness const '{}' read at a closed receiver without a witness",
                    self.strings.get(member)
                ),
            });
        };

        witness
            .constants
            .iter()
            .find(|constant| constant.member == member)
            .map(|constant| constant.global)
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "a witness without the associated const '{}'",
                    self.strings.get(member)
                ),
            })
    }

    /// Return the structural implementation of one requirement the witness leaves unnamed.
    fn intrinsic_requirement(
        &self,
        interface: mir::TypeId,
        requirement: mir::FunctionId,
    ) -> CompilerResult<Dispatch> {
        match mir::LanguageItem::of_type(&self.tree, interface) {
            Some(mir::LanguageItem::Clone) => Ok(Dispatch::Clone),
            Some(mir::LanguageItem::Drop) => Ok(Dispatch::Drop),
            Some(mir::LanguageItem::Zero) => Ok(Dispatch::Zero),
            Some(mir::LanguageItem::One) => Ok(Dispatch::One),
            Some(mir::LanguageItem::Copy) | None => Err(CompilerError::Internal {
                message: format!(
                    "an intrinsic '{}' requirement outside the structural set",
                    self.strings.get(self.tree.get(requirement).name)
                ),
            }),
        }
    }

    /// Return the specialization of one template at closed arguments, declared on first demand.
    fn specialization(
        &mut self,
        template: mir::FunctionId,
        arguments: Vec<mir::GenericArgument>,
    ) -> CompilerResult<mir::FunctionId> {
        let declared = self.tree.get(template).clone();
        let symbol = declared.symbol.instantiate(&arguments, &self.tree);
        if let Some(existing) = self.functions.get(&symbol) {
            return Ok(*existing);
        }

        // declare the header at the arguments, the body arriving from the template
        let parameters = declared
            .parameters
            .iter()
            .map(|parameter| {
                let ty = self.close_type(parameter.ty, &arguments);
                mir::FunctionParameter::new(parameter.value, ty)
            })
            .collect();
        let return_type = self.close_type(declared.return_type, &arguments);
        let specialization = mir::Function {
            generics: Vec::new(),
            arguments,
            symbol,
            linkage: mir::Linkage::Shared,
            parameters,
            return_type,
            template: Some(template),
            body: None,
            ..declared
        };
        let id = self.tree.insert(specialization);
        self.functions.insert(symbol, id);
        self.pending.push(id);

        Ok(id)
    }

    /// Substitute one type of this tree at the arguments and represent its closed applications.
    pub(crate) fn close_type(
        &mut self,
        ty: mir::TypeId,
        arguments: &[mir::GenericArgument],
    ) -> mir::TypeId {
        Substitution::new(&mut self.tree, arguments).ty(ty)
    }
}
