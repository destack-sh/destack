use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, Cause, CauseId, CauseKind, CheckState, Origin, Relation};

/// One derived generic parameter variance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    /// Return whether this declared variance admits one derived use.
    pub(in crate::check) fn admits(self, derived: Variance) -> bool {
        matches!(derived, Variance::Bivariant) || self == Variance::Invariant || self == derived
    }

    /// Return the written modifier this variance reifies as.
    pub(in crate::check) fn modifier(self) -> Option<dir::VarianceModifier> {
        match self {
            Variance::Bivariant => None,
            Variance::Covariant => Some(dir::VarianceModifier::Out),
            Variance::Contravariant => Some(dir::VarianceModifier::In),
            Variance::Invariant => Some(dir::VarianceModifier::InOut),
        }
    }

    /// Return the argument relation and operand order this variance demands.
    ///
    /// The edge is `Widens` when the arguments name storage inside an
    /// existing value, and `Assignable` when a conformance query encodes
    /// call edges that convert at each use.
    pub(in crate::check) fn argument_relation(
        self,
        edge: Relation,
    ) -> Option<(Relation, OperandOrder)> {
        match self {
            Variance::Bivariant => None,
            Variance::Covariant => Some((edge, OperandOrder::Forward)),
            Variance::Contravariant => Some((edge, OperandOrder::Reversed)),
            Variance::Invariant => Some((Relation::Equal, OperandOrder::Forward)),
        }
    }
}

/// Operand order for one variance-directed argument relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum OperandOrder {
    /// Source relates to target.
    Forward,
    /// Target relates to source.
    Reversed,
}

impl OperandOrder {
    /// Order one operand pair.
    pub(in crate::check) fn orient<T>(self, source: T, target: T) -> (T, T) {
        match self {
            Self::Forward => (source, target),
            Self::Reversed => (target, source),
        }
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

/// The handle context one nominal argument relation runs under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum VarianceContext {
    /// A writable aliased handle: the managed class default.
    Aliased,
    /// An owned or copied value position.
    Owned,
    /// A readonly view or readonly borrow.
    View,
}

impl VarianceContext {
    /// Return the position one mutable storage slot measures at.
    fn storage_position(self) -> Variance {
        match self {
            // writable aliases read and write the slot
            Self::Aliased => Variance::Invariant,
            // owned copies and readonly views only read it
            Self::Owned | Self::View => Variance::Covariant,
        }
    }

    /// Return the position one aliased mutable slot measures at.
    ///
    /// Container elements and managed payloads stay reachable through
    /// other aliases even from owned copies; only a readonly view strips
    /// every write path deeply.
    fn aliased_slot(self, position: Variance) -> Variance {
        match self {
            Self::View => position,
            Self::Aliased | Self::Owned => Variance::Invariant,
        }
    }
}

/// One parameter's variance derivation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum VarianceState {
    /// The derivation is on the stack, recursive uses stay optimistic.
    Deriving,
    /// The derived variance.
    Derived(Variance),
}

