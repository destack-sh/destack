use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    CandidateOutcome, Cause, CauseKind, CheckState, Origin, Relation, TypeSubstitution, Verdict,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Decide one applied compiler-known interface intrinsically.
    pub(in crate::sema) fn decide_intrinsic_interface(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<Verdict> {
        // read the compared type of each intrinsic binary interface
        if matches!(
            interface,
            dir::AutoInterface::Equal
                | dir::AutoInterface::PartialEqual
                | dir::AutoInterface::Compare
                | dir::AutoInterface::PartialCompare
                | dir::AutoInterface::StrictEqual
        ) {
            let (module, application) = self.nominal_application(target)?;
            let arguments = self.type_ids(module, application.arguments)?;

            // read an elided argument as the receiver itself
            let other = match arguments {
                [] => None,
                [other] => Some(*other),
                _ => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "intrinsic {} application has {} arguments",
                            interface.name(),
                            arguments.len()
                        ),
                    });
                }
            };

            // relate the receiver against the compared operand
            if let Some(other) = other {
                // read the argument solutions the bound picked up after queuing
                let other = self.shallow_resolve(other)?;
                let substitution = TypeSubstitution::default().with_receiver(ty);
                let other = self.substitute_type(other, &substitution)?;
                if !self.collect_open_variables([ty, other])?.is_empty() {
                    return Ok(Verdict::Ambiguous);
                }

                // decide strict equality from both exact operands
                if interface == dir::AutoInterface::StrictEqual {
                    let is_equatable = self.has_strict_equal_conformance(origin, ty, other)?;

                    return Ok(Verdict::decided(is_equatable));
                }

                // compare numeric scalars across their exact domains
                let domains = (
                    self.ty(ty)?.scalar_domain(),
                    self.ty(other)?.scalar_domain(),
                );
                let numeric = matches!(
                    domains,
                    (
                        Some(dir::ScalarDomain::Integer | dir::ScalarDomain::Float),
                        Some(dir::ScalarDomain::Integer | dir::ScalarDomain::Float),
                    )
                );

                // require the argument to equal the receiver otherwise
                let receiver = match numeric {
                    true => Verdict::Holds,
                    false => self.decide_relation(origin, Relation::Equal, ty, other)?,
                };
                if receiver != Verdict::Holds {
                    return Ok(receiver);
                }
            }
        }

        // decide the interface's own conformance rule, propagating its verdict
        self.decide_auto_interface(origin, ty, interface)
    }

    /// Decide one compiler-known auto interface for one type.
    pub(in crate::sema) fn decide_auto_interface(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<Verdict> {
        // decide an open numeric variable at its family default
        if let Some(variable) = self.root_variable(ty)? {
            return match self.root_kind(variable)?.fallback() {
                Some(fallback) => {
                    let fallback = self.intern_type(fallback)?;

                    self.decide_auto_interface(origin, fallback, interface)
                }
                None => Ok(Verdict::Ambiguous),
            };
        }

        // decide conformance once over variable-free types under the assuming template
        let flags = self.type_flags(ty)?;
        let key = if flags.has_variable() {
            None
        } else {
            self.decision_scope(origin, flags)?
                .map(|assumes| (ty, interface, assumes))
        };

        // serve the memoized verdict
        if let Some(key) = &key
            && let Some(is_holds) = self.conformances.get(key)
        {
            return Ok(Verdict::decided(*is_holds));
        }

        // use bounds declared by generic types
        if let Some(decision) = self.decide_generic_auto_interface(origin, ty, interface)? {
            if let Some(key) = key {
                self.conformances.insert(key, decision);
            }

            return Ok(Verdict::decided(decision));
        }

        // let a written derive list replace the auto set of its declaration
        if interface.is_auto_derivable()
            && let dir::Type::Application(instance) = self.ty(ty)?
        {
            let excluded = self
                .definition(instance.symbol)?
                .and_then(dir::Definition::derives)
                .is_some_and(|derives| !derives.contains(&interface));
            if excluded {
                if let Some(key) = key {
                    self.conformances.insert(key, false);
                }

                return Ok(Verdict::Fails);
            }
        }

        // dispatch compiler-known conformance rules
        let mut active = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        let verdict = match interface {
            dir::AutoInterface::AtomicSafe => self.is_atomic_safe(ty).map(Verdict::decided),
            dir::AutoInterface::DynamicSafe => self.decide_dynamic_safe(origin, ty, &mut active),
            dir::AutoInterface::OverwriteStable => {
                self.decide_overwrite_stable(origin, ty, &mut active)
            }
            dir::AutoInterface::Integer => self
                .has_scalar_representation(ty, dir::ScalarDomain::Integer)
                .map(Verdict::decided),
            dir::AutoInterface::IntegerDomain => self
                .is_scalar_domain_only(origin, ty, dir::ScalarDomain::Integer)
                .map(Verdict::decided),
            dir::AutoInterface::Float => self
                .has_scalar_representation(ty, dir::ScalarDomain::Float)
                .map(Verdict::decided),
            dir::AutoInterface::FloatDomain => self
                .is_scalar_domain_only(origin, ty, dir::ScalarDomain::Float)
                .map(Verdict::decided),
            dir::AutoInterface::Copy => self.decide_copy(origin, ty, &mut active),
            dir::AutoInterface::Drop => self.decide_drop(origin, ty, &mut active),
            dir::AutoInterface::SharedSafe => self.is_shared_safe(origin, ty).map(Verdict::decided),
            dir::AutoInterface::SuspendSafe => {
                self.is_suspend_safe(origin, ty).map(Verdict::decided)
            }
            dir::AutoInterface::Concrete => self.is_concrete(origin, ty).map(Verdict::decided),
            dir::AutoInterface::StrictEqual => self
                .has_strict_equal_conformance(origin, ty, ty)
                .map(Verdict::decided),
            dir::AutoInterface::Equal
            | dir::AutoInterface::PartialEqual
            | dir::AutoInterface::Clone
            | dir::AutoInterface::Debug
            | dir::AutoInterface::Display
            | dir::AutoInterface::Hash
            | dir::AutoInterface::Default
            | dir::AutoInterface::Unpin
            | dir::AutoInterface::Zeroable => self.decide_derivable(origin, ty, interface),
            // order scalars intrinsically
            dir::AutoInterface::Compare | dir::AutoInterface::PartialCompare => {
                Ok(Verdict::decided(
                    self.ty(ty)?
                        .scalar_domain()
                        .and_then(|domain| domain.conforms_to(interface))
                        .unwrap_or(false),
                ))
            }
            // leave defaults and serialization to written derives
            dir::AutoInterface::Serialize | dir::AutoInterface::Deserialize => Ok(Verdict::Fails),
        }?;

        // memoize a decided verdict, leaving an ambiguous one uncached
        if let Some(key) = key
            && verdict != Verdict::Ambiguous
        {
            self.conformances.insert(key, verdict.holds());
        }

        Ok(verdict)
    }

    /// Decide auto conformance for one generic type.
    pub(in crate::sema) fn decide_generic_auto_interface(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<Option<bool>> {
        // read the resolved head of the subject
        let ty = self.shallow_resolve(ty)?;

        // select the bounds owned by each generic form
        let decision = match self.ty(ty)? {
            dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) => {
                let item = dir::LanguageItem::from(interface);
                let target = self.language_type(item, &[])?;
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

                Some(
                    self.relate_parameter_bounds(
                        origin,
                        cause,
                        Relation::Subtype,
                        parameter,
                        target,
                    )?
                    .holds(),
                )
            }
            dir::Type::This => {
                let item = dir::LanguageItem::from(interface);
                let target = self.language_type(item, &[])?;
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

                Some(
                    self.relate_this_bounds(origin, cause, Relation::Subtype, target)?
                        .holds(),
                )
            }
            _ => None,
        };

        Ok(decision)
    }

    /// Return whether two operand types have builtin `StrictEqual<R>` conformance.
    fn has_strict_equal_conformance(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // require builtin equality over overlapping operands
        let has_builtin = self.has_builtin_strict_equality(origin, left, right)?;
        let overlaps = self.types_may_overlap(origin, left, right)?;

        Ok(has_builtin && overlaps)
    }

    /// Return whether one type has one builtin scalar representation.
    fn has_scalar_representation(
        &mut self,
        ty: dir::GlobalTypeId,
        domain: dir::ScalarDomain,
    ) -> CompilerResult<bool> {
        // match the primitive's own scalar domain
        let is_representation = matches!(
            self.ty(ty)?,
            dir::Type::Primitive(primitive) if primitive.scalar_domain() == domain
        );

        Ok(is_representation)
    }

    /// Return whether one type belongs entirely to one scalar domain.
    fn is_scalar_domain_only(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        domain: dir::ScalarDomain,
    ) -> CompilerResult<bool> {
        // require every family to sit inside that domain
        let families = self.scalar_families(origin, ty)?;
        let is_only_domain = families.is_some_and(|families| families.is_only_domain(domain));

        Ok(is_only_domain)
    }

    /// Decide the auto interfaces one closed type satisfies.
    fn decided_conformances(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::AutoInterfaceSet> {
        // reduce compiler-known applications to the representations they decide as
        let ty = self.normalize(origin, ty)?;

        // record each decided conformance, leaving undecided interfaces unset
        let mut conformances = dir::AutoInterfaceSet::new();
        for interface in dir::AutoInterface::ALL {
            // decide without committing bindings or reports
            let mut verdict = Verdict::Fails;
            self.decide_candidate(|state| {
                verdict = state.decide_auto_interface(origin, ty, interface)?;

                Ok(CandidateOutcome::<(), ()>::Rejected(()))
            })?;
            if verdict == Verdict::Holds {
                conformances.insert(interface);
            }
        }

        Ok(conformances)
    }

    /// Commit the auto conformances of each instantiation-invariant nominal declaration.
    pub(in crate::sema) fn commit_definition_conformances(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        // collect the nominal declarations whose conformances stay instantiation-invariant
        let mut candidates = Vec::new();
        for (symbol, definition) in self.module(module).iter_definitions() {
            let is_nominal = matches!(
                definition,
                dir::Definition::Struct(_)
                    | dir::Definition::Class(_)
                    | dir::Definition::Enum(_)
                    | dir::Definition::Newtype(_)
            );
            if is_nominal {
                candidates.push((symbol, definition.template()));
            }
        }

        // conformances are region and space invariant, so memory-only templates commit here
        let mut nominals = Vec::new();
        for (symbol, template) in candidates {
            let is_invariant = match template {
                None => true,
                Some(template) => {
                    let template = dir::GlobalGenericTemplateId::new(module, template);
                    self.generic_template_parameters(template)?
                        .iter()
                        .all(|parameter| {
                            self.generic_parameter(*parameter)
                                .is_some_and(|row| row.memory_parameter().is_some())
                        })
                }
            };
            if is_invariant {
                nominals.push(symbol);
            }
        }

        // decide and commit the satisfied set on each declaration
        for symbol in nominals {
            let instance = self.declaration_instance(symbol)?;
            let target = self.intern_type(dir::Type::Application(instance))?;
            let origin = Origin::Symbol(symbol);
            let conformances = self.decided_conformances(origin, target)?;
            if let Some(definition) = self.module_mut(module).definition_mut(symbol) {
                definition.set_conformances(conformances);
            }
        }

        Ok(())
    }

    /// Commit the assumed auto conformances of each declared generic parameter.
    pub(in crate::sema) fn commit_parameter_conformances(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        // collect the constrained parameters of the pass segment
        let mut constrained = Vec::new();
        for (parameter_id, binding) in self.module(module).generics_tail.iter_parameters() {
            if let Some(constraint) = binding.constraint {
                constrained.push((parameter_id, binding.source, constraint));
            }
        }

        // commit the interface closure each written bound assumes
        for (parameter_id, source, constraint) in constrained {
            let origin = Origin::Node(source, None);
            let conformances = self.assumed_auto_interfaces(origin, constraint)?;
            self.module_mut(module)
                .generics_tail
                .set_parameter_conformances(parameter_id, conformances);
        }

        Ok(())
    }

    /// Collect the auto interfaces one written bound assumes, closing over interface heritage.
    fn assumed_auto_interfaces(
        &mut self,
        origin: Origin,
        constraint: dir::GlobalTypeId,
    ) -> CompilerResult<dir::AutoInterfaceSet> {
        // seed the walk at the written bound
        let mut conformances = dir::AutoInterfaceSet::new();
        let mut pending = vec![constraint];
        let mut visited = FxIndexSet::default();

        // walk each bound and the interfaces it inherits
        while let Some(ty) = pending.pop() {
            let ty = self.normalize(origin, ty)?;
            match self.ty(ty)? {
                // intersection bounds assume every element
                dir::Type::Intersection(intersection) => {
                    pending.extend(self.type_ids(ty.module_id, intersection.elements)?);
                }
                // applied interfaces assume their identity and their heritage
                dir::Type::Application(dir::GenericApplication { symbol, .. })
                | dir::Type::Reference(dir::TypeReference { symbol }) => {
                    if !visited.insert(symbol) {
                        continue;
                    }
                    if let Some(item) = self.language_item(symbol)?
                        && let Some(interface) = dir::AutoInterface::from_language_item(item)
                    {
                        conformances.insert(interface);
                    }
                    if let Some(dir::Definition::Interface(definition)) = self.definition(symbol)? {
                        let inherited: Vec<_> = definition
                            .extends
                            .iter()
                            .map(|heritage| heritage.ty)
                            .collect();
                        pending.extend(inherited);
                    }
                }
                _ => {}
            }
        }

        Ok(conformances)
    }

    /// Commit the auto conformances of each nominal instance this pass materialized.
    pub(in crate::sema) fn commit_instance_conformances(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        // collect the nominal instances of the pass segment
        let mut nominals = Vec::new();
        for (instance_id, instance) in self.module(module).generics_tail.iter_instances() {
            nominals.push((instance_id, instance.key.clone(), instance.source));
        }

        // decide and commit the satisfied set on each closed nominal
        for (instance_id, key, source) in nominals {
            let is_nominal = matches!(
                self.definition(key.symbol)?,
                Some(
                    dir::Definition::Struct(_)
                        | dir::Definition::Class(_)
                        | dir::Definition::Enum(_)
                        | dir::Definition::Newtype(_)
                )
            );
            if !is_nominal {
                continue;
            }

            // apply the declaration at the instance's closed type arguments
            let mut arguments = Vec::with_capacity(key.arguments.len());
            for binding in &key.arguments {
                let is_induced = self
                    .generic_parameter(binding.parameter)
                    .is_some_and(|row| row.induced_memory_parameter().is_some());
                if !is_induced {
                    arguments.push(binding.argument);
                }
            }
            let arguments = self.intern_type_ids(&arguments)?;
            let target = self.intern_type(dir::Type::Application(dir::GenericApplication {
                symbol: key.symbol,
                arguments,
            }))?;
            let origin = Origin::Node(source, None);
            let conformances = self.decided_conformances(origin, target)?;
            self.module_mut(module)
                .generics_tail
                .set_instance_conformances(instance_id, conformances);
        }

        Ok(())
    }
}
