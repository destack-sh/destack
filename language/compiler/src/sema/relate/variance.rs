use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{Cause, CauseId, CauseKind, CheckState, Origin, Relation, Verdict};
use crate::{CompilerError, CompilerResult};

/// One derived generic parameter variance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum Variance {
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
    pub(in crate::sema) fn compose(self, inner: Variance) -> Variance {
        match (self, inner) {
            (Variance::Bivariant, _) | (_, Variance::Bivariant) => Variance::Bivariant,
            (Variance::Invariant, _) | (_, Variance::Invariant) => Variance::Invariant,
            (own, other) if own == other => Variance::Covariant,
            _ => Variance::Contravariant,
        }
    }

    /// Flip into the contravariant position.
    pub(in crate::sema) fn flip(self) -> Variance {
        Variance::Contravariant.compose(self)
    }

    /// Return whether this declared variance admits one derived use.
    pub(in crate::sema) fn admits(self, derived: Variance) -> bool {
        matches!(derived, Variance::Bivariant) || self == Variance::Invariant || self == derived
    }

    /// Return the written modifier this variance reifies as.
    pub(in crate::sema) fn modifier(self) -> Option<dir::VarianceModifier> {
        match self {
            Variance::Bivariant => None,
            Variance::Covariant => Some(dir::VarianceModifier::Out),
            Variance::Contravariant => Some(dir::VarianceModifier::In),
            Variance::Invariant => Some(dir::VarianceModifier::InOut),
        }
    }

    /// Return the argument relation and operand order this variance demands.
    ///
    /// The relation is `Widens` when the arguments name storage inside an
    /// existing value, and `Assignable` when a conformance query encodes
    /// call edges that convert at each use.
    pub(in crate::sema) fn argument_relation(
        self,
        relation: Relation,
    ) -> Option<(Relation, OperandOrder)> {
        match self {
            Variance::Bivariant => None,
            Variance::Covariant => Some((relation, OperandOrder::Forward)),
            Variance::Contravariant => Some((relation, OperandOrder::Reversed)),
            Variance::Invariant => Some((Relation::Equal, OperandOrder::Forward)),
        }
    }
}

/// Operand order for one variance-directed argument relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum OperandOrder {
    /// Source relates to target.
    Forward,
    /// Target relates to source.
    Reversed,
}

impl OperandOrder {
    /// Order one operand pair.
    pub(in crate::sema) fn orient<T>(self, source: T, target: T) -> (T, T) {
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

/// The handle form through which a generic value is related.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum VarianceForm {
    /// A managed reference to the value.
    Managed,
    /// Exclusive ownership of the value.
    Owned,
    /// A deeply readonly view of the value.
    Readonly,
}

impl VarianceForm {
    /// Return one direct storage field's variance.
    fn field(self) -> Variance {
        match self {
            Self::Managed => Variance::Invariant,
            Self::Owned | Self::Readonly => Variance::Covariant,
        }
    }

    /// Return one independently aliased storage position's variance.
    fn aliased(self, position: Variance) -> Variance {
        match self {
            Self::Readonly => position,
            Self::Managed | Self::Owned => Variance::Invariant,
        }
    }
}

/// One parameter's variance derivation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum VarianceState {
    /// The derivation is on the stack, recursive uses stay optimistic.
    Deriving,
    /// The derived variance.
    Derived(Variance),
}

