/// Greatest bytes carried by one byte stream item.
///
/// 64 KiB bounds decoded memory while amortizing RPC framing and storage reads.
pub(crate) const BYTE_STREAM_CHUNK_BYTE_LEN: usize = 64 * 1024;
