use destack_artifact::TargetArch;
use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, answer};

use super::query::LayoutQuery;

impl LayoutQuery<'_, '_> {
    /// Return one normalized u32 static value.
    pub(super) fn static_u32(
        &mut self,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<u32>>> {
        self.static_integer(value, u32::try_from)
    }

    /// Return one static i32 value.
    pub(super) fn static_i32(
        &mut self,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<i32>>> {
        self.static_integer(value, i32::try_from)
    }

    /// Return one normalized static integer.
    pub(super) fn static_integer<T>(
        &mut self,
        value: dir::GlobalTypeId,
        convert: impl FnOnce(i64) -> Result<T, std::num::TryFromIntError> + Copy,
    ) -> CompilerResult<Answer<Option<T>>> {
        let origin = self.origin;
        let value = answer!(self.check.reduce_type_root(origin, value)?);
        let value = self.represented_type(value)?;
        let value = match self.check.ty(value)? {
            dir::Type::Literal(dir::ScalarLiteral::Integer(value)) => convert(*value).ok(),
            dir::Type::Static(value) => self.static_integer_value(*value, convert),
            _ => None,
        };

        Ok(Answer::Ready(value))
    }

    /// Return one normalized static integer value.
    pub(super) fn static_integer_value<T>(
        &self,
        value: dir::GlobalStaticId,
        convert: impl FnOnce(i64) -> Result<T, std::num::TryFromIntError>,
    ) -> Option<T> {
        let dir::StaticTerm::ScalarLiteral {
            value: dir::ScalarLiteral::Integer(value),
        } = self.check.r#static(value)
        else {
            return None;
        };

        convert(*value).ok()
    }

    /// Return the target pointer size.
    pub(super) fn target_pointer_bytes(&self) -> CompilerResult<u32> {
        let profile = self
            .check
            .compiler
            .profile(self.check.context.revision(), self.check.profile)?;
        let bytes = match profile.key.target_arch {
            Some(TargetArch::X86)
            | Some(TargetArch::Armv7)
            | Some(TargetArch::Armv6)
            | Some(TargetArch::Riscv32)
            | Some(TargetArch::Wasm32) => 4,
            _ => 8,
        };

        Ok(bytes)
    }
}

/// Return the smallest unsigned tag width fitting one case count.
pub(super) fn smallest_tag_bytes(cases: usize) -> u32 {
    match cases {
        0..=0x100 => 1,
        0x101..=0x1_0000 => 2,
        _ => 4,
    }
}

/// Align a byte offset to an alignment.
pub(super) fn align_to(value: u32, alignment: u32) -> u32 {
    if alignment <= 1 {
        return value;
    }

    value.div_ceil(alignment) * alignment
}