impl CheckState<'_> {
    /// Return one generic parameter's variance through a handle form.
    pub(in crate::sema) fn parameter_variance(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
        form: VarianceForm,
    ) -> CompilerResult<Variance> {
        // replay derived variances, recursive uses start optimistic
        match self.variances.get(&(parameter, form)) {
            Some(VarianceState::Derived(variance)) => return Ok(*variance),
            Some(VarianceState::Deriving) => return Ok(Variance::Bivariant),
            None => {}
        }

        // derive once with the entry marking the active derivation
        self.variances
            .insert((parameter, form), VarianceState::Deriving);
        let derived = self.declared_or_derived_variance(parameter, form)?;
        self.variances
            .insert((parameter, form), VarianceState::Derived(derived));

        Ok(derived)
    }

    /// Return one parameter's declared variance, deriving when unannotated.
    fn declared_or_derived_variance(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
        form: VarianceForm,
    ) -> CompilerResult<Variance> {
        let Some(binding) = self.generic_parameter(parameter) else {
            return Ok(Variance::Invariant);
        };
        let declared = binding.variance.map(Variance::from);

        // intrinsic backings trust their declared storage direction
        if self.parameter_owner_is_intrinsic(parameter)? {
            return Ok(declared
                .map(|declared| form.field().compose(declared))
                .unwrap_or(Variance::Invariant));
        }

        let derived = self.derive_variance(parameter, form)?;
        let variance = match declared {
            Some(declared) if self.parameter_variance_form(parameter)? == form => declared,
            Some(declared) => derived.join(declared),
            None => derived,
        };

        Ok(variance)
    }

    /// Return the default handle form of one parameter's declaration.
    pub(in crate::sema) fn parameter_variance_form(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<VarianceForm> {
        let Some(binding) = self.generic_parameter(parameter) else {
            return Ok(VarianceForm::Owned);
        };
        let template = binding.template.into_global(parameter.module_id);
        let Some(template) = self.generic_template(template) else {
            return Ok(VarianceForm::Owned);
        };

        match template.symbol {
            Some(symbol) => self.default_variance_form(symbol),
            None => Ok(VarianceForm::Owned),
        }
    }

    /// Return the default handle form of one nominal declaration.
    pub(in crate::sema) fn default_variance_form(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<VarianceForm> {
        // default foreign parameters to the owned form while
        //  declaring, checking derives the real form
        let Some(kind) = self.symbol_kind_maybe(symbol)? else {
            return Ok(VarianceForm::Owned);
        };

        let form = match kind {
            dir::SymbolKind::Class
            | dir::SymbolKind::Interface
            | dir::SymbolKind::NewtypeInterface => VarianceForm::Managed,
            _ => VarianceForm::Owned,
        };

        Ok(form)
    }

    /// Return whether one parameter's declaration derives a variance.
    pub(in crate::sema) fn parameter_owner_is_nominal(
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
        let value = newtype.backing;

        Ok(matches!(self.ty(value)?, dir::Type::Intrinsic))
    }

    /// Derive one parameter's variance from its uses in the declaration.
    pub(in crate::sema) fn derive_variance(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
        form: VarianceForm,
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

        // measure class and interface methods as reference instances,
        //  and every other declaration's methods as value instances
        let is_reference = matches!(
            definition,
            dir::Definition::Class(_) | dir::Definition::Interface(_)
        );
        let storage = form.field();

        // collect the measured member types before walking type graphs
        let mut members = SmallVec::<[_; 8]>::new();
        for member in definition.members() {
            let measured = match member {
                // storage slots measure by the handle's write capability
                dir::DefinitionMember::Field(field) => {
                    let position = match field.is_readonly {
                        true => Variance::Covariant,
                        false => storage,
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

                    Some((signature.value_type, storage))
                }
                dir::DefinitionMember::EnumVariant(_) => None,
            };
            if let Some(measured) = measured {
                members.push(measured);
            }
        }

        // newtype backings measure like stored values
        if let dir::Definition::Newtype(newtype) = &definition {
            members.push((newtype.backing, storage));
        }

        let mut heritages = definition
            .bases()
            .iter()
            .map(|heritage| heritage.ty)
            .collect::<SmallVec<[_; 2]>>();
        heritages.extend(
            definition
                .implementations()
                .iter()
                .map(|conformance| conformance.interface),
        );

        // measure every member occurrence
        let mut measured = Variance::Bivariant;
        for (ty, position) in members {
            measured = measured.join(self.measure_type(ty, position, form, parameter)?);
        }

        // measure complete heritage types covariantly
        for heritage in heritages {
            measured =
                measured.join(self.measure_type(heritage, Variance::Covariant, form, parameter)?);
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
        let parameters: SmallVec<[_; 4]> = self
            .signature_parameters(ty.module_id, signature.parameters)?
            .into();
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

    /// Return whether one borrow form's solved access is readonly.
    fn borrow_is_readonly(
        &mut self,
        module: ModuleId,
        borrow: dir::BorrowFormId,
    ) -> CompilerResult<bool> {
        let access = self.type_borrow(module, borrow)?.access;
        let is_readonly = matches!(
            self.ty(access)?,
            dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Readonly))
        );

        Ok(is_readonly)
    }

    /// Measure one parameter's occurrences in one type graph.
    fn measure_type(
        &mut self,
        ty: dir::GlobalTypeId,
        position: Variance,
        form: VarianceForm,
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
                        form,
                        parameter,
                    )?);
                }
                let inputs: SmallVec<[_; 4]> = self
                    .signature_parameters(ty.module_id, function.parameters)?
                    .into();
                for input in inputs {
                    measured = measured.join(self.measure_type(
                        input.ty,
                        position.flip(),
                        form,
                        parameter,
                    )?);
                }
                if let Some(return_type) = function.return_type {
                    measured =
                        measured.join(self.measure_type(return_type, position, form, parameter)?);
                }

                measured
            }

            // measure functions through their signature
            dir::Type::Function(function) => {
                self.measure_type(function.signature, position, form, parameter)?
            }
            dir::Type::FunctionPointer(pointer) => {
                self.measure_type(pointer.signature, position, form, parameter)?
            }

            // applications compose with the base parameter variances
            dir::Type::Application(instance) => {
                let arguments: SmallVec<[_; 8]> =
                    self.type_ids(ty.module_id, instance.arguments)?.into();

                self.measure_application(instance.symbol, &arguments, position, form, parameter)?
            }

            // independently aliased storage remains writable outside owned values
            dir::Type::Slice(slice) => {
                self.measure_type(slice.element, form.aliased(position), form, parameter)?
            }

            // value containers keep their position
            dir::Type::FixedArray(array) => {
                self.measure_type(array.element, position, form, parameter)?
            }
            dir::Type::Tuple(tuple) => {
                let elements: SmallVec<[_; 4]> =
                    self.tuple_elements(ty.module_id, tuple.elements)?.into();
                let mut measured = Variance::Bivariant;
                for element in elements {
                    measured =
                        measured.join(self.measure_type(element.ty, position, form, parameter)?);
                }

                measured
            }

            // structural shapes measure reads forward and writes backward
            dir::Type::Object(shape) => {
                let fields: SmallVec<[_; 4]> = self
                    .shape_properties(ty.module_id, shape.properties)?
                    .into();
                let mut measured = Variance::Bivariant;
                for field in fields {
                    if let Some(read) = field.access.read() {
                        measured =
                            measured.join(self.measure_type(read, position, form, parameter)?);
                    }
                    if let Some(write) = field.access.write() {
                        measured = measured.join(self.measure_type(
                            write,
                            position.flip(),
                            form,
                            parameter,
                        )?);
                    }
                }
                let signatures = self
                    .type_ids(ty.module_id, shape.call_signatures)?
                    .iter()
                    .chain(self.type_ids(ty.module_id, shape.construct_signatures)?)
                    .copied()
                    .collect::<SmallVec<[_; 4]>>();
                for signature in signatures {
                    measured =
                        measured.join(self.measure_type(signature, position, form, parameter)?);
                }
                let index_signatures: SmallVec<[_; 4]> = self
                    .shape_index_signatures(ty.module_id, shape.index_signatures)?
                    .into();
                for signature in index_signatures {
                    measured = measured.join(self.measure_type(
                        signature.value_type,
                        form.aliased(position),
                        form,
                        parameter,
                    )?);
                }

                measured
            }

            // nested memory forms contribute their own access capability
            dir::Type::Form(type_form) => match type_form.form {
                // transparent value forms keep the surrounding capability
                dir::Form::Readonly | dir::Form::Owned | dir::Form::Placed { .. } => {
                    self.measure_type(type_form.value, position, form, parameter)?
                }
                // readonly borrows view their pointee without write-back
                dir::Form::Borrowed(borrow) if self.borrow_is_readonly(ty.module_id, borrow)? => {
                    self.measure_type(type_form.value, position, VarianceForm::Readonly, parameter)?
                }
                // independently writable references stay invariant unless deeply readonly
                dir::Form::Managed | dir::Form::Borrowed(_) | dir::Form::Raw => {
                    self.measure_type(type_form.value, form.aliased(position), form, parameter)?
                }
            },

            // algebraic composites keep their position
            dir::Type::Union(dir::UnionType { elements })
            | dir::Type::Intersection(dir::IntersectionType { elements }) => {
                let elements: SmallVec<[_; 8]> = self.type_ids(ty.module_id, elements)?.into();
                let mut measured = Variance::Bivariant;
                for element in elements {
                    measured =
                        measured.join(self.measure_type(element, position, form, parameter)?);
                }

                measured
            }

            // projections and operations are unmeasurable
            dir::Type::Member(member) => {
                let member = self.type_member(ty.module_id, member)?;
                let mut measured =
                    self.measure_type(member.owner, Variance::Invariant, form, parameter)?;
                let arguments: SmallVec<[_; 8]> =
                    self.type_ids(ty.module_id, member.arguments)?.into();
                for argument in arguments {
                    measured = measured.join(self.measure_type(
                        argument,
                        Variance::Invariant,
                        form,
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
                        form,
                        parameter,
                    )?);
                }

                measured
            }

            // stop at leaves, they have no occurrences
            _ => Variance::Bivariant,
        };

        Ok(measured)
    }

    /// Relate same-template applications, slotting written arguments by kind first.
    pub(in crate::sema) fn relate_application_arguments(
        &mut self,
        origin: Origin,
        cause: CauseId,
        symbol: dir::GlobalSymbolId,
        form: VarianceForm,
        relation: Relation,
        source: &[dir::GlobalTypeId],
        target: &[dir::GlobalTypeId],
    ) -> CompilerResult<Verdict> {
        // pair the written arguments by kind, collecting elided lifetimes for proof only
        let Some(slots) = self.slot_application_arguments(source, target)? else {
            return Ok(Verdict::Fails);
        };
        let (source, target): (SmallVec<[_; 4]>, SmallVec<[_; 4]>) = slots.iter().copied().unzip();

        self.relate_type_arguments(origin, cause, symbol, form, relation, &source, &target)
    }

    /// Relate same-template type arguments by their parameter variances.
    pub(in crate::sema) fn relate_type_arguments(
        &mut self,
        origin: Origin,
        cause: CauseId,
        symbol: dir::GlobalSymbolId,
        form: VarianceForm,
        relation: Relation,
        source: &[dir::GlobalTypeId],
        target: &[dir::GlobalTypeId],
    ) -> CompilerResult<Verdict> {
        if source.len() != target.len() {
            return Ok(Verdict::Fails);
        }

        let relation = self.instance_argument_relation(symbol, relation)?;
        let mut verdict = Verdict::Holds;
        for (index, (source, target)) in source.iter().zip(target.iter()).enumerate() {
            // read both arguments through their solutions
            let source = self.shallow_resolve(*source)?;
            let target = self.shallow_resolve(*target)?;

            // erased target arguments admit every instantiation of their parameter
            if matches!(self.ty(target)?, dir::Type::Erased(_)) {
                continue;
            }

            // skip closed lifetime slots for Verify, still linking open ones
            if !self.type_flags(source)?.has_variable()
                && !self.type_flags(target)?.has_variable()
                && self.is_lifetime_slot_type(source)?
                && self.is_lifetime_slot_type(target)?
            {
                continue;
            }
            let variance = self.argument_variance(symbol, index, form)?;
            let slot = CauseKind::TypeArgument {
                symbol,
                index: index as u32,
                variance,
            };
            let child = self.intern_cause(Cause::child(origin, slot, cause));
            let related = match variance.argument_relation(relation) {
                // bivariant arguments still constrain open holes so inference closes
                None => {
                    if self.type_flags(source)?.has_variable()
                        || self.type_flags(target)?.has_variable()
                    {
                        self.constrain_type(origin, child, Relation::Equal, source, target)?
                    } else {
                        Verdict::Holds
                    }
                }
                Some((relation, order)) => {
                    let (source, target) = order.orient(source, target);

                    self.constrain_type(origin, child, relation, source, target)?
                }
            };
            verdict = verdict.and(related);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Return the relation used by one instance symbol's arguments.
    pub(in crate::sema) fn instance_argument_relation(
        &mut self,
        symbol: dir::GlobalSymbolId,
        relation: Relation,
    ) -> CompilerResult<Relation> {
        let relation = match self.symbol_kind_maybe(symbol)? {
            // widen interface applications by assignability
            Some(dir::SymbolKind::Interface | dir::SymbolKind::NewtypeInterface)
                if relation == Relation::Widens =>
            {
                Relation::Assignable
            }
            _ => relation,
        };

        Ok(relation)
    }

    /// Return one indexed argument's variance, invariant when unknown.
    pub(in crate::sema) fn argument_variance(
        &mut self,
        symbol: dir::GlobalSymbolId,
        index: usize,
        form: VarianceForm,
    ) -> CompilerResult<Variance> {
        let parameters = match self.symbol_template(symbol)? {
            Some(template) => Some(self.generic_template_parameters(template)?),
            None => None,
        };
        let parameter = parameters
            .as_ref()
            .and_then(|parameters| parameters.get(index).copied());

        match parameter {
            Some(parameter) => self.parameter_variance(parameter, form),
            None => Ok(Variance::Invariant),
        }
    }

    /// Measure one application's arguments under the base variances.
    fn measure_application(
        &mut self,
        base: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
        position: Variance,
        form: VarianceForm,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<Variance> {
        let parameters = match self.symbol_template(base)? {
            Some(template) => Some(self.generic_template_parameters(template)?),
            None => None,
        };

        let mut measured = Variance::Bivariant;
        for (index, argument) in arguments.iter().enumerate() {
            // unknown base templates measure conservatively
            let argument_position = match &parameters {
                Some(parameters) => match parameters.get(index) {
                    Some(base_parameter) => {
                        // occurrences compose through the applied constructor
                        let base_variance = self.parameter_variance(*base_parameter, form)?;

                        position.compose(base_variance)
                    }
                    None => Variance::Invariant,
                },
                None => Variance::Invariant,
            };
            measured =
                measured.join(self.measure_type(*argument, argument_position, form, parameter)?);
        }

        Ok(measured)
    }

    /// Derive the parameter variances of every definition in one module.
    pub(in crate::sema) fn derive_module_variances(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        // derive each declared definition parameter at its own context
        let symbols = self
            .module(module)
            .iter_definitions()
            .map(|(symbol, _)| symbol)
            .collect::<Vec<_>>();
        let mut parameters = Vec::new();
        for symbol in symbols {
            let Some(template) = self.symbol_template(symbol)? else {
                continue;
            };
            parameters.extend(self.generic_template_parameters(template)?);
        }

        // keep only unannotated nominal type parameters
        let mut filled = Vec::new();
        for parameter in parameters {
            let form = self.parameter_variance_form(parameter)?;
            let derived = self.parameter_variance(parameter, form)?;
            let Some(binding) = self.generic_parameter(parameter) else {
                continue;
            };
            if binding.variance.is_some()
                || binding.origin != dir::GenericParameterOrigin::Explicit
                || binding.memory_parameter().is_some()
                || binding.is_const
            {
                continue;
            }
            if !self.parameter_owner_is_nominal(parameter)? {
                continue;
            }
            let Some(modifier) = derived.modifier() else {
                continue;
            };
            filled.push((parameter.local_id, modifier));
        }

        // record the derivations on the checked tail
        for (parameter, modifier) in filled {
            self.module_mut(module)
                .generics_tail
                .set_variance(parameter, modifier);
        }

        Ok(())
    }

    /// Record the cardinalities native implementations impose on one module.
    pub(in crate::sema) fn derive_native_cardinalities(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        // native implementations consume their value parameters directly
        let symbols = self
            .module(module)
            .iter_definitions()
            .map(|(symbol, _)| symbol)
            .collect::<Vec<_>>();
        let mut native = Vec::new();
        for symbol in symbols {
            let Some(template) = self.symbol_template(symbol)? else {
                continue;
            };
            let state = self.module(module);
            let Some(node) = state.bindings.get_symbol(symbol.local_id).declaration else {
                continue;
            };
            let is_native = state
                .decorators_tail
                .applications_for_owner(node)
                .any(|application| {
                    matches!(
                        application.resolution.target,
                        dir::DecoratorTarget::LanguageItem {
                            item: dir::LanguageItem::Intrinsic | dir::LanguageItem::Binding,
                            ..
                        }
                    )
                });
            if !is_native {
                continue;
            }

            for parameter in self.generic_template_parameters(template)? {
                let Some(binding) = self.generic_parameter(parameter) else {
                    continue;
                };
                if binding.is_const && binding.memory_parameter().is_none() {
                    native.push((parameter.local_id, node));
                }
            }
        }
        for (parameter, node) in native {
            self.module_mut(module).generics_tail.set_cardinality(
                parameter,
                dir::Cardinality::One {
                    source: node.local_id,
                },
            );
        }

        Ok(())
    }

    /// Return the One cardinality one parameter resolves to, if any.
    pub(in crate::sema) fn recorded_cardinality(
        &self,
        parameter: dir::GlobalGenericParameterId,
    ) -> Option<dir::Cardinality> {
        let mut visited = FxIndexSet::default();
        let mut current = parameter;

        // follow Of links to the recorded One
        while visited.insert(current) {
            match self.parameter_cardinality(current)? {
                one @ dir::Cardinality::One { .. } => return Some(one),
                dir::Cardinality::Of { callee } => current = callee,
            }
        }

        None
    }

    /// Return the cardinality one parameter's segments record.
    fn parameter_cardinality(
        &self,
        parameter: dir::GlobalGenericParameterId,
    ) -> Option<dir::Cardinality> {
        let state = self.module_maybe(parameter.module_id)?;
        if let Some(cardinality) = state.generics_tail.cardinality(parameter.local_id) {
            return Some(cardinality);
        }
        if let Some(elaborated) = &state.elaborated
            && let Some(cardinality) = elaborated.generics.cardinality(parameter.local_id)
        {
            return Some(cardinality);
        }
        if let Some(declared) = &state.declared
            && let Some(cardinality) = declared.generics.cardinality(parameter.local_id)
        {
            return Some(cardinality);
        }

        None
    }

    /// Return whether one type satisfies a One cardinality demand.
    pub(in crate::sema) fn type_satisfies_one_cardinality(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // expose the value domain behind computation heads before judging
        let ty = self.normalize_computation(origin, ty)?;
        let satisfies = match self.ty(ty)? {
            // literals and errors stand for one value
            dir::Type::Literal(_) | dir::Type::Error => true,
            // bare enum members carry one discriminant value
            dir::Type::Variant(_) => true,
            // reject unsettled variables loudly
            dir::Type::Variable(_) => {
                return Err(CompilerError::Internal {
                    message: format!("unsettled variable at {origin:?}"),
                });
            }
            // rigid parameters carry their own recorded cardinality
            dir::Type::Parameter(parameter) => {
                self.recorded_cardinality(parameter).is_some()
                    || self
                        .generic_parameter(parameter)
                        .is_some_and(|binding| binding.memory_parameter().is_some())
            }
            // static operations over exact operands compute one exact value
            dir::Type::Operation(operation) => {
                match self.type_operation(ty.module_id, operation)? {
                    dir::TypeOperation::StaticBinary(binary) => {
                        self.type_satisfies_one_cardinality(origin, binary.left)?
                            && self.type_satisfies_one_cardinality(origin, binary.right)?
                    }
                    dir::TypeOperation::StaticUnary(unary) => {
                        self.type_satisfies_one_cardinality(origin, unary.target)?
                    }
                    // infer binders match the one value the scrutinee fixed
                    dir::TypeOperation::Infer(_) => true,
                    _ => false,
                }
            }
            _ => false,
        };

        Ok(satisfies)
    }

    /// Return the derived variance recorded for one parameter, if any.
    pub(in crate::sema) fn recorded_variance(
        &self,
        module: ModuleId,
        parameter: dir::LocalGenericParameterId,
    ) -> Option<dir::VarianceModifier> {
        let state = self.module_maybe(module)?;
        if let Some(modifier) = state.generics_tail.variance(parameter) {
            return Some(modifier);
        }
        if let Some(elaborated) = &state.elaborated
            && let Some(modifier) = elaborated.generics.variance(parameter)
        {
            return Some(modifier);
        }

        None
    }
}
