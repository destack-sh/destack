use destack_dir as dir;
use smallvec::SmallVec;

use destack_source::ModuleId;

use crate::check::{Cause, CauseKind, CheckState, Origin, Relation, TypeSubstitution, Verdict};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Decide whether one type intrinsically satisfies one applied compiler-known interface.
    pub(in crate::check) fn satisfies_intrinsic_interface(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<Verdict> {
        // require intrinsic binary protocols to use the receiver type on both sides
        if matches!(
            interface,
            dir::AutoInterface::Equal
                | dir::AutoInterface::PartialEqual
                | dir::AutoInterface::Compare
                | dir::AutoInterface::PartialCompare
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
                let receiver_holds =
                    numeric || self.evaluate_relation(origin, Relation::Equal, ty, other)?;
                if !receiver_holds {
                    // leave the receiver rule undecided for an open argument
                    if !self.open_type_variables([ty, other])?.is_empty() {
                        return Ok(Verdict::Ambiguous);
                    }

                    return Ok(Verdict::Fails);
                }
            }
        }

        // decide the interface's own conformance rule
        let holds = self.satisfies_auto_interface(origin, ty, interface)?;

        // leave the rule undecided for an open variable inside the subject
        if !holds && !self.open_type_variables([ty])?.is_empty() {
            return Ok(Verdict::Ambiguous);
        }

        Ok(Verdict::decided(holds))
    }

    /// Decide whether one type satisfies a compiler-known auto interface.
    pub(in crate::check) fn satisfies_auto_interface(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<bool> {
        // conformance over settled types is a durable fact
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

        // serve the memo
        if let Some(key) = &key
            && let Some(holds) = self.conforms.get(key)
        {
            return Ok(*holds);
        }

        // use bounds declared by generic types
        if let Some(decision) = self.decide_generic_auto_interface(origin, ty, interface)? {
            if let Some(key) = key {
                self.conforms.insert(key, decision);
            }

            return Ok(decision);
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

                return Ok(false);
            }
        }

        // dispatch compiler-known conformance rules
        let mut active = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        let holds = match interface {
            dir::AutoInterface::AtomicSafe => self.satisfies_atomic_safe(ty),
            dir::AutoInterface::DynamicSafe => self.satisfies_dynamic_safe(origin, ty, &mut active),
            dir::AutoInterface::OverwriteStable => {
                self.satisfies_overwrite_stable(origin, ty, &mut active)
            }
            dir::AutoInterface::Integer => {
                self.satisfies_scalar_representation(ty, dir::ScalarDomain::Integer)
            }
            dir::AutoInterface::IntegerDomain => {
                self.satisfies_scalar_domain(origin, ty, dir::ScalarDomain::Integer)
            }
            dir::AutoInterface::Float => {
                self.satisfies_scalar_representation(ty, dir::ScalarDomain::Float)
            }
            dir::AutoInterface::FloatDomain => {
                self.satisfies_scalar_domain(origin, ty, dir::ScalarDomain::Float)
            }
            dir::AutoInterface::Copy => self.satisfies_copy(origin, ty, &mut active),
            dir::AutoInterface::SharedSafe => self.satisfies_shared_safe(origin, ty),
            // TODO #Incomplete: the remaining auto interfaces never hold
            dir::AutoInterface::Unpin | dir::AutoInterface::Zeroable => Ok(false),
            dir::AutoInterface::Concrete => self.satisfies_concrete(origin, ty),
            dir::AutoInterface::Equal
            | dir::AutoInterface::PartialEqual
            | dir::AutoInterface::Clone
            | dir::AutoInterface::Debug
            | dir::AutoInterface::Display
            | dir::AutoInterface::Hash => self.satisfies_derivable(origin, ty, interface),
            // order scalars intrinsically
            dir::AutoInterface::Compare | dir::AutoInterface::PartialCompare => Ok(self
                .ty(ty)?
                .scalar_domain()
                .and_then(|domain| domain.conforms_to(interface))
                .unwrap_or(false)),
            // leave defaults and serialization to written derives
            dir::AutoInterface::Default
            | dir::AutoInterface::Serialize
            | dir::AutoInterface::Deserialize => Ok(false),
        }?;

        // memoize the settled decision
        if let Some(key) = key {
            self.conforms.insert(key, holds);
        }

        Ok(holds)
    }

    /// Decide auto conformance for one generic type.
    pub(in crate::check) fn decide_generic_auto_interface(
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

                Some(self.relate_parameter_bounds(
                    origin,
                    cause,
                    Relation::Satisfies,
                    parameter,
                    target,
                )?)
            }
            dir::Type::This => {
                let item = dir::LanguageItem::from(interface);
                let target = self.language_type(item, &[])?;
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

                Some(self.relate_this_bounds(origin, cause, Relation::Satisfies, target)?)
            }
            _ => None,
        };

        Ok(decision)
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
    pub(in crate::check) fn derive_module_conformances(
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
                let holds = self.satisfies_auto_interface(origin, target, interface)?;
                if holds {
                    self.module_mut(module)
                        .auto
                        .push_conformance(dir::AutoConformance { interface, target });
                }
            }
        }

        Ok(())
    }
}
