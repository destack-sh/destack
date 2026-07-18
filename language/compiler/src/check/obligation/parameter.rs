use destack_core::FxIndexSet;
use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, ObligationCheck, ObligationFailure, Variance, answer};

impl CheckState<'_> {
    /// Check that every declared generic parameter occurs in its definition.
    pub(in crate::check) fn check_parameter_use(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let Some(template) = self.symbol_template(symbol)? else {
            return Ok(Answer::Ready(ObligationCheck::holds()));
        };
        let parameters = self.generic_template_parameters(template);
        if parameters.is_empty() {
            return Ok(Answer::Ready(ObligationCheck::holds()));
        }

        // intrinsic newtype backings use their parameters as compiler storage
        let backing = match self.definition(symbol)? {
            Some(dir::Definition::Newtype(newtype)) => Some(newtype.backing),
            Some(_) => None,
            None => return Ok(Answer::Ready(ObligationCheck::holds())),
        };
        if let Some(backing) = backing
            && matches!(self.ty(backing)?, dir::Type::Intrinsic)
        {
            return Ok(Answer::Ready(ObligationCheck::holds()));
        }

        // check declared parameters against the collected surface
        let surface = answer!(self.definition_type_surface(symbol)?);
        let mut failures = Vec::new();
        for parameter in parameters {
            let Some(binding) = self.generic_parameter(parameter) else {
                continue;
            };

            // value parameters have no variance role to protect
            if binding.induced_memory_parameter().is_some()
                || binding.is_comptime()
                || binding.is_const
            {
                continue;
            }
            let dir::GenericParameterKey::Symbol(parameter_symbol) = binding.key else {
                continue;
            };

            // declared modifiers keep an unused marker parameter, but must
            //  admit the derived use under the declaration's own handle
            //  context; other contexts derive and the modifier only caps
            if let Some(declared) = binding.variance {
                let context = self.default_symbol_context(symbol);
                let derived = self.derive_variance(parameter, context)?;
                if !Variance::from(declared).admits(derived) {
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
            let context = self.default_symbol_context(symbol);
            self.parameter_variance(parameter, context)?;

            // require one occurrence anywhere in the surface
            let mut occurs = false;
            for ty in &surface {
                if self.parameter_occurs(*ty, parameter)? {
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

        Ok(Answer::Ready(ObligationCheck::from_failures(failures)))
    }

    /// Collect the types one declaration exposes for parameter occurrence.
    fn definition_type_surface(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<Vec<dir::GlobalTypeId>>> {
        let Some(definition) = self.definition(symbol)?.cloned() else {
            return Ok(Answer::Ready(Vec::new()));
        };
        let mut surface = Vec::new();

        // collect member types, skipping synthetic self applications
        for member in definition.members() {
            match member {
                // methods expose their inputs and non-constructing outputs
                dir::DefinitionMember::Method(method) => {
                    let Some(ty) = answer!(self.definition_member_type(member)?) else {
                        continue;
                    };
                    let Some(signature) = self.signature_head(ty)? else {
                        surface.push(ty);

                        continue;
                    };
                    let parameters = self
                        .signature_parameters(ty.module_id, signature.parameters)?
                        .to_vec();
                    surface.extend(parameters.iter().map(|parameter| parameter.ty));

                    // constructor returns restate the receiver instance
                    let constructs = matches!(
                        method.slot,
                        dir::MemberSlot::Constructor | dir::MemberSlot::New
                    );
                    if !constructs {
                        surface.extend(signature.return_type);
                    }
                }
                // associated types expose their value and constraint
                dir::DefinitionMember::AssociatedType(associated) => {
                    surface.extend(associated.value);
                    surface.extend(associated.constraint);
                }
                // index signatures expose their key domain and value
                dir::DefinitionMember::IndexSignature(signature) => {
                    surface.push(signature.key_type);
                    surface.push(signature.value_type);
                }
                // variant singletons restate the receiver instance
                dir::DefinitionMember::EnumVariant(_) | dir::DefinitionMember::TaggedVariant(_) => {
                }
                dir::DefinitionMember::Field(_)
                | dir::DefinitionMember::AssociatedConst(_)
                | dir::DefinitionMember::CallSignature(_)
                | dir::DefinitionMember::ConstructSignature(_) => {
                    surface.extend(answer!(self.definition_member_type(member)?));
                }
            }
        }

        // newtype backings and heritage arguments are declaration usage
        if let dir::Definition::Newtype(newtype) = &definition {
            surface.push(newtype.backing);
        }
        let bases = definition.bases();
        let heritages = definition.heritages();
        for heritage in bases.iter().chain(heritages.iter()) {
            surface.extend(heritage.arguments.iter().copied());
        }

        Ok(Answer::Ready(surface))
    }

    /// Return whether one generic parameter occurs in one type graph.
    fn parameter_occurs(
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