impl CheckState<'_> {
    /// Return one generic parameter's variance under one handle context.
    pub(in crate::check) fn parameter_variance(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
        context: VarianceContext,
    ) -> CompilerResult<Variance> {
        // replay derived variances, recursive uses start optimistic
        match self.variances.get(&(parameter, context)) {
            Some(VarianceState::Derived(variance)) => return Ok(*variance),
            Some(VarianceState::Deriving) => return Ok(Variance::Bivariant),
            None => {}
        }

        // derive once with the entry marking the active derivation
        self.variances
            .insert((parameter, context), VarianceState::Deriving);
        let derived = self.declared_or_derived_variance(parameter, context)?;
        self.variances
            .insert((parameter, context), VarianceState::Derived(derived));

        Ok(derived)
    }

    /// Return one parameter's declared variance, deriving when unannotated.
    fn declared_or_derived_variance(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
        context: VarianceContext,
    ) -> CompilerResult<Variance> {
        let Some(binding) = self.generic_parameter(parameter) else {
            return Ok(Variance::Invariant);
        };
        let declared = binding.variance.map(Variance::from);

        // intrinsic storage assertions compose with the handle context:
        //  the compiler cannot derive an opaque payload, so the modifier
        //  is trusted as the storage direction and aliasing still caps it
        if self.parameter_owner_is_intrinsic(parameter)? {
            return Ok(match declared {
                Some(declared) => context.storage_position().compose(declared),
                None => Variance::Invariant,
            });
        }

        let Some(declared) = declared else {
            return self.derive_variance(parameter, context);
        };

        // a declared modifier binds the declaration's own handle context,
        //  where the parameter-use obligation checks it against usage;
        //  every other context derives as usual and the modifier only caps
        match self.parameter_owner_context(parameter)? {
            Some(default) if default == context => Ok(declared),
            _ => Ok(self.derive_variance(parameter, context)?.join(declared)),
        }
    }

    /// Return the default handle context of one parameter's declaration.
    pub(in crate::check) fn parameter_owner_context(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<Option<VarianceContext>> {
        let Some(binding) = self.generic_parameter(parameter) else {
            return Ok(None);
        };
        let template = binding.template.into_global(parameter.module_id);
        let Some(template) = self.generic_template(template) else {
            return Ok(None);
        };

        Ok(template
            .symbol
            .map(|symbol| self.default_symbol_context(symbol)))
    }

    /// Return whether one parameter's declaration derives a variance.
    pub(in crate::check) fn parameter_owner_is_nominal(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<bool> {
        let Some(binding) = self.generic_parameter(parameter) else {
            return Ok(false);
        };
        let template = binding.template.into_global(parameter.module_id);
        let Some(template) = self.generic_template(template) else {
            return Ok(false);
        };
        let Some(symbol) = template.symbol else {
            return Ok(false);
        };

        Ok(matches!(
            self.definition(symbol)?,
            Some(
                dir::Definition::Struct(_)
                    | dir::Definition::Class(_)
                    | dir::Definition::Enum(_)
                    | dir::Definition::Interface(_)
                    | dir::Definition::Newtype(_)
            )
        ))
    }

    /// Return whether one parameter belongs to an intrinsic newtype backing.
    fn parameter_owner_is_intrinsic(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<bool> {
        let Some(binding) = self.generic_parameter(parameter) else {
            return Ok(false);
        };
        let template = binding.template.into_global(parameter.module_id);
        let Some(template) = self.generic_template(template) else {
            return Ok(false);
        };
        let Some(symbol) = template.symbol else {
            return Ok(false);
        };
        let Some(dir::Definition::Newtype(newtype)) = self.definition(symbol)? else {
            return Ok(false);
        };
        let value = newtype.value;

        Ok(matches!(self.ty(value)?, dir::Type::Intrinsic))
    }

    /// Return the handle context one symbol's bare instances relate under.
    pub(in crate::check) fn default_symbol_context(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> VarianceContext {
        match self.symbol_kind(symbol) {
            // class and interface instances are managed handles by default
            dir::SymbolKind::Class
            | dir::SymbolKind::Interface
            | dir::SymbolKind::NewtypeInterface => VarianceContext::Aliased,
            // value instances copy or move
            _ => VarianceContext::Owned,
        }
    }

    /// Derive one parameter's variance from its uses in the declaration.
    pub(in crate::check) fn derive_variance(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
        context: VarianceContext,
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
        let Some(definition) = self.definition(symbol)?.cloned() else {
            // function templates infer per call and need no variance
            return Ok(Variance::Invariant);
        };

        // reference instances carry their constructed method instantiations,
        //  so their methods constrain every context; value instances dispatch
        //  methods statically at each use and read them covariantly
        let is_reference = matches!(
            definition,
            dir::Definition::Class(_) | dir::Definition::Interface(_)
        );

        // collect the measured member types before walking type graphs
        let mut members = SmallVec::<[_; 8]>::new();
        for member in definition.members() {
            let measured = match member {
                // storage slots measure by the handle's write capability
                dir::DefinitionMember::Field(field) => {
                    let position = if field.is_readonly {
                        Variance::Covariant
                    } else {
                        context.storage_position()
                    };

                    self.require_definition_member_type(member)?
                        .map(|ty| (ty, position))
                }
                dir::DefinitionMember::Method(_) if !is_reference => {
                    // value methods read their whole signature covariantly
                    if let Some(ty) = self.require_definition_member_type(member)? {
                        members.extend(self.value_method_positions(ty)?);
                    }

                    None
                }
                dir::DefinitionMember::Method(_) | dir::DefinitionMember::AssociatedConst(_) => {
                    self.require_definition_member_type(member)?
                        .map(|ty| (ty, Variance::Covariant))
                }
                dir::DefinitionMember::AssociatedType(associated) => associated
                    .value
                    .or(associated.constraint)
                    .map(|ty| (ty, Variance::Covariant)),
                dir::DefinitionMember::CallSignature(signature)
                | dir::DefinitionMember::ConstructSignature(signature) => {
                    Some((signature.ty, Variance::Covariant))
                }
                // index keys accept like parameters, values store like slots
                dir::DefinitionMember::IndexSignature(signature) => {
                    members.push((signature.key_type, Variance::Contravariant));

                    Some((signature.value_type, context.storage_position()))
                }
                dir::DefinitionMember::Variant(_) => None,
            };
            if let Some(measured) = measured {
                members.push(measured);
            }
        }

        // newtype backings measure like stored values
        if let dir::Definition::Newtype(newtype) = &definition {
            members.push((newtype.value, context.storage_position()));
        }

        let heritages = definition
            .heritages()
            .iter()
            .map(|heritage| (heritage.symbol, heritage.arguments.clone()))
            .collect::<SmallVec<[_; 2]>>();

        // measure every member occurrence
        let mut measured = Variance::Bivariant;
        for (ty, position) in members {
            measured = measured.join(self.measure_type(ty, position, context, parameter)?);
        }

        // measure heritage arguments under the base parameter positions
        for (base, arguments) in heritages {
            measured = measured.join(self.measure_application(
                base,
                &arguments,
                Variance::Covariant,
                context,
                parameter,
            )?);
        }

        Ok(measured)
    }

    /// Collect one value method's positions, reading the signature covariantly.
    fn value_method_positions(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[(dir::GlobalTypeId, Variance); 4]>> {
        let mut positions = SmallVec::new();
        let Some(signature) = self.signature_head(ty)? else {
            positions.push((ty, Variance::Covariant));

            return Ok(positions);
        };

        // statically dispatched calls type against the copy, not the origin
        let parameters = self
            .signature_parameters(ty.module_id, signature.parameters)?
            .to_vec();
        positions.extend(
            parameters
                .iter()
                .map(|parameter| (parameter.ty, Variance::Covariant)),
        );
        positions.extend(
            signature
                .return_type
                .map(|return_type| (return_type, Variance::Covariant)),
        );

        Ok(positions)
    }

    /// Measure one parameter's occurrences in one type graph.
    fn measure_type(
        &mut self,
        ty: dir::GlobalTypeId,
        position: Variance,
        context: VarianceContext,
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
                let function = self.type_signature(ty.module_id, function)?;
                let mut measured = Variance::Bivariant;
                if let Some(this_parameter) = function.this_parameter {
                    measured = measured.join(self.measure_type(
                        this_parameter,
                        position.flip(),
                        context,
                        parameter,
                    )?);
                }
                let inputs = self
                    .signature_parameters(ty.module_id, function.parameters)?
                    .to_vec();
                for input in inputs {
                    measured = measured.join(self.measure_type(
                        input.ty,
                        position.flip(),
                        context,
                        parameter,
                    )?);
                }
                if let Some(return_type) = function.return_type {
                    measured = measured.join(self.measure_type(
                        return_type,
                        position,
                        context,
                        parameter,
                    )?);
                }

                measured
            }

            // function carriers measure through their wrapped signatures
            dir::Type::Function(function) => {
                let signature =
                    self.measure_type(function.signature, position, context, parameter)?;
                let environment = self.measure_type(
                    function.environment,
                    Variance::Invariant,
                    context,
                    parameter,
                )?;

                signature.join(environment)
            }
            dir::Type::FunctionPointer(pointer) => {
                self.measure_type(pointer.signature, position, context, parameter)?
            }

            // applications compose with the base parameter variances
            dir::Type::Instance(instance) => {
                let arguments = self.type_ids(ty.module_id, instance.arguments)?.to_vec();

                self.measure_application(instance.symbol, &arguments, position, context, parameter)?
            }

            // mutable container storage follows the handle: readonly views
            //  read elements covariantly, every other handle writes them
            dir::Type::Array(array) => self.measure_type(
                array.element,
                context.aliased_slot(position),
                context,
                parameter,
            )?,
            dir::Type::Slice(slice) => self.measure_type(
                slice.element,
                context.aliased_slot(position),
                context,
                parameter,
            )?,

            // value containers keep their position
            dir::Type::FixedArray(array) => {
                self.measure_type(array.element, position, context, parameter)?
            }
            dir::Type::Tuple(tuple) => {
                let elements = self.tuple_elements(ty.module_id, tuple.elements)?.to_vec();
                let mut measured = Variance::Bivariant;
                for element in elements {
                    measured =
                        measured.join(self.measure_type(element.ty, position, context, parameter)?);
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
                    measured = measured.join(self.measure_type(
                        field.ty,
                        field_position,
                        context,
                        parameter,
                    )?);
                }
                let signatures = self
                    .type_ids(ty.module_id, shape.call_signatures)?
                    .iter()
                    .chain(self.type_ids(ty.module_id, shape.construct_signatures)?)
                    .copied()
                    .collect::<SmallVec<[_; 4]>>();
                for signature in signatures {
                    measured =
                        measured.join(self.measure_type(signature, position, context, parameter)?);
                }
                let index_signatures = self
                    .shape_index_signatures(ty.module_id, shape.index_signatures)?
                    .to_vec();
                for signature in index_signatures {
                    measured = measured.join(self.measure_type(
                        signature.value_type,
                        context.aliased_slot(position),
                        context,
                        parameter,
                    )?);
                }

                measured
            }

            // memory forms follow their aliasing behavior
            dir::Type::Form(form) => match form.form {
                // readonly views and owned payloads keep their position
                dir::Form::Readonly | dir::Form::Owned | dir::Form::Placed { .. } => {
                    self.measure_type(form.value, position, context, parameter)?
                }
                // managed payloads are storage the handle reaches
                dir::Form::Managed => self.measure_type(
                    form.value,
                    context.aliased_slot(position),
                    context,
                    parameter,
                )?,
                // borrows and raw pointers alias mutably in every context
                dir::Form::Borrowed(_) | dir::Form::Raw => {
                    self.measure_type(form.value, Variance::Invariant, context, parameter)?
                }
            },

            // algebraic composites keep their position
            dir::Type::Union(dir::UnionType { elements })
            | dir::Type::Intersection(dir::IntersectionType { elements }) => {
                let elements = self.type_ids(ty.module_id, elements)?.to_vec();
                let mut measured = Variance::Bivariant;
                for element in elements {
                    measured =
                        measured.join(self.measure_type(element, position, context, parameter)?);
                }

                measured
            }

            // projections and operations are unmeasurable
            dir::Type::Member(member) => {
                let member = self.type_member(ty.module_id, member)?;
                let mut measured =
                    self.measure_type(member.owner, Variance::Invariant, context, parameter)?;
                let arguments = self.type_ids(ty.module_id, member.arguments)?.to_vec();
                for argument in arguments {
                    measured = measured.join(self.measure_type(
                        argument,
                        Variance::Invariant,
                        context,
                        parameter,
                    )?);
                }

                measured
            }
            head @ (dir::Type::Operation(_) | dir::Type::Dynamic(_)) => {
                let mut children = SmallVec::<[dir::GlobalTypeId; 8]>::new();
                self.for_each_type_child(ty.module_id, &head, |child| children.push(child))?;
                let mut measured = Variance::Bivariant;
                for child in children {
                    measured = measured.join(self.measure_type(
                        child,
                        Variance::Invariant,
                        context,
                        parameter,
                    )?);
                }

                measured
            }

            // leaves carry no occurrences
            _ => Variance::Bivariant,
        };

        Ok(measured)
    }

    /// Decide same-template type arguments under one handle context.
    pub(in crate::check) fn decide_type_arguments(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        context: VarianceContext,
        edge: Relation,
        source: &[dir::GlobalTypeId],
        target: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        if source.len() != target.len() {
            return Ok(Answer::Ready(false));
        }
        let edge = self.instance_argument_edge(symbol, edge);
        let mut decision = Answer::Ready(true);
        for (index, (source, target)) in source.iter().zip(target.iter()).enumerate() {
            // bivariant arguments relate freely under a closed judgment
            let Some((relation, order)) = self
                .argument_variance(symbol, index, context)?
                .argument_relation(edge)
            else {
                continue;
            };
            let (source, target) = order.orient(*source, *target);

            decision = decision.and(self.decide_relation(origin, relation, source, target)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Relate same-template type arguments under one handle context.
    pub(in crate::check) fn relate_type_arguments(
        &mut self,
        cause: CauseId,
        symbol: dir::GlobalSymbolId,
        context: VarianceContext,
        edge: Relation,
        source: &[dir::GlobalTypeId],
        target: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        let origin = self.cause_origin(cause);
        let edge = self.instance_argument_edge(symbol, edge);
        let mut decision = Answer::Ready(true);
        for (index, (source, target)) in source.iter().zip(target.iter()).enumerate() {
            let variance = self.argument_variance(symbol, index, context)?;
            let slot = CauseKind::TypeArgument {
                symbol,
                index: index as u32,
                variance,
            };
            let child = self.intern_cause(Cause::slot(origin, slot, cause));
            let answer = match variance.argument_relation(edge) {
                // bivariant arguments still constrain open holes so inference closes
                None => {
                    if self.type_flags(*source)?.has_variable()
                        || self.type_flags(*target)?.has_variable()
                    {
                        self.constrain_type(child, Relation::Equal, *source, *target)?
                    } else {
                        Answer::Ready(true)
                    }
                }
                Some((relation, order)) => {
                    let (source, target) = order.orient(*source, *target);

                    self.constrain_type(child, relation, source, target)?
                }
            };
            decision = decision.and(answer);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Return the argument edge one instance symbol relates by.
    ///
    /// Interface instances are dynamic carriers pending the dynamic-safe
    /// wiring, so their arguments keep conformance edges until vtable
    /// construction enforces identity there.
    pub(in crate::check) fn instance_argument_edge(
        &self,
        symbol: dir::GlobalSymbolId,
        edge: Relation,
    ) -> Relation {
        match self.symbol_kind(symbol) {
            dir::SymbolKind::Interface | dir::SymbolKind::NewtypeInterface
                if edge == Relation::Widens =>
            {
                Relation::Assignable
            }
            _ => edge,
        }
    }

    /// Return one indexed argument's variance, invariant when unknown.
    pub(in crate::check) fn argument_variance(
        &mut self,
        symbol: dir::GlobalSymbolId,
        index: usize,
        context: VarianceContext,
    ) -> CompilerResult<Variance> {
        let parameter = self
            .symbol_template(symbol)?
            .map(|template| self.generic_template_parameters(template))
            .and_then(|parameters| parameters.get(index).copied());

        match parameter {
            Some(parameter) => self.parameter_variance(parameter, context),
            None => Ok(Variance::Invariant),
        }
    }

    /// Measure one application's arguments under the base variances.
    fn measure_application(
        &mut self,
        base: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
        position: Variance,
        context: VarianceContext,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<Variance> {
        let parameters = self
            .symbol_template(base)?
            .map(|template| self.generic_template_parameters(template));

        // readonly views stay views deeply, other handles reset per symbol
        let base_context = match context {
            VarianceContext::View => VarianceContext::View,
            _ => self.default_symbol_context(base),
        };

        let mut measured = Variance::Bivariant;
        for (index, argument) in arguments.iter().enumerate() {
            // unknown base templates measure conservatively
            let argument_position = match &parameters {
                Some(parameters) => match parameters.get(index) {
                    Some(base_parameter) => {
                        // occurrences compose under the base's handle
                        let base_variance =
                            self.parameter_variance(*base_parameter, base_context)?;

                        position.compose(base_variance)
                    }
                    None => Variance::Invariant,
                },
                None => Variance::Invariant,
            };
            measured = measured.join(self.measure_type(
                *argument,
                argument_position,
                base_context,
                parameter,
            )?);
        }

        Ok(measured)
    }
}
