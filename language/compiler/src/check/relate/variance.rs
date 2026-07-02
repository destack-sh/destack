use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, Relation};

/// One derived generic parameter variance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum Variance {
    /// The parameter is unused and relates freely.
    Bivariant,
    /// The parameter only flows out and relates covariantly.
    Covariant,
    /// The parameter only flows in and relates contravariantly.
    Contravariant,
    /// The parameter flows both ways and must match exactly.
    Invariant,
}

impl Variance {
    /// Join two occurrence measurements.
    fn join(self, other: Variance) -> Variance {
        match (self, other) {
            (Variance::Bivariant, other) => other,
            (own, Variance::Bivariant) => own,
            (own, other) if own == other => own,
            _ => Variance::Invariant,
        }
    }

    /// Compose one occurrence position with an inner position.
    pub(in crate::check) fn compose(self, inner: Variance) -> Variance {
        match (self, inner) {
            (Variance::Bivariant, _) | (_, Variance::Bivariant) => Variance::Bivariant,
            (Variance::Invariant, _) | (_, Variance::Invariant) => Variance::Invariant,
            (own, other) if own == other => Variance::Covariant,
            _ => Variance::Contravariant,
        }
    }

    /// Flip into the contravariant position.
    pub(in crate::check) fn flip(self) -> Variance {
        Variance::Contravariant.compose(self)
    }
}

impl From<dir::VarianceModifier> for Variance {
    /// Map one explicit variance modifier onto its derived value.
    fn from(modifier: dir::VarianceModifier) -> Self {
        match modifier {
            dir::VarianceModifier::In => Variance::Contravariant,
            dir::VarianceModifier::Out => Variance::Covariant,
            dir::VarianceModifier::InOut => Variance::Invariant,
        }
    }
}

/// One variance derivation entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum VarianceEntry {
    /// The derivation is on the stack, recursive uses stay optimistic.
    Deriving,
    /// The derived variance.
    Derived(Variance),
}

