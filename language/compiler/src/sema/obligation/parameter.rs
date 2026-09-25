use tspp_core::FxIndexSet;
use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::{
    CheckState, ObligationCheck, ObligationFailure, Origin, RestParameterObligation, Variance,
};

impl CheckState<'_> {
    /// Require a rest parameter's solved type to describe an argument sequence.
    pub(in crate::sema) fn check_rest_parameter(
        &mut self,
        origin: Origin,
        obligation: &RestParameterObligation,
    ) -> CompilerResult<ObligationCheck> {
        // wait for contextual parameter inference
        let stalls = self.collect_open_variables([obligation.ty])?;
        if !stalls.is_empty() {
            return Ok(ObligationCheck::Ambiguous(stalls));
        }

        // validate concrete collections and the bounds of generic packs
        let mut visiting = FxIndexSet::default();
        if self.is_rest_parameter_type(origin, obligation.ty, &mut visiting)? {
            Ok(ObligationCheck::holds())
        } else {
            Ok(ObligationCheck::fail(
                ObligationFailure::InvalidRestParameter {
                    source: obligation.source,
                    ty: obligation.ty,
                },
            ))
        }
    }

    /// Return whether a type or its declared bounds describe an argument sequence.
    fn is_rest_parameter_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        visiting: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<bool> {
        // expand each bound at most once along a recursive constraint
        let ty = self.structurally_normalize(origin, ty)?;
        let ty = self.strip_form(origin, ty)?;
        if !visiting.insert(ty) {
            return Ok(false);
        }

        // accept sequences directly and inspect generic and compound constraints
        let subject = self.ty(ty)?;
        let mut is_valid = match subject {
            dir::Type::Never => true,
            dir::Type::Tuple(tuple) => {
                let elements = self.tuple_elements(ty.module_id, tuple.elements)?.to_vec();
                let mut is_valid = true;
                for element in elements {
                    if element.is_rest {
                        is_valid &= self.is_rest_parameter_type(origin, element.ty, visiting)?;
                    }
                }

                is_valid
            }
            dir::Type::Parameter(parameter) => {
                let parameter = self.require_generic_parameter(parameter)?;
                let mut is_valid = parameter.is_variadic;
                if let Some(constraint) = parameter.constraint {
                    is_valid |= self.is_rest_parameter_type(origin, constraint, visiting)?;
                }

                is_valid
            }
            dir::Type::Operation(_) => match self.operation_head(ty)? {
                // infer captures the complete rest sequence in a conditional type pattern
                Some(dir::TypeOperation::Infer(infer)) => match infer.constraint {
                    Some(constraint) => {
                        self.is_rest_parameter_type(origin, constraint, visiting)?
                    }
                    None => true,
                },
                _ => false,
            },
            dir::Type::Intersection(intersection) => {
                let members = self.type_ids(ty.module_id, intersection.elements)?.to_vec();
                let mut is_valid = false;
                for member in members {
                    is_valid |= self.is_rest_parameter_type(origin, member, visiting)?;
                }

                is_valid
            }
            dir::Type::Union(union) => {
                let members = self.type_ids(ty.module_id, union.elements)?.to_vec();
                let mut is_valid = true;
                for member in members {
                    is_valid &= self.is_rest_parameter_type(origin, member, visiting)?;
                }

                is_valid
            }
            _ => self.rest_element_type(origin, ty)?.is_some(),
        };

        // apply constraints established by the enclosing conditional or where clause
        for bound in self.assumed_bounds(origin, |bound| bound == &subject)? {
            is_valid |= self.is_rest_parameter_type(origin, bound, visiting)?;
        }
        visiting.shift_remove(&ty);

        Ok(is_valid)
    }

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
        let backing = match self.definition(symbol)?.as_deref() {
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
            let Some(binding) = self.generic_parameter(parameter)? else {
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
        let Some(definition) = self.definition(symbol)? else {
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
                    let parameters =
                        self.signature_parameters(ty.module_id, signature.parameters)?;
                    types.extend(parameters.iter().map(|parameter| parameter.ty));

                    // expose the bounds and predicates the method's own template writes
                    if let Some(template) = signature.template {
                        for parameter in self.generic_template_parameters(template)? {
                            if let Some(binding) = self.generic_parameter(parameter)? {
                                types.extend(binding.constraint);
                            }
                        }
                        for predicate in self.template_predicates(Some(template))? {
                            types.push(predicate.left);
                            types.push(predicate.right);
                        }
                    }

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
        if let dir::Definition::Newtype(newtype) = &*definition {
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
