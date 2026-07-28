use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, answer};

impl CheckState<'_> {
    /// Decide whether one type satisfies a compiler-known auto interface.
    pub(in crate::check) fn satisfies_auto_interface(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<Answer<bool>> {
        let mut active = SmallVec::<[dir::GlobalTypeId; 8]>::new();

        match interface {
            dir::AutoInterface::DynamicSafe => self.satisfies_dynamic_safe(origin, ty, &mut active),
            dir::AutoInterface::OverwriteStable => {
                self.satisfies_overwrite_stable(origin, ty, &mut active)
            }
            dir::AutoInterface::Integer => {
                self.satisfies_scalar_marker(origin, ty, dir::ScalarDomain::Integer)
            }
            dir::AutoInterface::Float => {
                self.satisfies_scalar_marker(origin, ty, dir::ScalarDomain::Float)
            }
            dir::AutoInterface::Copy => self.satisfies_copy(origin, ty, &mut active),
            dir::AutoInterface::SharedSafe => self.satisfies_shared_safe(origin, ty),
            // TODO #Incomplete: the remaining auto interfaces never hold
            dir::AutoInterface::Unpin | dir::AutoInterface::Zeroable => Ok(Answer::Ready(false)),
            dir::AutoInterface::Concrete => self.satisfies_concrete(origin, ty),
            // derivable interfaces hold only through their generated extensions
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

    /// Decide whether one type holds only scalars of one domain.
    fn satisfies_scalar_marker(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        domain: dir::ScalarDomain,
    ) -> CompilerResult<Answer<bool>> {
        let root = answer!(self.reduce_type_head(origin, ty)?);
        let holds = match self.ty(root)? {
            // concrete builtin formats carry their matching marker
            dir::Type::Primitive(primitive) => primitive.scalar_domain() == domain,

            // rigid parameters carry markers from their declared assumptions
            dir::Type::Parameter(_) => {
                let families = answer!(self.builtin_scalar_families(origin, root)?);

                families.is_some_and(|families| families.is_only_domain(domain))
            }

            // literals, ranges, unions, and nominal types are not scalar formats
            _ => false,
        };

        Ok(Answer::Ready(holds))
    }
}