impl CheckState<'_> {
    /// Return one generic parameter's variance, deriving it from its
    /// uses in the declaration when no explicit modifier exists.
    pub(in crate::check) fn parameter_variance(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<Variance> {
        // explicit modifiers win
        if let Some(binding) = self.generic_parameter(parameter)
            && let Some(modifier) = binding.variance
        {
            return Ok(modifier.into());
        }

        // replay derived variances, recursive uses start optimistic
        match self.variances.get(&parameter) {
            Some(VarianceEntry::Derived(variance)) => return Ok(*variance),
            Some(VarianceEntry::Deriving) => return Ok(Variance::Bivariant),
            None => {}
        }

        // derive once with the entry marking the active derivation
        self.variances.insert(parameter, VarianceEntry::Deriving);
        let derived = self.derive_variance(parameter)?;
        self.variances
            .insert(parameter, VarianceEntry::Derived(derived));

        Ok(derived)
    }

    /// Derive one parameter's variance from its uses in the declaration.
    fn derive_variance(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<Variance> {
        // find the definition that owns the parameter's template
        let Some(binding) = self.generic_parameter(parameter) else {
            return Ok(Variance::Invariant);
        };
        let template = binding.template.into_global(parameter.module_id);
        let Some(template) = self.generic_template(template) else {
            return Ok(Variance::Invariant);
        };
        let Some(symbol) = template.symbol else {
            return Ok(Variance::Invariant);
        };
        let Some(definition) = self.definition(symbol).cloned() else {
            // function templates infer per call and need no variance
            return Ok(Variance::Invariant);
        };

        // collect the measured member types before walking type graphs
        let mut members = SmallVec::<[_; 8]>::new();
        for member in definition.members() {
            let measured = match member {
                // mutable fields force both positions
                dir::DefinitionMember::Field(_) => self
                    .require_definition_member_type(member)?
                    .map(|ty| (ty, Variance::Invariant)),
                dir::DefinitionMember::Method(_) | dir::DefinitionMember::AssociatedConst(_) => {
                    self.require_definition_member_type(member)?
                        .map(|ty| (ty, Variance::Covariant))
                }
                dir::DefinitionMember::AssociatedType(associated) => associated
                    .value
                    .or(associated.constraint)
                    .map(|ty| (ty, Variance::Covariant)),
                dir::DefinitionMember::CallSignature(signature)
                | dir::DefinitionMember::ConstructSignature(signature)
                | dir::DefinitionMember::IndexSignature(signature) => {
                    Some((signature.ty, Variance::Covariant))
                }
                dir::DefinitionMember::Variant(_) => None,
            };
            if let Some(measured) = measured {
                members.push(measured);
            }
        }
        let heritages = definition
            .heritages()
            .iter()
            .map(|heritage| (heritage.symbol, heritage.arguments.clone()))
            .collect::<SmallVec<[_; 2]>>();

        // measure every member occurrence
        let mut measured = Variance::Bivariant;
        for (ty, position) in members {
            measured = measured.join(self.measure_type(ty, position, parameter)?);
        }

        // measure heritage arguments under the base parameter positions
        for (base, arguments) in heritages {
            measured = measured.join(self.measure_application(
                base,
                &arguments,
                Variance::Covariant,
                parameter,
            )?);
        }

        Ok(measured)
    }

    /// Measure one parameter's occurrences in one type graph.
    fn measure_type(
        &mut self,
        ty: dir::GlobalTypeId,
        position: Variance,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<Variance> {
        // unused positions cannot contribute occurrences
        if position == Variance::Bivariant {
            return Ok(Variance::Bivariant);
        }

        let measured = match self.ty(ty)? {
            // the measured parameter occurs at this position
            dir::Type::Parameter(occurrence) if occurrence == parameter => position,

            // functions flip inputs and keep outputs
            dir::Type::FunctionSignature(function) => {
                let mut measured = Variance::Bivariant;
                if let Some(this_parameter) = function.this_parameter {
                    measured = measured.join(self.measure_type(
                        this_parameter,
                        position.flip(),
                        parameter,
                    )?);
                }
                let inputs = self
                    .signature_parameters(ty.module_id, function.parameters)?
                    .to_vec();
                for input in inputs {
                    measured =
                        measured.join(self.measure_type(input.ty, position.flip(), parameter)?);
                }
                if let Some(return_type) = function.return_type {
                    measured =
                        measured.join(self.measure_type(return_type, position, parameter)?);
                }

                measured
            }

            // applications compose with the base parameter variances
            dir::Type::Instance(instance) => {
                let arguments = self.type_ids(ty.module_id, instance.arguments)?.to_vec();

                self.measure_application(instance.symbol, &arguments, position, parameter)?
            }

            // aliased mutable containers force both positions
            dir::Type::Array(array) => {
                self.measure_type(array.element, Variance::Invariant, parameter)?
            }
            dir::Type::Slice(slice) => {
                self.measure_type(slice.element, Variance::Invariant, parameter)?
            }

            // value containers keep their position
            dir::Type::FixedArray(array) => {
                self.measure_type(array.element, position, parameter)?
            }
            dir::Type::Tuple(tuple) => {
                let elements = self.tuple_elements(ty.module_id, tuple.elements)?.to_vec();
                let mut measured = Variance::Bivariant;
                for element in elements {
                    measured = measured.join(self.measure_type(element.ty, position, parameter)?);
                }

                measured
            }

            // structural shapes measure mutable fields both ways
            dir::Type::Shape(shape) => {
                let fields = self.shape_fields(ty.module_id, shape.fields)?.to_vec();
                let mut measured = Variance::Bivariant;
                for field in fields {
                    let field_position = if field.is_readonly {
                        position
                    } else {
                        Variance::Invariant
                    };
                    measured =
                        measured.join(self.measure_type(field.ty, field_position, parameter)?);
                }
                let signatures = self
                    .type_ids(ty.module_id, shape.call_signatures)?
                    .iter()
                    .chain(self.type_ids(ty.module_id, shape.construct_signatures)?)
                    .copied()
                    .collect::<SmallVec<[_; 4]>>();
                for signature in signatures {
                    measured = measured.join(self.measure_type(signature, position, parameter)?);
                }
                let index_signatures = self
                    .shape_index_signatures(ty.module_id, shape.index_signatures)?
                    .to_vec();
                for signature in index_signatures {
                    measured = measured.join(self.measure_type(
                        signature.value_type,
                        Variance::Invariant,
                        parameter,
                    )?);
                }

                measured
            }

            // memory forms follow their aliasing behavior
            dir::Type::Form(form) => match form.form {
                // readonly forms keep their position
                dir::Form::Readonly => self.measure_type(form.value, position, parameter)?,
                // owned and placed payloads are values
                dir::Form::Owned | dir::Form::Placed { .. } => {
                    self.measure_type(form.value, position, parameter)?
                }
                // mutable aliases force both positions
                dir::Form::Managed | dir::Form::Borrowed { .. } | dir::Form::Raw => {
                    self.measure_type(form.value, Variance::Invariant, parameter)?
                }
            },

            // algebraic composites keep their position
            dir::Type::Union(union) => {
                let elements = self.type_ids(ty.module_id, union.elements)?.to_vec();
                let mut measured = Variance::Bivariant;
                for element in elements {
                    measured = measured.join(self.measure_type(element, position, parameter)?);
                }

                measured
            }
            dir::Type::Intersection(intersection) => {
                let elements = self.type_ids(ty.module_id, intersection.elements)?.to_vec();
                let mut measured = Variance::Bivariant;
                for element in elements {
                    measured = measured.join(self.measure_type(element, position, parameter)?);
                }

                measured
            }

            // projections and operations are unmeasurable
            dir::Type::Member(member) => {
                let mut measured =
                    self.measure_type(member.owner, Variance::Invariant, parameter)?;
                let arguments = self.type_ids(ty.module_id, member.arguments)?.to_vec();
                for argument in arguments {
                    measured = measured.join(self.measure_type(
                        argument,
                        Variance::Invariant,
                        parameter,
                    )?);
                }

                measured
            }
            dir::Type::Operation(_) | dir::Type::Dynamic(_) => {
                let mut children = SmallVec::<[dir::GlobalTypeId; 8]>::new();
                let child_ty = self.ty(ty)?;
                self.for_each_type_child(ty.module_id, &child_ty, |child| children.push(child))?;
                let mut measured = Variance::Bivariant;
                for child in children {
                    measured =
                        measured.join(self.measure_type(child, Variance::Invariant, parameter)?);
                }

                measured
            }

            // leaves carry no occurrences
            _ => Variance::Bivariant,
        };

        Ok(measured)
    }

    /// Decide same-template type arguments.
    pub(in crate::check) fn decide_type_arguments(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        source: &[dir::GlobalTypeId],
        target: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        if source.len() != target.len() {
            return Ok(Answer::Ready(false));
        }
        let parameters = self
            .symbol_template(symbol)
            .map(|template| self.generic_template_parameters(template));

        let mut decision = Answer::Ready(true);
        for (index, (source, target)) in source.iter().zip(target.iter()).enumerate() {
            // map the declared variance to its argument relation
            let variance = match &parameters {
                Some(parameters) => match parameters.get(index) {
                    Some(parameter) => self.parameter_variance(*parameter)?,
                    None => Variance::Invariant,
                },
                None => Variance::Invariant,
            };

            let answer = match variance {
                Variance::Bivariant => Answer::Ready(true),
                Variance::Covariant => {
                    self.decide_relation(origin, Relation::Assignable, *source, *target)?
                }
                Variance::Contravariant => {
                    self.decide_relation(origin, Relation::Assignable, *target, *source)?
                }
                Variance::Invariant => {
                    self.decide_relation(origin, Relation::Equal, *source, *target)?
                }
            };
            decision = decision.and(answer);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Relate same-template type arguments.
    pub(in crate::check) fn relate_type_arguments(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        source: &[dir::GlobalTypeId],
        target: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        let parameters = self
            .symbol_template(symbol)
            .map(|template| self.generic_template_parameters(template));

        let mut decision = Answer::Ready(true);
        for (index, (source, target)) in source.iter().zip(target.iter()).enumerate() {
            // map the declared variance to its argument relation
            let variance = match &parameters {
                Some(parameters) => match parameters.get(index) {
                    Some(parameter) => self.parameter_variance(*parameter)?,
                    None => Variance::Invariant,
                },
                None => Variance::Invariant,
            };

            let answer = match variance {
                Variance::Bivariant => Answer::Ready(true),
                Variance::Covariant => {
                    self.constrain(origin, Relation::Assignable, *source, *target)?
                }
                Variance::Contravariant => {
                    self.constrain(origin, Relation::Assignable, *target, *source)?
                }
                Variance::Invariant => self.constrain(origin, Relation::Equal, *source, *target)?,
            };
            decision = decision.and(answer);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Measure one application's arguments under the base variances.
    fn measure_application(
        &mut self,
        base: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
        position: Variance,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<Variance> {
        let parameters = self
            .symbol_template(base)
            .map(|template| self.generic_template_parameters(template));

        let mut measured = Variance::Bivariant;
        for (index, argument) in arguments.iter().enumerate() {
            // unknown base templates measure conservatively
            let argument_position = match &parameters {
                Some(parameters) => match parameters.get(index) {
                    Some(base_parameter) => {
                        let base_variance = self.parameter_variance(*base_parameter)?;

                        position.compose(base_variance)
                    }
                    None => Variance::Invariant,
                },
                None => Variance::Invariant,
            };
            measured = measured.join(self.measure_type(*argument, argument_position, parameter)?);
        }

        Ok(measured)
    }
}
