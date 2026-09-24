use std::sync::Arc;

use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    Cause, CauseId, CauseKind, CheckState, GenericParameterId, Origin, Relation, Verdict,
};
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
            (variance, Variance::Bivariant) => variance,
            (variance, other) if variance == other => variance,
            _ => Variance::Invariant,
        }
    }

    /// Compose one occurrence position with the position beneath it.
    pub(in crate::sema) fn compose(self, nested: Variance) -> Variance {
        match (self, nested) {
            (Variance::Bivariant, _) | (_, Variance::Bivariant) => Variance::Bivariant,
            (Variance::Invariant, _) | (_, Variance::Invariant) => Variance::Invariant,
            (variance, other) if variance == other => Variance::Covariant,
            _ => Variance::Contravariant,
        }
    }

    /// Flip into the contravariant position.
    pub(in crate::sema) fn flip(self) -> Variance {
        Variance::Contravariant.compose(self)
    }

    /// Return whether this declared variance is compatible with one derived use.
    pub(in crate::sema) fn is_compatible_with(self, derived: Variance) -> bool {
        matches!(derived, Variance::Bivariant) || self == Variance::Invariant || self == derived
    }

    /// Return the written modifier for this variance.
    pub(in crate::sema) fn modifier(self) -> Option<dir::VarianceModifier> {
        match self {
            Variance::Bivariant => None,
            Variance::Covariant => Some(dir::VarianceModifier::Out),
            Variance::Contravariant => Some(dir::VarianceModifier::In),
            Variance::Invariant => Some(dir::VarianceModifier::InOut),
        }
    }

    /// Return the argument relation and operand order this variance requires.
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

impl<'a> CheckState<'a> {
    /// Return one generic parameter's variance through a handle form.
    pub(in crate::sema) fn parameter_variance(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
        form: VarianceForm,
    ) -> CompilerResult<Variance> {
        // reuse derived variances, recursive uses start optimistic
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
        let Some(binding) = self.generic_parameter(parameter)? else {
            return Ok(Variance::Invariant);
        };
        let declared = binding.variance.map(Variance::from);

        // intrinsic backings trust their declared storage direction
        if self.parameter_owner_is_intrinsic(parameter)? {
            return Ok(declared
                .map(|declared| form.field().compose(declared))
                .unwrap_or(Variance::Invariant));
        }

        // join the declared variance with the derived one
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
        let Some(binding) = self.generic_parameter(parameter)? else {
            return Ok(VarianceForm::Owned);
        };
        let template = binding.template.into_global(parameter.module_id);
        let Some(template) = self.generic_template(template)? else {
            return Ok(VarianceForm::Owned);
        };

        // read the form the owning declaration defaults to
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
        // NOTE #Suspicious: the kind read always answers here, surface wrong declare forms
        let kind = self.symbol_kind(symbol)?;

        // read the form each declaration kind defaults to
        let form = match kind {
            dir::SymbolKind::Class
            | dir::SymbolKind::Interface
            | dir::SymbolKind::NewtypeInterface => VarianceForm::Managed,
            _ => VarianceForm::Owned,
        };

        Ok(form)
    }

