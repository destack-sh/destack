use destack_dir as dir;
use smallvec::SmallVec;

use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{CheckState, Origin, Relation};

impl CheckState<'_> {
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
            // leave derivable interfaces to their generated extensions
            dir::AutoInterface::Clone
            | dir::AutoInterface::Debug
            | dir::AutoInterface::Default
            | dir::AutoInterface::Hash
            | dir::AutoInterface::Equal
            | dir::AutoInterface::PartialEqual
            | dir::AutoInterface::Compare
            | dir::AutoInterface::PartialCompare
            | dir::AutoInterface::Serialize
            | dir::AutoInterface::Deserialize => Ok(false),
        }?;
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
        // record one conformance for each concrete nominal instance
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
