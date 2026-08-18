use destack_dir as dir;
use smallvec::SmallVec;

use destack_source::ModuleId;

use crate::sema::{Cause, CauseKind, CheckState, Origin, Relation, TypeSubstitution, Verdict};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Decide whether one type intrinsically satisfies one applied compiler-known interface.
    pub(in crate::sema) fn satisfies_intrinsic_interface(
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
            if let Some(other) = other {
                // read argument solutions settled since the bound was queued
                let other = self.shallow_resolve(other)?;
                let substitution = TypeSubstitution::default().with_receiver(ty);
                let other = self.substitute_type(other, &substitution)?;

                // decide strict equality from both exact operands
                if interface == dir::AutoInterface::StrictEqual {
                    let satisfied = self.satisfies_strict_equal(origin, ty, other)?;

                    return Ok(Verdict::decided(satisfied));
                }

                // allow numeric scalars to compare across their exact domains
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
                    false => self.evaluate_relation(origin, Relation::Equal, ty, other)?,
                };
                if receiver != Verdict::Holds {
                    return Ok(receiver);
                }
            }
        }

        // decide the interface's own conformance rule, propagating its verdict
        self.satisfies_auto_interface(origin, ty, interface)
    }

    /// Decide whether one type satisfies a compiler-known auto interface.
    pub(in crate::sema) fn satisfies_auto_interface(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<Verdict> {
        // decide conformance once over variable-free types, since a hole stands still
        let flags = self.type_flags(ty)?;
        let key = if flags.has_variable() {
            None
        } else {
            let scope = if flags.has_parameter() || flags.has_this() {
                self.assuming_scope(origin)?
            } else {
                None
            };

            Some((ty, interface, scope))
        };

        // serve the memoized verdict
        if let Some(key) = &key
            && let Some(holds) = self.conforms.get(key)
        {
            return Ok(Verdict::decided(*holds));
        }

        // use bounds declared by generic types
        if let Some(decision) = self.decide_generic_auto_interface(origin, ty, interface)? {
            if let Some(key) = key {
                self.conforms.insert(key, decision);
            }

            return Ok(Verdict::decided(decision));
        }

        // allow a written derive list to replace the auto set of its declaration
        if interface.is_auto_derivable()
            && let dir::Type::Application(instance) = self.ty(ty)?
        {
            let excluded = self
                .definition(instance.symbol)?
                .and_then(dir::Definition::derives)
                .is_some_and(|derives| !derives.contains(&interface));
            if excluded {
                if let Some(key) = key {
                    self.conforms.insert(key, false);
                }

                return Ok(Verdict::Fails);
            }
        }

        // dispatch compiler-known conformance rules
        let mut active = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        let verdict = match interface {
            dir::AutoInterface::AtomicSafe => self.satisfies_atomic_safe(ty).map(Verdict::decided),
            dir::AutoInterface::DynamicSafe => self.satisfies_dynamic_safe(origin, ty, &mut active),
            dir::AutoInterface::OverwriteStable => {
                self.satisfies_overwrite_stable(origin, ty, &mut active)
            }
            dir::AutoInterface::Integer => self
                .satisfies_scalar_representation(ty, dir::ScalarDomain::Integer)
                .map(Verdict::decided),
            dir::AutoInterface::IntegerDomain => self
                .satisfies_scalar_domain(origin, ty, dir::ScalarDomain::Integer)
                .map(Verdict::decided),
            dir::AutoInterface::Float => self
                .satisfies_scalar_representation(ty, dir::ScalarDomain::Float)
                .map(Verdict::decided),
            dir::AutoInterface::FloatDomain => self
                .satisfies_scalar_domain(origin, ty, dir::ScalarDomain::Float)
                .map(Verdict::decided),
            dir::AutoInterface::Copy => self.satisfies_copy(origin, ty, &mut active),
            dir::AutoInterface::SharedSafe => {
                self.satisfies_shared_safe(origin, ty).map(Verdict::decided)
            }
            // TODO #Incomplete: the remaining auto interfaces never hold
            dir::AutoInterface::Unpin | dir::AutoInterface::Zeroable => Ok(Verdict::Fails),
            dir::AutoInterface::Concrete => {
                self.satisfies_concrete(origin, ty).map(Verdict::decided)
            }
            dir::AutoInterface::StrictEqual => self
                .satisfies_strict_equal(origin, ty, ty)
                .map(Verdict::decided),
            dir::AutoInterface::Equal
            | dir::AutoInterface::PartialEqual
            | dir::AutoInterface::Clone
            | dir::AutoInterface::Debug
            | dir::AutoInterface::Display
            | dir::AutoInterface::Hash => self
                .satisfies_derivable(origin, ty, interface)
                .map(Verdict::decided),
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
            dir::AutoInterface::Default
            | dir::AutoInterface::Serialize
            | dir::AutoInterface::Deserialize => Ok(Verdict::Fails),
        }?;

        // memoize a settled verdict, leaving an ambiguous one uncached
        if let Some(key) = key
            && verdict != Verdict::Ambiguous
        {
            self.conforms.insert(key, verdict.holds());
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
        // read the settled head of the subject
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
                        Relation::Satisfies,
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
                    self.relate_this_bounds(origin, cause, Relation::Satisfies, target)?
                        .holds(),
                )
            }
            _ => None,
        };

        Ok(decision)
    }

    /// Decide builtin `StrictEqual<R>` conformance for two operand types.
    fn satisfies_strict_equal(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let supported = self.supports_builtin_strict_equality(origin, left, right)?;
        let overlaps = self.types_may_overlap(origin, left, right)?;

        Ok(supported && overlaps)
    }

    /// Decide whether one type has one builtin scalar representation.
    fn satisfies_scalar_representation(
        &mut self,
        ty: dir::GlobalTypeId,
        domain: dir::ScalarDomain,
    ) -> CompilerResult<bool> {
        let holds = matches!(
            self.ty(ty)?,
            dir::Type::Primitive(primitive) if primitive.scalar_domain() == domain
        );

        Ok(holds)
    }

    /// Decide whether one type belongs entirely to one scalar domain.
    fn satisfies_scalar_domain(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        domain: dir::ScalarDomain,
    ) -> CompilerResult<bool> {
        let families = self.scalar_families(origin, ty)?;
        let holds = families.is_some_and(|families| families.is_only_domain(domain));

        Ok(holds)
    }

    /// Record marker conformance and seal written derives for each concrete nominal.
    pub(in crate::sema) fn derive_module_conformances(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        // collect every concrete nominal declaration in the module
        let mut nominals = Vec::new();
        for (symbol, definition) in self.module(module).iter_definitions() {
            let is_nominal = matches!(
                definition,
                dir::Definition::Struct(_)
                    | dir::Definition::Class(_)
                    | dir::Definition::Enum(_)
                    | dir::Definition::Newtype(_)
            );
            if is_nominal && definition.template().is_none() {
                nominals.push(symbol);
            }
        }

        // record the satisfied markers on each nominal's own instance
        for symbol in nominals {
            let instance = self.declaration_instance(symbol)?;
            let target = self.intern_type(dir::Type::Application(instance))?;
            let origin = Origin::Symbol(symbol);
            for interface in dir::AutoInterface::REPRESENTATION {
                let verdict = self.satisfies_auto_interface(origin, target, interface)?;
                if verdict == Verdict::Ambiguous {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "declaration instance {target:?} left {interface:?} conformance ambiguous"
                        ),
                    });
                }
                if verdict == Verdict::Holds {
                    self.module_mut(module)
                        .auto
                        .push_conformance(dir::AutoConformance { interface, target });
                }
            }
        }

        Ok(())
    }
}
