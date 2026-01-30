/// Clamp a value to a minimum.
pub(crate) fn clamp_min(value: i64, min: i64) -> i64 {
    // compare against minimum
    if value < min {
        return min;
    }

    // return original value
    value
}

/// Compute the absolute value with wrapping semantics.
pub(crate) fn abs_i64(value: i64) -> i64 {
    // handle negative input
    if value < 0 {
        return value.wrapping_neg();
    }

    // return original value
    value
}

/// Compare two byte slices with memcmp semantics.
pub(crate) fn memcmp_bytes(left: &[u8], right: &[u8]) -> i32 {
    // walk each byte pair
    let mut index = 0usize;
    while index < left.len() {
        // compare byte values
        let left_byte = left[index];
        let right_byte = right[index];

        // return on first difference
        match left_byte.cmp(&right_byte) {
            std::cmp::Ordering::Less => return -1,
            std::cmp::Ordering::Greater => return 1,
            std::cmp::Ordering::Equal => {}
        }

        // advance cursor
        index += 1;
    }

    // return equal
    0
}

/// Mix an accumulator with a seed for lightweight post processing.
pub(crate) fn mix_result(value: i64, seed: i64) -> i64 {
    // combine base values
    let step0 = value
        .wrapping_add(seed.wrapping_mul(7))
        .wrapping_add(3);
    let step1 = step0 ^ (value >> 3);

    // fold in a masked seed
    let masked = seed & 255;
    step1
        .wrapping_mul(33)
        .wrapping_add(masked)
}