    /// Return whether one parameter belongs to a nominal declaration.
    pub(in crate::sema) fn parameter_owner_is_nominal(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<bool> {
        let Some(binding) = self.generic_parameter(parameter)? else {
            return Ok(false);
        };
        let template = binding.template.into_global(parameter.module_id);
        let Some(template) = self.generic_template(template)? else {
            return Ok(false);
        };
        let Some(symbol) = template.symbol else {
            return Ok(false);
        };

        // read whether the symbol declares stored fields
        Ok(matches!(
            self.definition(symbol)?.as_deref(),
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
        let Some(binding) = self.generic_parameter(parameter)? else {
            return Ok(false);
        };
        let template = binding.template.into_global(parameter.module_id);
        let Some(template) = self.generic_template(template)? else {
            return Ok(false);
        };
        let Some(symbol) = template.symbol else {
            return Ok(false);
        };
        let definition = self.definition(symbol)?;
        let Some(dir::Definition::Newtype(newtype)) = definition.as_deref() else {
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
        // find the definition that declares the parameter's template
        let Some(definition) = self.parameter_owner_definition(parameter)? else {
            return Ok(Variance::Invariant);
        };

        // measure class and interface methods as reference instances
        let is_reference = matches!(
            *definition,
            dir::Definition::Class(_) | dir::Definition::Interface(_)
        );
        let storage = form.field();

        // collect the measured member types before walking type graphs
        let mut members = SmallVec::<[_; 8]>::new();
        for member in definition.members() {
            let measured = match member {
                // measure storage fields by the handle's write capability
                dir::DefinitionMember::Field(field) => {
                    let position = match field.is_readonly {
                        true => Variance::Covariant,
                        false => storage,
                    };

                    self.require_definition_member_type(member)?
                        .map(|ty| (ty, position))
                }
                // skip a constructor, which the constructor type owns
                dir::DefinitionMember::Method(method)
                    if matches!(
                        method.role,
                        Some(dir::FunctionRole::Constructor | dir::FunctionRole::New)
                    ) =>
                {
                    None
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
        if let dir::Definition::Newtype(newtype) = &*definition {
            members.push((newtype.backing, storage));
        }

        // measure the parameter through the declaration's heritage
        let mut heritages = definition
            .bases()
            .iter()
            .map(|heritage| heritage.ty)
            .collect::<SmallVec<[_; 2]>>();
        heritages.extend(
            definition
                .implementations()
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

    /// Return the declaration owning one parameter's template, none for function templates.
    fn parameter_owner_definition(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<Option<Arc<dir::Definition>>> {
        let Some(binding) = self.generic_parameter(parameter)? else {
            return Ok(None);
        };
        let template = binding.template.into_global(parameter.module_id);
        let Some(template) = self.generic_template(template)? else {
            return Ok(None);
        };
        let Some(symbol) = template.symbol else {
            return Ok(None);
        };

        self.definition(symbol)
    }

    /// Return the associated type value one parameter's declaration gives for one key.
    fn declared_associated_type(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(definition) = self.parameter_owner_definition(parameter)? else {
            return Ok(None);
        };

        let value = definition.members().iter().find_map(|member| match member {
            dir::DefinitionMember::AssociatedType(associated) if associated.key == key => {
                associated.value.or(associated.constraint)
            }
            _ => None,
        });

        Ok(value)
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

        // read the parameters and return covariantly, static dispatch types against the copy
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

        Ok(self.access_of(access)? == Some(dir::Access::Readonly))
    }

    /// Measure one parameter's occurrences in one type graph.
    fn measure_type(
        &mut self,
        ty: dir::GlobalTypeId,
        position: Variance,
        form: VarianceForm,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<Variance> {
        // stop at unused positions, which carry no occurrences
        if position == Variance::Bivariant {
            return Ok(Variance::Bivariant);
        }

        // measure the position the parameter occurs at
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
                    .object_properties(ty.module_id, shape.properties)?
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
                    .chain(self.type_ids(ty.module_id, shape.construct_signatures)?);
                for signature in signatures {
                    measured =
                        measured.join(self.measure_type(*signature, position, form, parameter)?);
                }
                let index_signatures: SmallVec<[_; 4]> = self
                    .object_index_signatures(ty.module_id, shape.index_signatures)?
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
                dir::Form::Readonly | dir::Form::Owned => {
                    self.measure_type(type_form.value, position, form, parameter)?
                }
                // readonly borrows view their pointee without write-back
                dir::Form::Borrowed(borrow) if self.borrow_is_readonly(ty.module_id, borrow)? => {
                    self.measure_type(type_form.value, position, VarianceForm::Readonly, parameter)?
                }
                // independently writable references stay invariant unless deeply readonly
                dir::Form::Borrowed(_) | dir::Form::Raw => {
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

            // measure a projection through the owner's declared value, else invariantly
            dir::Type::Member(member) => {
                let member = self.type_member(ty.module_id, member)?;
                if matches!(self.ty(member.owner)?, dir::Type::This)
                    && let Some(value) = self.declared_associated_type(parameter, member.key)?
                {
                    return self.measure_type(value, position, form, parameter);
                }
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

    /// Relate same-template applications, binding written arguments by kind first.
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
        // pair the written arguments by kind, recording elided lifetimes for Verify
        let Some(pairs) = self.pair_application_arguments(source, target)? else {
            return Ok(Verdict::Fails);
        };
        let (source, target): (SmallVec<[_; 4]>, SmallVec<[_; 4]>) = pairs.iter().copied().unzip();

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

        // relate each argument pair under its parameter's variance
        let mut verdict = Verdict::Holds;
        for (index, (source, target)) in source
            .iter()
            .copied()
            .zip(target.iter().copied())
            .enumerate()
        {
            // read both arguments through their solutions
            let source = self.shallow_resolve(source)?;
            let target = self.shallow_resolve(target)?;
            if self.is_free_argument_slot(source, target)? {
                continue;
            }

            // name the argument slot so diagnostics point at it
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

            // stop at the first failing argument
            verdict = verdict.and(related);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Match one implemented header against the requested arguments.
    pub(in crate::sema) fn match_header_arguments(
        &mut self,
        origin: Origin,
        cause: CauseId,
        symbol: dir::GlobalSymbolId,
        declared: &[dir::GlobalTypeId],
        requested: &[dir::GlobalTypeId],
    ) -> CompilerResult<Verdict> {
        // require the same number of arguments
        if declared.len() != requested.len() {
            return Ok(Verdict::Fails);
        }

        // relate each argument pair by its written variance
        let mut verdict = Verdict::Holds;
        for (index, (declared, requested)) in declared.iter().zip(requested.iter()).enumerate() {
            // skip the pairs that constrain neither slot
            let declared = self.shallow_resolve(*declared)?;
            let requested = self.shallow_resolve(*requested)?;
            if self.is_free_argument_slot(declared, requested)? {
                continue;
            }

            // orient the pair under the written variance and constrain it
            let variance = self.written_argument_variance(symbol, index)?;
            let related = match variance.argument_relation(Relation::Subtype) {
                None => Verdict::Holds,
                Some((relation, order)) => {
                    let (source, target) = order.orient(declared, requested);

                    self.constrain_type(origin, cause, relation, source, target)?
                }
            };

            // stop at the first failing argument
            verdict = verdict.and(related);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Return whether one argument pair leaves its slots unconstrained.
    fn is_free_argument_slot(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // erased target arguments match every instantiation of their parameter
        if matches!(self.ty(target)?, dir::Type::Erased(_)) {
            return Ok(true);
        }

        // link the open lifetime slots for Verify
        Ok(!self.type_flags(source)?.has_variable()
            && !self.type_flags(target)?.has_variable()
            && self.memory_kind(source)? == Some(dir::MemoryParameter::Region)
            && self.memory_kind(target)? == Some(dir::MemoryParameter::Region))
    }

    /// Return one indexed parameter's written variance, invariant when unannotated.
    fn written_argument_variance(
        &mut self,
        symbol: dir::GlobalSymbolId,
        index: usize,
    ) -> CompilerResult<Variance> {
        let written = match self.template_parameter(symbol, index)? {
            Some(parameter) => self
                .generic_parameter(parameter)?
                .and_then(|binding| binding.variance),
            None => None,
        };

        Ok(written.map(Variance::from).unwrap_or(Variance::Invariant))
    }

    /// Return one symbol's indexed template parameter.
    fn template_parameter(
        &mut self,
        symbol: dir::GlobalSymbolId,
        index: usize,
    ) -> CompilerResult<Option<GenericParameterId>> {
        let Some(template) = self.symbol_template(symbol)? else {
            return Ok(None);
        };

        Ok(self
            .generic_template_parameters(template)?
            .get(index)
            .copied())
    }

    /// Return one indexed argument's variance, invariant when unknown.
    pub(in crate::sema) fn argument_variance(
        &mut self,
        symbol: dir::GlobalSymbolId,
        index: usize,
        form: VarianceForm,
    ) -> CompilerResult<Variance> {
        // read the variance the declared parameter states
        match self.template_parameter(symbol, index)? {
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

        // measure the parameter across every argument
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
        // collect every declared definition's template parameters
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
            let Some(binding) = self.generic_parameter(parameter)? else {
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

            // keep the parameters whose derivation reifies as a modifier
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

    /// Return whether one const parameter ranges over literals or enum cases alone.
    pub(in crate::sema) fn is_static_const_parameter(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<bool> {
        let Some(binding) = self.generic_parameter(parameter)? else {
            return Ok(false);
        };
        if !binding.is_const || binding.memory_parameter().is_some() {
            return Ok(false);
        }
        let Some(constraint) = binding.constraint else {
            return Ok(false);
        };
        let symbol = match self.ty(constraint)? {
            dir::Type::Primitive(_) | dir::Type::Literal(_) | dir::Type::Variant(_) => {
                return Ok(true);
            }
            dir::Type::Reference(reference) => reference.symbol,
            dir::Type::Application(application) => application.symbol,
            _ => return Ok(false),
        };

        Ok(matches!(self.symbol_kind(symbol)?, dir::SymbolKind::Enum))
    }

    /// Return whether one type carries One cardinality.
    pub(in crate::sema) fn has_one_cardinality(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // expose the value domain behind computation heads before deciding
        let ty = self.normalize_computation(origin, ty)?;
        let is_one = match self.ty(ty)? {
            // literals and errors stand for one value
            dir::Type::Literal(_) | dir::Type::Error => true,
            // bare enum members carry one discriminant value
            dir::Type::Variant(_) => true,
            // fail loudly on unsettled variables
            dir::Type::Variable(_) => {
                return Err(CompilerError::Internal {
                    message: format!("unsettled variable at {origin:?}"),
                });
            }
            // const parameters stand for one value by declaration
            dir::Type::Parameter(parameter) => {
                self.is_static_const_parameter(parameter)?
                    || self
                        .generic_parameter(parameter)?
                        .is_some_and(|binding| binding.memory_parameter().is_some())
            }
            // static operations over exact operands compute one exact value
            dir::Type::Operation(operation) => {
                match self.type_operation(ty.module_id, operation)? {
                    dir::TypeOperation::StaticBinary(binary) => {
                        self.has_one_cardinality(origin, binary.left)?
                            && self.has_one_cardinality(origin, binary.right)?
                    }
                    dir::TypeOperation::StaticUnary(unary) => {
                        self.has_one_cardinality(origin, unary.target)?
                    }
                    // infer binders match the one value the scrutinee fixed
                    dir::TypeOperation::Infer(_) => true,
                    _ => false,
                }
            }
            _ => false,
        };

        Ok(is_one)
    }
}
