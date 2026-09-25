use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::CheckState;

impl CheckState<'_> {
    /// Return whether one type has a supported atomic representation.
    pub(in crate::sema) fn is_atomic_safe(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let is_atomic = match self.ty(ty)? {
            // supported scalar representations
            dir::Type::Primitive(dir::PrimitiveType::Boolean) => true,
            dir::Type::Primitive(dir::PrimitiveType::Integer(integer)) => {
                integer.width().is_none_or(|width| width <= 64)
            }
            dir::Type::Primitive(dir::PrimitiveType::Float(float)) => float.width() <= 64,

            // every other representation lacks an atomic form
            _ => false,
        };

        Ok(is_atomic)
    }
}
