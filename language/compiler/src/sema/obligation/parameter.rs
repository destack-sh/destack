use destack_core::FxIndexSet;
use destack_dir as dir;

use crate::CompilerResult;
use crate::sema::{CheckState, ObligationCheck, ObligationFailure, Variance};

impl CheckState<'_> {
    /// Check that every declared generic parameter occurs in its definition.
    pub(in crate::sema) fn check_parameter_use(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<ObligationCheck> {
        let Some(template) = self.symbol_template(symbol)? else {
            return Ok(ObligationCheck::holds());
        };
        let parameters = self.generic_template_parameters(template)?;
        if parameters.is_empty() {
            return Ok(ObligationCheck::holds());
        }

        // accept intrinsic newtypes, their parameters are compiler storage
        let backing = match self.definition(symbol)? {
            Some(dir::Definition::Newtype(newtype)) => Some(newtype.backing),
            Some(_) => None,
            None => return Ok(ObligationCheck::holds()),
        };
        if let Some(backing) = backing
            && matches!(self.ty(backing)?, dir::Type::Intrinsic)
        {
            return Ok(ObligationCheck::holds());
        }

        // check declared parameters against every exposed type
        let types = self.definition_parameter_types(symbol)?;
        let mut failures = Vec::new();
        for parameter in parameters {
            let Some(binding) = self.generic_parameter(parameter) else {
                continue;
            };

            // skip memory parameters, they have no variance role
            if binding.memory_parameter().is_some() {
                continue;
            }
            let dir::GenericParameterKey::Symbol(parameter_symbol) = binding.key else {
                continue;
            };
            let declared_variance = binding.variance;

            // require the declared modifier to admit the derived variance
            let form = self.parameter_variance_form(parameter)?;
            if let Some(declared) = declared_variance {
                let derived = self.derive_variance(parameter, form)?;
                if !Variance::from(declared).is_compatible_with(derived) {
                    failures.push(ObligationFailure::VarianceConflict {
                        source: self.symbol_source(parameter_symbol)?,
                        parameter: parameter_symbol,
                        derived,
                        declared,
                    });
                }

                continue;
            }

            // derive and cache the variance the reifier annotates
            self.parameter_variance(parameter, form)?;

            // require one occurrence in the exposed types
            let mut occurs = false;
            for ty in &types {
                if self.has_parameter_occurrence(*ty, parameter)? {
                    occurs = true;

                    break;
                }
            }
            if !occurs {
                failures.push(ObligationFailure::UnusedGenericParameter {
                    source: self.symbol_source(parameter_symbol)?,
                    parameter: parameter_symbol,
                });
            }
        }

        Ok(ObligationCheck::from_failures(failures))
    }

    /// Collect the declaration types inspected for parameter occurrence.
    fn definition_parameter_types(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let Some(definition) = self.definition(symbol)?.cloned() else {
            return Ok(Vec::new());
        };
        let mut types = Vec::new();

        // collect member types, skipping synthetic self applications
        for member in definition.members() {
            match member {
                // expose method inputs and non-constructing outputs
                dir::DefinitionMember::Method(method) => {
                    let Some(ty) = self.definition_member_type(member)? else {
                        continue;
                    };
                    let Some(signature) = self.signature_head(ty)? else {
                        types.push(ty);

                        continue;
                    };
                    let parameters = self
                        .signature_parameters(ty.module_id, signature.parameters)?
                        .to_vec();
                    types.extend(parameters.iter().map(|parameter| parameter.ty));

                    // skip constructor returns, they restate the receiver instance
                    let constructs = matches!(
                        method.slot,
                        dir::MemberSlot::Constructor | dir::MemberSlot::New
                    );
                    if !constructs {
                        types.extend(signature.return_type);
                    }
                }
                // expose associated type values and constraints
                dir::DefinitionMember::AssociatedType(associated) => {
                    types.extend(associated.value);
                    types.extend(associated.constraint);
                }
                // expose index signature key domains and values
                dir::DefinitionMember::IndexSignature(signature) => {
                    types.push(signature.key_type);
                    types.push(signature.value_type);
                }
                // skip variant singletons, they restate the receiver instance
                dir::DefinitionMember::EnumVariant(_) => {}
                dir::DefinitionMember::Field(_)
                | dir::DefinitionMember::AssociatedConst(_)
                | dir::DefinitionMember::CallSignature(_)
                | dir::DefinitionMember::ConstructSignature(_) => {
                    types.extend(self.definition_member_type(member)?);
                }
            }
        }

        // include newtype backings and complete base types
        if let dir::Definition::Newtype(newtype) = &definition {
            types.push(newtype.backing);
        }
        for heritage in definition.bases() {
            types.push(heritage.ty);
        }

        // include complete interface applications
        for conformance in definition.implementations() {
            types.push(conformance.interface);
        }

        Ok(types)
    }

    /// Return whether one generic parameter occurs in one type graph.
    pub(in crate::sema) fn has_parameter_occurrence(
        &mut self,
        ty: dir::GlobalTypeId,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<bool> {
        let mut pending = vec![ty];
        let mut seen = FxIndexSet::default();

        // walk every reachable child type
        while let Some(ty) = pending.pop() {
            if !seen.insert(ty) {
                continue;
            }

            match self.ty(ty)? {
                dir::Type::Parameter(occurrence) | dir::Type::Erased(occurrence)
                    if occurrence == parameter =>
                {
                    return Ok(true);
                }
                kind => {
                    self.for_each_type_child(ty.module_id, &kind, |child| pending.push(child))?;
                }
            }
        }

        Ok(false)
    }
}
