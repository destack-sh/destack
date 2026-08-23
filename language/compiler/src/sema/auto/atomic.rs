use destack_dir as dir;

use crate::CompilerResult;
use crate::sema::CheckState;

impl CheckState<'_> {
    /// Decide whether one type has a supported atomic representation.
    pub(in crate::sema) fn satisfies_atomic_safe(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let is_supported = match self.ty(ty)? {
            // admit supported scalar representations
            dir::Type::Primitive(dir::PrimitiveType::Boolean) => true,
            dir::Type::Primitive(dir::PrimitiveType::Integer(integer)) => {
                integer.width().is_none_or(|width| width <= 64)
            }
            dir::Type::Primitive(dir::PrimitiveType::Float(float)) => float.width() <= 64,

            // reject types without an atomic scalar representation
            _ => false,
        };

        Ok(is_supported)
    }
}
