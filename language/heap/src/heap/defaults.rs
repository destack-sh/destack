/// The default payload size that spills into one dedicated large span.
///
/// 4 KiB matches one typical hardware page and is the first size where
/// isolating one payload into its own durable leaf is usually worth it.
pub(crate) const DEFAULT_LARGE_SPAN_TARGET_BYTES: usize = 4 * 1024;
