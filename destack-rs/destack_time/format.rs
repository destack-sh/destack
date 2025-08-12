use std::fmt;

#[inline]
/// Write HH:MM:SS.ffffff to the provided formatter
pub(crate) fn write_hms_us(
    f: &mut fmt::Formatter<'_>,
    hours: i64,
    minutes: i64,
    seconds: i64,
    micros: i64,
) -> fmt::Result {
    debug_assert!(hours >= 0 && minutes >= 0 && seconds >= 0 && micros >= 0);
    write!(
        f,
        "{:02}:{:02}:{:02}.{:06}",
        hours, minutes, seconds, micros
    )
}

#[inline]
/// Write HH:MM:SS.nnnnnnnnn to the provided formatter
pub(crate) fn write_hms_ns(
    f: &mut fmt::Formatter<'_>,
    hours: u64,
    minutes: u64,
    seconds: u64,
    nanos: u64,
) -> fmt::Result {
    write!(f, "{:02}:{:02}:{:02}.{:09}", hours, minutes, seconds, nanos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_hms_us() {
        let s = format!("{}", fmt::format(format_args!("{}", H6(1, 2, 3, 4))));
        assert!(!s.is_empty());
    }

    #[test]
    fn test_format_hms_ns() {
        let s = format!("{}", fmt::format(format_args!("{}", H9(1, 2, 3, 4))));
        assert!(!s.is_empty());
    }

    struct H6(i64, i64, i64, i64);
    impl fmt::Display for H6 {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write_hms_us(f, self.0, self.1, self.2, self.3)
        }
    }

    struct H9(u64, u64, u64, u64);
    impl fmt::Display for H9 {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write_hms_ns(f, self.0, self.1, self.2, self.3)
        }
    }
}
