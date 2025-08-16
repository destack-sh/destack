//! Fractional indexing using raw bytes (base-256).
//!
//! Invariant: stored keys contain **no 0x00 bytes**. We reserve 0x00 internally
//! as a virtual padding symbol during midpoint computation. Lexicographic byte
//! order (`Ord` for `[u8]`) is the total order.

use std::fmt;

/// Zero constant.
pub const ORDER_KEY_ZERO: &[u8] = &[0x80];

/// A position token in a sequence. Immutable, orderable, and compact.
///
/// Internally stored as `Box<[u8]>` (no 0x00 bytes). Lexicographic order of
/// the byte slice is the sort order.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Order(Box<[u8]>);

impl Order {
    /// Construct from bytes (fails if any byte is 0).
    pub fn new<B: AsRef<[u8]>>(bytes: B) -> Result<Self, OrderError> {
        let b = bytes.as_ref();
        if b.contains(&0) {
            return Err(OrderError::InvalidByteZero);
        }
        Ok(Self(b.into()))
    }

    /// Get the bytes view of the key.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Get the human-friendly hex for debugging (not order-preserving as text).
    pub fn to_hex(&self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(self.0.len() * 2);
        for &b in self.0.iter() {
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 0x0f) as usize] as char);
        }
        out
    }

    /// Get the smallest element strictly greater than `a` by minimal extension.
    pub fn next_after(a: &Order) -> Order {
        let mut v = a.0.to_vec();
        v.push(1);
        Order(v.into_boxed_slice())
    }

    /// Get the greatest element strictly less than `b`, if it exists.
    ///
    /// Decrements the last byte that is `> 0x01` and appends `0xFF` to make it
    /// the maximal key below `b`. Returns `None` if there is no predecessor
    /// within the allowed alphabet (all bytes were `0x01`).
    pub fn prev_before(b: &Order) -> Option<Order> {
        let mut v = b.0.to_vec();
        if v.is_empty() {
            return None;
        }
        let mut i = v.len();
        while i > 0 && v[i - 1] == 1 {
            i -= 1;
        }
        if i == 0 {
            return None;
        }
        v[i - 1] -= 1;
        v.truncate(i);
        v.push(0xFF);
        Some(Order(v.into_boxed_slice()))
    }

    /// Get the midpoint between `a` and `b` with logarithmic growth.
    ///
    /// Use `None` for open ends. Requires `a < b` when both are `Some`.
    ///
    /// Properties:
    /// - `between(None, None)` returns the canonical seed (`0x80`).
    /// - `between(Some(a), None)` returns a key `> a` (the lexicographic midpoint to the open end).
    /// - `between(None, Some(b))` returns a key `< b`.
    pub fn between(a: Option<&Order>, b: Option<&Order>) -> Result<Order, OrderError> {
        if let (Some(x), Some(y)) = (a, b)
            && x >= y
        {
            return Err(OrderError::InvalidComparison {
                a: x.to_hex(),
                b: y.to_hex(),
            });
        }

        let ab = a.map_or(&[][..], |r| r.as_bytes());
        let bb = b.map_or(&[][..], |r| r.as_bytes());

        // remove common prefix
        let mut i = 0usize;
        while i < ab.len() && i < bb.len() && ab[i] == bb[i] {
            i += 1;
        }

        // virtual padded digits at position i:
        // - a_pad: ab[i] if present else 0 (reserved padding)
        // - b_pad: bb[i] if present else 256 if open-end
        let a_pad: u16 = if i < ab.len() { ab[i] as u16 } else { 0 };
        let b_pad: u16 = if b.is_some() {
            if i < bb.len() { bb[i] as u16 } else { 256 }
        } else {
            256
        };

        let mut out = Vec::with_capacity(i + 1);
        out.extend_from_slice(&ab[..i]);

        if b_pad.saturating_sub(a_pad) >= 2 {
            // choose ceil midpoint; guaranteed to be in 1..=255 here
            let mid = (a_pad + b_pad).div_ceil(2); // ceil((a+b)/2)
            debug_assert!((1..=255).contains(&mid));
            out.push(mid as u8);
            return Order::new(out);
        }

        // consecutive: copy a's byte (if any) and recurse on the tail vs open end
        if i < ab.len() {
            out.push(ab[i]); // ab[i] != 0 by invariant
            let tail = Order::between(
                // construct a temporary Order from the tail slice (no zeros by invariant)
                Some(&Order(ab[i + 1..].into())),
                None,
            )?;
            out.extend_from_slice(tail.as_bytes());
            return Order::new(out);
        }

        // `a` ended; minimal extension above `a` is to append 0x01
        out.push(1);
        Order::new(out)
    }
}

/// Generate a key between `a` and `b` (open ends allowed).
pub fn get_order(a: Option<&Order>, b: Option<&Order>) -> Result<Order, OrderError> {
    Order::between(a, b)
}

