//! Fast `memchr`-like string search to find an ASCII needle.

/// Detects any zero byte in a machine word.
/// Works for 32/64-bit usizes.
#[inline(always)]
pub(crate) fn has_zero_byte(x: usize) -> bool {
    const LO: usize = usize::MAX / 0xFF; // 0x0101...
    const HI: usize = LO << 7; // 0x8080...
    (x.wrapping_sub(LO) & !x & HI) != 0
}

/// Creates a word with the given byte repeated in every position.
#[inline(always)]
pub(crate) fn repeat_byte(b: u8) -> usize {
    let v = b as usize;
    let mut x = v;
    x |= x << 8;
    if core::mem::size_of::<usize>() >= 4 {
        x |= x << 16;
    }
    if core::mem::size_of::<usize>() >= 8 {
        x |= x << 32;
    }
    x
}

/// Find the first occurrence of `needle` byte in `hay`.
/// Returns the index of the first match, or None if not found.
#[inline]
pub(crate) fn find_byte(hay: &[u8], needle: u8) -> Option<usize> {
    let n = hay.len();
    if n == 0 {
        return None;
    }

    // tiny slices: straight loop wins
    if n <= 16 {
        for (i, &b) in hay.iter().enumerate() {
            if b == needle {
                return Some(i);
            }
        }
        return None;
    }

    let ptr = hay.as_ptr();
    let usize_bytes = core::mem::size_of::<usize>();
    let mask = usize_bytes - 1;

    // align to word boundary
    let mut i = 0usize;
    let align = ((ptr as usize) & mask) ^ mask; // bytes until next alignment-1
    let head = core::cmp::min(align + 1, n);
    while i < head {
        if unsafe { *ptr.add(i) } == needle {
            return Some(i);
        }
        i += 1;
    }

    // word scanning
    let needle_word = repeat_byte(needle);
    while i + usize_bytes <= n {
        let w = unsafe { (ptr.add(i) as *const usize).read_unaligned() };
        let x = w ^ needle_word;
        if has_zero_byte(x) {
            // at least one match in this word - find exact byte
            let base = i;
            let end = base + usize_bytes;
            i = base;
            while i < end {
                if unsafe { *ptr.add(i) } == needle {
                    return Some(i);
                }
                i += 1;
            }
            // shouldn't reach here
        } else {
            i += usize_bytes;
        }
    }

    // tail
    while i < n {
        if unsafe { *ptr.add(i) } == needle {
            return Some(i);
        }
        i += 1;
    }
    None
}
