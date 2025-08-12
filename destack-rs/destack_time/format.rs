use std::fmt;

#[inline]
/// Write HH:MM:SS.nnnnnnnnn to the provided formatter
pub(crate) fn write_hms_ns(
    f: &mut fmt::Formatter<'_>,
    hours: u64,
    minutes: u64,
    seconds: u64,
    nanos: u64,
) -> fmt::Result {
    write!(f, "{hours:02}:{minutes:02}:{seconds:02}.{nanos:09}")
}
