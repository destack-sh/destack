/// One bitmap set for byte values below 64.
#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash)]
pub(crate) struct SmallCharSet {
    /// The bitset payload.
    pub(crate) bits: u64,
}

impl SmallCharSet {
    /// Return whether one small byte value is in the set.
    #[inline]
    fn contains(&self, value: u8) -> bool {
        0 != (self.bits & (1 << (value as usize)))
    }

    /// Count the leading bytes that are not in the set.
    pub(crate) fn nonmember_prefix_len(&self, buffer: &str) -> u32 {
        let mut count = 0;

        // scan the leading run
        for byte in buffer.bytes() {
            if byte >= 64 || !self.contains(byte) {
                count += 1;
            } else {
                break;
            }
        }

        count
    }
}
