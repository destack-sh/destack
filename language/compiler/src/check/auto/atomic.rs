use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{CheckState, Origin};

impl CheckState<'_> {
    /// Decide whether one type has a supported atomic representation.
    pub(in crate::check) fn satisfies_atomic_safe(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let root = self.reduce_type_head(origin, ty)?;
        let is_supported = match self.ty(root)? {
            // admit supported scalar representations
            dir::Type::Primitive(dir::PrimitiveType::Boolean) => true,
            dir::Type::Primitive(dir::PrimitiveType::Integer(integer)) => {
                integer.width().is_none_or(|width| width <= 64)
            }
            dir::Type::Primitive(dir::PrimitiveType::Float(float)) => {
                float.width().is_some_and(|width| width <= 64)
            }

            // reject types without an atomic scalar representation
            _ => false,
        };

        Ok(is_supported)
    }
}