/// Generate `n` evenly spread keys between `a` and `b` (inclusive-ish),
/// using recursive bisection (logarithmic fraction growth).
pub fn get_orders(a: Option<&Order>, b: Option<&Order>, n: u32) -> Result<Vec<Order>, OrderError> {
    if n == 0 {
        return Ok(vec![]);
    }
    if n == 1 {
        return Ok(vec![get_order(a, b)?]);
    }
    if b.is_none() {
        let mut c = get_order(a, b)?;
        let mut result = vec![c.clone()];
        for _ in 0..(n - 1) {
            c = get_order(Some(&c), b)?;
            result.push(c.clone());
        }
        return Ok(result);
    }
    if a.is_none() {
        let mut c = get_order(a, b)?;
        let mut result = vec![c.clone()];
        for _ in 0..(n - 1) {
            c = get_order(a, Some(&c))?;
            result.push(c.clone());
        }
        result.reverse();
        return Ok(result);
    }
    let mid = n / 2;
    let c = get_order(a, b)?;
    let mut left = get_orders(a, Some(&c), mid)?;
    let right = get_orders(Some(&c), b, n - mid - 1)?;
    let mut res = Vec::with_capacity(left.len() + 1 + right.len());
    res.append(&mut left);
    res.push(c);
    res.extend(right);
    Ok(res)
}

/// Errors for order key operations.
#[derive(Debug)]
pub enum OrderError {
    /// Provided bytes contained a 0x00.
    InvalidByteZero,
    /// a >= b when a < b was required.
    InvalidComparison { a: String, b: String },
}

impl fmt::Display for OrderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderError::InvalidByteZero => write!(f, "order contains disallowed 0x00 byte"),
            OrderError::InvalidComparison { a, b } => write!(f, "invalid comparison: {a} >= {b}"),
        }
    }
}

impl std::error::Error for OrderError {}

impl fmt::Debug for Order {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Order").field(&self.to_hex()).finish()
    }
}

impl From<Vec<u8>> for Order {
    fn from(v: Vec<u8>) -> Self {
        debug_assert!(!v.contains(&0));
        Order(v.into_boxed_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn _mk(bytes: &[u8]) -> Order {
        Order::new(bytes).unwrap()
    }

    #[test]
    fn test_seed_is_constant_and_0x80() {
        // seed is constant and 0x80
        let o = get_order(None, None).unwrap();
        assert_eq!(o.as_bytes(), ORDER_KEY_ZERO);
        assert_eq!(o.as_bytes(), &[0x80]);
    }

    #[test]
    fn test_open_right_grows() {
        // open right grows
        let a = _mk(&[0x80]);
        let b = get_order(Some(&a), None).unwrap();
        assert!(a < b);
        // midpoint between 0x80 and open-end (virtual 256) is ceil((128+256)/2)=192=0xC0
        assert_eq!(b.as_bytes(), &[0xC0]);
    }

    #[test]
    fn test_open_left_shrinks() {
        // open left shrinks
        let b = _mk(&[0x80]);
        let a = get_order(None, Some(&b)).unwrap();
        assert!(a < b);
        // midpoint between 0 and 0x80 is ceil((0+128)/2)=64=0x40
        assert_eq!(a.as_bytes(), &[0x40]);
    }

    #[test]
    fn test_between_simple_gap() {
        // between simple gap
        let a = _mk(&[0x40]);
        let b = _mk(&[0x80]);
        let m = get_order(Some(&a), Some(&b)).unwrap();
        assert!(a < m && m < b);
        assert_eq!(m.as_bytes(), &[0x60]); // ceil((0x40+0x80)/2)=0x60
    }

    #[test]
    fn test_between_consecutive_prefixes() {
        // between consecutive prefixes
        // a: 0x40,0xFF ; b: 0x41
        let a = _mk(&[0x40, 0xFF]);
        let b = _mk(&[0x41]);
        let m = get_order(Some(&a), Some(&b)).unwrap();
        assert!(a < m && m < b);
        assert_eq!(m.as_bytes(), &[0x40, 0xFF, 0x80]);
    }

    #[test]
    fn test_get_orders_balanced() {
        // get orders balanced
        let a = _mk(&[0x40]);
        let b = _mk(&[0xC0]);
        let out = get_orders(Some(&a), Some(&b), 5).unwrap();
        assert_eq!(out.len(), 5);
        for w in out.windows(2) {
            assert!(w[0] < w[1]);
        }
    }

    #[test]
    fn test_prev_and_next() {
        // prev and next
        let a = _mk(&[0x80]);
        let next = Order::next_after(&a);
        assert_eq!(next.as_bytes(), &[0x80, 0x01]);

        let b = _mk(&[0x81, 0x01, 0x01]);
        let prev = Order::prev_before(&b).unwrap();
        assert!(prev < b);
        // decrement last non-0x01 (0x81 -> 0x80), truncate, then append 0xFF
        assert_eq!(prev.as_bytes(), &[0x80, 0xFF]);
    }

    #[test]
    fn test_rejects_zero_byte() {
        // rejects zero byte
        assert!(Order::new([0x01, 0x00, 0x02]).is_err());
    }

    #[test]
    fn test_comparison_invariant() {
        // comparison invariant
        let a = get_order(None, None).unwrap();
        let b = get_order(Some(&a), None).unwrap();
        assert!(Order::between(Some(&b), Some(&a)).is_err());
    }
}
