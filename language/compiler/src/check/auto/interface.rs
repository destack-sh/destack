use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, Relation, answer};

impl CheckState<'_> {
    /// Decide whether one type satisfies a compiler-known auto interface.
    pub(in crate::check) fn satisfies_auto_interface(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<Answer<bool>> {
        // use bounds declared by generic types
        if let Some(decision) = self.decide_generic_auto_interface(origin, ty, interface)? {
            return Ok(decision);
        }

        // dispatch compiler-known conformance rules
        let mut active = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        match interface {
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
            dir::AutoInterface::Unpin | dir::AutoInterface::Zeroable => Ok(Answer::Ready(false)),
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
            | dir::AutoInterface::Deserialize => Ok(Answer::Ready(false)),
        }
    }

    /// Decide auto conformance for one generic type.
    pub(in crate::check) fn decide_generic_auto_interface(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<Option<Answer<bool>>> {
        let ty = self.settled_root(ty)?;

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
    ) -> CompilerResult<Answer<bool>> {
        let ty = answer!(self.reduce_type_head(origin, ty)?);
        let holds = matches!(
            self.ty(ty)?,
            dir::Type::Primitive(primitive) if primitive.scalar_domain() == domain
        );

        Ok(Answer::Ready(holds))
    }

    /// Decide whether one type belongs entirely to one scalar domain.
    fn satisfies_scalar_domain(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        domain: dir::ScalarDomain,
    ) -> CompilerResult<Answer<bool>> {
        let families = answer!(self.scalar_families(origin, ty)?);
        let holds = families.is_some_and(|families| families.is_only_domain(domain));

        Ok(Answer::Ready(holds))
    }
}
