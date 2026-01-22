/// Messages used for runtime checks emitted during lowering.
pub(crate) struct RuntimeCheckMessages {
    /// Message for bounds check failures.
    pub(crate) bounds_check: &'static str,
    /// Message for null check failures.
    pub(crate) null_check: &'static str,
    /// Message for division by zero failures.
    pub(crate) division_by_zero: &'static str,
    /// Message for division overflow failures.
    pub(crate) division_overflow: &'static str,
    /// Message for integer overflow failures.
    pub(crate) integer_overflow: &'static str,
    /// Message for shift range failures.
    pub(crate) shift_out_of_range: &'static str,
}

/// Standard runtime check messages emitted by the compiler.
pub(crate) const RUNTIME_CHECK_MESSAGES: RuntimeCheckMessages = RuntimeCheckMessages {
    bounds_check: "bounds check failed",
    null_check: "null check failed",
    division_by_zero: "division by zero",
    division_overflow: "division overflow",
    integer_overflow: "integer overflow",
    shift_out_of_range: "shift out of range",
};
