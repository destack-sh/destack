use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Dependency, Origin, Relation};

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
    fn compose(self, inner: Variance) -> Variance {
        match (self, inner) {
            (Variance::Bivariant, _) | (_, Variance::Bivariant) => Variance::Bivariant,
            (Variance::Invariant, _) | (_, Variance::Invariant) => Variance::Invariant,
            (own, other) if own == other => Variance::Covariant,
            _ => Variance::Contravariant,
        }
    }

    /// Flip into the contravariant position.
    fn flip(self) -> Variance {
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
    /// The derivation finished.
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

        // replay finished derivations, recursive uses start optimistic
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
        // find the declaring definition through the parameter's template
        let Some(binding) = self.generic_parameter(parameter) else {
            return Ok(Variance::Invariant);
        };
        let template = binding.template.into_global(parameter.module_id);
        let Some(template) = self.generic_template(template) else {
            return Ok(Variance::Invariant);
        };
        let source = template.source;
        let Some(module) = self.modules.get(&source.module_id) else {
            return Ok(Variance::Invariant);
        };
        let Some(symbol) = module.declaration_symbol(source.local_id) else {
            return Ok(Variance::Invariant);
        };
        let Some(definition) = self.definition(symbol) else {
            // function templates infer per call and need no variance
            return Ok(Variance::Invariant);
        };

        // collect the measured member types before walking type graphs
        let members = definition
            .members()
            .iter()
            .filter_map(|member| match member {
                // mutable fields force both positions
                dir::DefinitionMember::Field(field) => Some((field.ty, Variance::Invariant)),
                dir::DefinitionMember::Method(method) => Some((method.ty, Variance::Covariant)),
                dir::DefinitionMember::AssociatedType(associated) => associated
                    .value
                    .or(associated.constraint)
                    .map(|ty| (ty, Variance::Covariant)),
                dir::DefinitionMember::AssociatedConst(associated) => {
                    Some((associated.ty, Variance::Covariant))
                }
                dir::DefinitionMember::CallSignature(signature)
                | dir::DefinitionMember::ConstructSignature(signature)
                | dir::DefinitionMember::IndexSignature(signature) => {
                    Some((signature.ty, Variance::Covariant))
                }
                dir::DefinitionMember::Variant(_) => None,
            })
            .collect::<SmallVec<[_; 8]>>();
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

        let measured = match self.ty(ty)?.clone() {
            // the measured parameter occurs at this position
            dir::Type::Parameter(occurrence) if occurrence == parameter => position,

            // functions flip inputs and keep outputs
            dir::Type::Function(function) => {
                let mut measured = Variance::Bivariant;
                if let Some(this_parameter) = function.this_parameter {
                    measured = measured.join(self.measure_type(
                        this_parameter,
                        position.flip(),
                        parameter,
                    )?);
                }
                for input in &function.parameters {
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
            dir::Type::Reference(instance) => {
                self.measure_application(instance.symbol, &instance.arguments, position, parameter)?
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
                let mut measured = Variance::Bivariant;
                for element in &tuple.elements {
                    measured = measured.join(self.measure_type(element.ty, position, parameter)?);
                }

                measured
            }

            // structural shapes measure mutable fields both ways
            dir::Type::Shape(shape) => {
                let mut measured = Variance::Bivariant;
                for field in &shape.fields {
                    let field_position = if field.is_readonly {
                        position
                    } else {
                        Variance::Invariant
                    };
                    measured =
                        measured.join(self.measure_type(field.ty, field_position, parameter)?);
                }
                for signature in shape
                    .call_signatures
                    .iter()
                    .chain(&shape.construct_signatures)
                {
                    measured = measured.join(self.measure_type(*signature, position, parameter)?);
                }
                for signature in &shape.index_signatures {
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
                // readable views keep their position
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
                let mut measured = Variance::Bivariant;
                for element in &union.elements {
                    measured = measured.join(self.measure_type(*element, position, parameter)?);
                }

                measured
            }
            dir::Type::Intersection(intersection) => {
                let mut measured = Variance::Bivariant;
                for element in &intersection.elements {
                    measured = measured.join(self.measure_type(*element, position, parameter)?);
                }

                measured
            }

            // projections and operations are unmeasurable
            dir::Type::Member(member) => {
                let mut measured =
                    self.measure_type(member.owner, Variance::Invariant, parameter)?;
                for argument in &member.arguments {
                    measured = measured.join(self.measure_type(
                        *argument,
                        Variance::Invariant,
                        parameter,
                    )?);
                }

                measured
            }
            dir::Type::Operation(_) | dir::Type::Dynamic(_) => {
                let mut children = SmallVec::<[dir::GlobalTypeId; 8]>::new();
                self.ty(ty)?.for_each_child(|child| children.push(child));
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

    /// Decide same-template argument pairs by their parameter variances.
    pub(in crate::check) fn decide_arguments_by_variance(
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
            .generics
            .template_by_symbol(symbol)
            .map(|template| self.generic_template_parameters(template));

        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for (index, (source, target)) in source.iter().zip(target.iter()).enumerate() {
            // unknown templates compare invariantly
            let variance = match &parameters {
                Some(parameters) => match parameters.get(index) {
                    Some(parameter) => self.parameter_variance(*parameter)?,
                    None => Variance::Invariant,
                },
                None => Variance::Invariant,
            };

            let decision = match variance {
                // unused parameters relate freely
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
            match decision {
                Answer::Ready(false) => return Ok(Answer::Ready(false)),
                Answer::Ready(true) => {}
                Answer::Pending(dependencies) => blockers.extend(dependencies),
            }
        }

        if blockers.is_empty() {
            Ok(Answer::Ready(true))
        } else {
            Ok(Answer::pending(blockers))
        }
    }

    /// Constrain same-template argument pairs by their parameter variances.
    pub(in crate::check) fn constrain_arguments_by_variance(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        source: &[dir::GlobalTypeId],
        target: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        let parameters = self
            .generics
            .template_by_symbol(symbol)
            .map(|template| self.generic_template_parameters(template));

        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for (index, (source, target)) in source.iter().zip(target.iter()).enumerate() {
            // unknown templates compare invariantly
            let variance = match &parameters {
                Some(parameters) => match parameters.get(index) {
                    Some(parameter) => self.parameter_variance(*parameter)?,
                    None => Variance::Invariant,
                },
                None => Variance::Invariant,
            };

            let answer = match variance {
                // unused parameters relate freely
                Variance::Bivariant => Answer::Ready(true),
                Variance::Covariant => {
                    self.constrain(origin, Relation::Assignable, *source, *target)?
                }
                Variance::Contravariant => {
                    self.constrain(origin, Relation::Assignable, *target, *source)?
                }
                Variance::Invariant => self.constrain(origin, Relation::Equal, *source, *target)?,
            };
            match answer {
                Answer::Ready(false) => return Ok(Answer::Ready(false)),
                Answer::Ready(true) => {}
                Answer::Pending(dependencies) => blockers.extend(dependencies),
            }
        }

        if blockers.is_empty() {
            Ok(Answer::Ready(true))
        } else {
            Ok(Answer::pending(blockers))
        }
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
            .generics
            .template_by_symbol(base)
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
