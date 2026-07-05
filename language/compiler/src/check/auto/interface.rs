use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, ScalarFamily, answer};

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
            dir::AutoInterface::Send
            | dir::AutoInterface::Sync
            | dir::AutoInterface::Unpin
            | dir::AutoInterface::Zeroable => Ok(Answer::Ready(false)),
            dir::AutoInterface::Concrete
            | dir::AutoInterface::Clone
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
    ///
    /// The scalar markers derive like rustc's builtin marker traits:
    /// membership is classified, never declared, so every width of the
    /// domain satisfies the marker, including parameters through their
    /// bounds and static operations through their operands.
    fn satisfies_scalar_marker(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        domain: dir::ScalarDomain,
    ) -> CompilerResult<Answer<bool>> {
        // literals widen into the float marker like into float64
        let root = answer!(self.reduce_type_head(origin, ty)?);
        if domain == dir::ScalarDomain::Float
            && let dir::Type::Literal(literal) = self.ty(root)?
        {
            let float64 = dir::Type::Primitive(dir::PrimitiveType::Float(dir::FloatType::Float64));

            return Ok(Answer::Ready(literal.widens_to(&float64)));
        }

        let Some(families) = answer!(self.scalar_families(origin, ty)?) else {
            return Ok(Answer::Ready(false));
        };
        let holds = !families.is_empty()
            && families
                .iter()
                .all(|family| matches!(family, ScalarFamily::Domain(held) if *held == domain));

        Ok(Answer::Ready(holds))
    }
}
