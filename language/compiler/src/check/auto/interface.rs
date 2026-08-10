use destack_dir as dir;
use smallvec::SmallVec;

use destack_source::ModuleId;

use crate::check::{CheckState, Origin, Relation, Verdict};
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
            dir::AutoInterface::Equal | dir::AutoInterface::PartialEqual
        ) {
            let (module, application) = self.nominal_application(target)?;
            let arguments = self.type_ids(module, application.arguments)?;
            let [other] = arguments else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "intrinsic {} application has {} arguments",
                        interface.name(),
                        arguments.len()
                    ),
                });
            };

            // read argument solutions settled since the bound was queued
            let other = self.shallow_resolve(*other)?;
            let receiver_holds = self.decide_relation(origin, Relation::Equal, ty, other)?;
            if !receiver_holds {
                // an open argument leaves the receiver rule undecided
                if !self.open_type_variables([ty, other])?.is_empty() {
                    return Ok(Verdict::Ambiguous);
                }

                return Ok(Verdict::Fails);
            }
        }

        // decide the interface's own conformance rule
        let holds = self.satisfies_auto_interface(origin, ty, interface)?;

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
        let ty = self.reduce_type_head(origin, ty)?;
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

        // dispatch compiler-known conformance rules
        let mut active = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        let holds = match interface {
            dir::AutoInterface::AtomicSafe => self.satisfies_atomic_safe(origin, ty),
            dir::AutoInterface::DynamicSafe => self.satisfies_dynamic_safe(origin, ty, &mut active),
            dir::AutoInterface::OverwriteStable => {
                self.satisfies_overwrite_stable(origin, ty, &mut active)
            }
            dir::AutoInterface::Integer => {
                self.satisfies_scalar_representation(origin, ty, dir::ScalarDomain::Integer)
            }
            dir::AutoInterface::IntegerDomain => {
                self.satisfies_scalar_domain(origin, ty, dir::ScalarDomain::Integer)
            }
            dir::AutoInterface::Float => {
                self.satisfies_scalar_representation(origin, ty, dir::ScalarDomain::Float)
            }
            dir::AutoInterface::FloatDomain => {
                self.satisfies_scalar_domain(origin, ty, dir::ScalarDomain::Float)
            }
            dir::AutoInterface::Copy => self.satisfies_copy(origin, ty, &mut active),
            dir::AutoInterface::SharedSafe => self.satisfies_shared_safe(origin, ty),
            // TODO #Incomplete: the remaining auto interfaces never hold
            dir::AutoInterface::Unpin | dir::AutoInterface::Zeroable => Ok(false),
            dir::AutoInterface::Concrete => self.satisfies_concrete(origin, ty),
            dir::AutoInterface::Equal | dir::AutoInterface::PartialEqual => {
                self.satisfies_builtin_equality(origin, ty, interface)
            }
            // leave derivable interfaces to their generated extensions
            dir::AutoInterface::Clone
            | dir::AutoInterface::Debug
            | dir::AutoInterface::Default
            | dir::AutoInterface::Hash
            | dir::AutoInterface::Compare
            | dir::AutoInterface::PartialCompare
            | dir::AutoInterface::Serialize
            | dir::AutoInterface::Deserialize => Ok(false),
        }?;

        // memoize the settled decision
        if let Some(key) = key {
            self.conforms.insert(key, holds);
        }

        Ok(holds)
    }

    /// Decide intrinsic equality conformance for builtin scalar values.
    fn satisfies_builtin_equality(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<bool> {
        // read the subject's scalar domain
        let ty = self.reduce_type_head(origin, ty)?;
        let domain = self.ty(ty)?.scalar_domain();

        // discrete scalar domains compare exactly, floats only partially
        let holds = matches!(
            (interface, domain),
            (
                dir::AutoInterface::Equal | dir::AutoInterface::PartialEqual,
                Some(
                    dir::ScalarDomain::Integer
                        | dir::ScalarDomain::Bigint
                        | dir::ScalarDomain::Character
                        | dir::ScalarDomain::Symbol
                        | dir::ScalarDomain::Boolean
                        | dir::ScalarDomain::Null
                        | dir::ScalarDomain::Undefined,
                ),
            ) | (
                dir::AutoInterface::PartialEqual,
                Some(dir::ScalarDomain::Float)
            )
        );

        Ok(holds)
    }

    /// Decide auto conformance for one generic type.
    pub(in crate::check) fn decide_generic_auto_interface(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<Option<bool>> {
        let ty = self.shallow_resolve(ty)?;

        // select the bounds owned by each generic form
        let decision = match self.ty(ty)? {
            dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) => {
                let item = dir::LanguageItem::from(interface);
                let target = self.language_type(item, &[])?;

                Some(self.decide_parameter_relation(
                    origin,
                    Relation::Satisfies,
                    parameter,
                    target,
                )?)
            }
            dir::Type::This => {
                let item = dir::LanguageItem::from(interface);
                let target = self.language_type(item, &[])?;

                Some(self.decide_this_relation(origin, Relation::Satisfies, target)?)
            }
            _ => None,
        };

        Ok(decision)
    }

    /// Decide whether one type has one builtin scalar representation.
    fn satisfies_scalar_representation(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        domain: dir::ScalarDomain,
    ) -> CompilerResult<bool> {
        let ty = self.reduce_type_head(origin, ty)?;
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

    /// Record representation marker conformance for each concrete nominal.
    pub(in crate::check) fn derive_module_conformances(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        // collect every concrete nominal declaration in the module
        let mut nominals = Vec::new();
        for (symbol, definition) in self.module(module).definitions.iter_definitions() {
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
