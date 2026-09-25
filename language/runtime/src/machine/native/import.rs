use std::ptr::copy_nonoverlapping;

use tspp_native as native;

use super::{Error, Platform};

#[cfg(unix)]
unsafe extern "C" {
    fn rust_eh_personality();
    fn ceilf(value: f32) -> f32;
    fn ceil(value: f64) -> f64;
    fn floorf(value: f32) -> f32;
    fn floor(value: f64) -> f64;
    fn truncf(value: f32) -> f32;
    fn trunc(value: f64) -> f64;
    fn nearbyintf(value: f32) -> f32;
    fn nearbyint(value: f64) -> f64;
    fn fmaf(left: f32, right: f32, addend: f32) -> f32;
    fn fma(left: f64, right: f64, addend: f64) -> f64;
    fn fmodf(left: f32, right: f32) -> f32;
    fn fmod(left: f64, right: f64) -> f64;
    fn sinf(value: f32) -> f32;
    fn sin(value: f64) -> f64;
    fn cosf(value: f32) -> f32;
    fn cos(value: f64) -> f64;
    fn tanf(value: f32) -> f32;
    fn tan(value: f64) -> f64;
    fn asinf(value: f32) -> f32;
    fn asin(value: f64) -> f64;
    fn acosf(value: f32) -> f32;
    fn acos(value: f64) -> f64;
    fn atanf(value: f32) -> f32;
    fn atan(value: f64) -> f64;
    fn atan2f(left: f32, right: f32) -> f32;
    fn atan2(left: f64, right: f64) -> f64;
    fn cbrtf(value: f32) -> f32;
    fn cbrt(value: f64) -> f64;
    fn expf(value: f32) -> f32;
    fn exp(value: f64) -> f64;
    fn expm1f(value: f32) -> f32;
    fn expm1(value: f64) -> f64;
    fn exp2f(value: f32) -> f32;
    fn exp2(value: f64) -> f64;
    fn logf(value: f32) -> f32;
    fn log(value: f64) -> f64;
    fn log1pf(value: f32) -> f32;
    fn log1p(value: f64) -> f64;
    fn log2f(value: f32) -> f32;
    fn log2(value: f64) -> f64;
    fn log10f(value: f32) -> f32;
    fn log10(value: f64) -> f64;
    fn powf(left: f32, right: f32) -> f32;
    fn pow(left: f64, right: f64) -> f64;
}

#[cfg(unix)]
impl Platform {
    /// Patch one platform import pointer.
    pub(super) fn patch_import(
        image: *mut u8,
        byte_len: usize,
        relocation: native::ImportRelocation,
    ) -> Result<(), Error> {
        if !relocation.is_within(byte_len) {
            return Err(Error::NativeImageRange);
        }

        let destination = unsafe { image.add(relocation.offset as usize) };
        let pointer = Self::import(relocation.import);
        let bytes = pointer.to_ne_bytes();

        // SAFETY: the checked pointer range lies inside the writable mapping
        unsafe { copy_nonoverlapping(bytes.as_ptr(), destination, bytes.len()) };

        Ok(())
    }

    /// Resolve one native import in the current process.
    fn import(import: native::Import) -> usize {
        match import {
            native::Import::UnwindPersonality => rust_eh_personality as *const () as usize,
            native::Import::Memcpy => libc::memcpy as *const () as usize,
            native::Import::Memmove => libc::memmove as *const () as usize,
            native::Import::Memset => libc::memset as *const () as usize,
            native::Import::Memcmp => libc::memcmp as *const () as usize,
            native::Import::CeilF32 => ceilf as *const () as usize,
            native::Import::CeilF64 => ceil as *const () as usize,
            native::Import::FloorF32 => floorf as *const () as usize,
            native::Import::FloorF64 => floor as *const () as usize,
            native::Import::TruncF32 => truncf as *const () as usize,
            native::Import::TruncF64 => trunc as *const () as usize,
            native::Import::NearestF32 => nearbyintf as *const () as usize,
            native::Import::NearestF64 => nearbyint as *const () as usize,
            native::Import::FmaF32 => fmaf as *const () as usize,
            native::Import::FmaF64 => fma as *const () as usize,
            native::Import::RemainderF32 => fmodf as *const () as usize,
            native::Import::RemainderF64 => fmod as *const () as usize,
            native::Import::SinF32 => sinf as *const () as usize,
            native::Import::SinF64 => sin as *const () as usize,
            native::Import::CosF32 => cosf as *const () as usize,
            native::Import::CosF64 => cos as *const () as usize,
            native::Import::TanF32 => tanf as *const () as usize,
            native::Import::TanF64 => tan as *const () as usize,
            native::Import::AsinF32 => asinf as *const () as usize,
            native::Import::AsinF64 => asin as *const () as usize,
            native::Import::AcosF32 => acosf as *const () as usize,
            native::Import::AcosF64 => acos as *const () as usize,
            native::Import::AtanF32 => atanf as *const () as usize,
            native::Import::AtanF64 => atan as *const () as usize,
            native::Import::Atan2F32 => atan2f as *const () as usize,
            native::Import::Atan2F64 => atan2 as *const () as usize,
            native::Import::CbrtF32 => cbrtf as *const () as usize,
            native::Import::CbrtF64 => cbrt as *const () as usize,
            native::Import::ExpF32 => expf as *const () as usize,
            native::Import::ExpF64 => exp as *const () as usize,
            native::Import::Expm1F32 => expm1f as *const () as usize,
            native::Import::Expm1F64 => expm1 as *const () as usize,
            native::Import::Exp2F32 => exp2f as *const () as usize,
            native::Import::Exp2F64 => exp2 as *const () as usize,
            native::Import::LogF32 => logf as *const () as usize,
            native::Import::LogF64 => log as *const () as usize,
            native::Import::Log1pF32 => log1pf as *const () as usize,
            native::Import::Log1pF64 => log1p as *const () as usize,
            native::Import::Log2F32 => log2f as *const () as usize,
            native::Import::Log2F64 => log2 as *const () as usize,
            native::Import::Log10F32 => log10f as *const () as usize,
            native::Import::Log10F64 => log10 as *const () as usize,
            native::Import::PowF32 => powf as *const () as usize,
            native::Import::PowF64 => pow as *const () as usize,
        }
    }
}
