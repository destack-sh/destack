//! destack.core.builtin.fractional@2025.08.15.1

#![destack::generated(destack.core.builtin.fractional, file)]

use crate::FractionalIntegerError;

#[destack::generated(FractionalIntegerError, Debug, block)]
impl std::fmt::Debug for FractionalIntegerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FractionalIntegerError")
    }
}
