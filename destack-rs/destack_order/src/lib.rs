//! Fractional indexing using raw bytes (base-256).
//!
//! Invariant: stored tails contain **no 0x00 bytes**. We reserve 0x00 internally
//! as a virtual padding symbol during midpoint computation. Lexicographic byte
//! order (`Ord` for `[u8]`) is the secondary order after the head.

use std::fmt;
use std::hash::{Hash, Hasher};

pub const ORDER_HEAD_INLINE_LENGTH: usize = 4;
pub const ORDER_TAIL_INLINE_LENGTH: usize = 4;

/// A position token in a sequence as a single u64. Immutable, orderable, and compact.
/// - head 4 bytes (32 bits):
///   incrementing integer for linear appends
/// - tail 4 bytes (32 bits)
///   big-endian, left-aligned in 32 bits; zeros on the right are padding
#[derive(Copy, Clone)]
pub struct Order(u64);

/// Errors for order key operations.
#[derive(Debug)]
pub enum OrderError {
    /// Provided tail bytes contained a 0x00.
    InvalidByteZero,
    /// a >= b when a < b was required.
    InvalidComparison { a: String, b: String },
}

impl fmt::Display for OrderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderError::InvalidByteZero => write!(f, "order tail contains disallowed 0x00 byte"),
            OrderError::InvalidComparison { a, b } => write!(f, "invalid comparison: {a} >= {b}"),
        }
    }
}

impl std::error::Error for OrderError {}

impl fmt::Debug for Order {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Order")
            .field("head", &self.head())
            .field("tail_hex", &self.to_hex())
            .finish()
    }
}

/// Zero constant for the **tail** seed.
pub const ORDER_KEY_ZERO: &[u8] = &[0x80];

impl Order {
    /// Construct an Order at head = u32, from tail bytes (fails if any byte is 0 or len>4).
    pub fn new<B: AsRef<[u8]>>(head: u32, bytes: B) -> Result<Self, OrderError> {
        Self::with_head_tail(head.to_be(), bytes)
    }

    /// Construct with explicit head and tail.
    pub fn with_head_tail<B: AsRef<[u8]>>(head: u32, bytes: B) -> Result<Self, OrderError> {
        let b = bytes.as_ref();
        if b.contains(&0) {
            return Err(OrderError::InvalidByteZero);
        }
        if b.len() > ORDER_TAIL_INLINE_LENGTH {
            return Err(OrderError::InvalidComparison {
                a: hex(b),
                b: String::from("tail too long"),
            });
        }
        Ok(Self::from_head_and_slice(head, b))
    }

    /// Get the head.
    #[inline]
    pub fn head(&self) -> u32 {
        (self.0 >> 32) as u32
    }

    /// Tail value view.
    #[inline]
    pub fn as_bytes(&self) -> OrderTail {
        self.tail()
    }

    /// Tail value view.
    #[inline]
    pub fn tail(&self) -> OrderTail {
        let t = (self.0 & 0xFFFF_FFFF) as u32;
        let b0 = ((t >> 24) & 0xFF) as u8;
        let b1 = ((t >> 16) & 0xFF) as u8;
        let b2 = ((t >> 8) & 0xFF) as u8;
        let b3 = (t & 0xFF) as u8;
        // infer length from right-side zero padding
        let mut len = 4u8;
        if b3 == 0 {
            len -= 1;
        } else {
            return OrderTail {
                buf: [b0, b1, b2, b3],
                len,
            };
        }
        if b2 == 0 {
            len -= 1;
        } else {
            return OrderTail {
                buf: [b0, b1, b2, b3],
                len,
            };
        }
        if b1 == 0 {
            len -= 1;
        } else {
            return OrderTail {
                buf: [b0, b1, b2, b3],
                len,
            };
        }
        if b0 == 0 {
            len = 0;
        }
        OrderTail {
            buf: [b0, b1, b2, b3],
            len,
        }
    }

    /// Is the tail empty?
    #[inline]
    pub fn tail_is_empty(&self) -> bool {
        (self.0 & 0xFFFF_FFFF) == 0
    }

    /// Human-friendly hex of the tail (not order-preserving as text).
    pub fn to_hex(&self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let bytes = self.tail();
        let s = bytes.as_slice();
        let mut out = String::with_capacity(s.len() * 2);
        for &b in s.iter() {
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 0x0f) as usize] as char);
        }
        out
    }

    /// Smallest element strictly greater than `a` by head append (O(1) growth).
    ///
    /// Appends do not lengthen the tail: `head = a.head + 1`, empty tail.
    ///
    /// NOTE @Cleanup: this ignores potential space in the tail to keep append fast.
    pub fn next_after(a: &Order) -> Order {
        let head = a.head().wrapping_add(1);
        Self::from_head_and_slice(head, &[])
    }

    /// Greatest element strictly less than `b`, if it exists.
    ///
    /// If tail non-empty, uses tail predecessor within the same head.
    /// If tail empty:
    ///   - if head>0, returns previous head with a maximal tail `[0xFF]`.
    ///   - if head==0, there is no predecessor.
    pub fn prev_before(b: &Order) -> Option<Order> {
        let mut head = b.head();
        let mut v = b.tail().as_slice().to_vec();
        if !v.is_empty() {
            // same-head predecessor via tail
            let mut i = v.len();
            while i > 0 && v[i - 1] == 1 {
                i -= 1;
            }
            if i == 0 {
                // no smaller tail; drop to previous head if possible
                if head == 0 {
                    return None;
                }
                head -= 1;
                v.clear();
                v.push(0xFF);
            } else {
                v[i - 1] -= 1;
                v.truncate(i);
                v.push(0xFF);
            }
            Some(Order::from_head_and_vec(head, v))
        } else {
            // empty tail
            if head == 0 {
                return None;
            }
            head -= 1;
            v.push(0xFF);
            Some(Order::from_head_and_vec(head, v))
        }
    }

    /// Midpoint between `a` and `b` with logarithmic tail growth where needed (tail capped at 4 bytes).
    /// Uses `None` for open ends. Requires `a < b` when both are `Some`.
    ///
    /// Rules:
    /// - `between(None, None)` => head=0, tail=`0x80` (seed).
    /// - Append right: `between(Some(a), None)` => head=a.head+1, empty tail.
    /// - Open left: `between(None, Some(b))` => if b.head>0 then head=b.head-1 (empty tail),
    ///   else head=b.head with tail midpoint to 0.
    /// - Same head: tail midpoint.
    /// - Gapped heads: midpoint head, empty tail.
    /// - Consecutive heads: same as append to the left head (tail midpoint to open end).
    pub fn between(a: Option<&Order>, b: Option<&Order>) -> Result<Order, OrderError> {
        if let (Some(x), Some(y)) = (a, b)
            && x >= y
        {
            return Err(OrderError::InvalidComparison {
                a: format!("head={}, tail={}", x.head(), x.to_hex()),
                b: format!("head={}, tail={}", y.head(), y.to_hex()),
            });
        }

        match (a, b) {
            (None, None) => Ok(Order::from_head_and_slice(0, ORDER_KEY_ZERO)),
            (Some(lhs), None) => {
                // pure append → head+1, empty tail
                Ok(Order::from_head_and_slice(lhs.head().wrapping_add(1), &[]))
            }
            (None, Some(rhs)) => {
                if rhs.head() > 0 {
                    // place one head before, empty tail
                    Ok(Order::from_head_and_slice(rhs.head() - 1, &[]))
                } else {
                    // no lower head, use tail midpoint against 0 within head 0
                    let rtail = rhs.tail();
                    let t =
                        tail_between_capped(None, Some(rtail.as_slice()), ORDER_TAIL_INLINE_LENGTH)
                            .ok_or_else(|| OrderError::InvalidComparison {
                                a: String::from("None"),
                                b: rhs.to_hex(),
                            })?;
                    Ok(Order::from_head_and_vec(0, t))
                }
            }
            (Some(lhs), Some(rhs)) => {
                let lh = lhs.head();
                let rh = rhs.head();
                if lh + 1 < rh {
                    // plenty of integer space → midpoint head, empty tail
                    let mid = lh + (rh - lh) / 2;
                    Ok(Order::from_head_and_slice(mid, &[]))
                } else if lh == rh {
                    // same head → tail midpoint
                    let ltail = lhs.tail();
                    let rtail = rhs.tail();
                    let lt = if lhs.tail_is_empty() {
                        None
                    } else {
                        Some(ltail.as_slice())
                    };
                    let rt = if rhs.tail_is_empty() {
                        None
                    } else {
                        Some(rtail.as_slice())
                    };
                    let t =
                        tail_between_capped(lt, rt, ORDER_TAIL_INLINE_LENGTH).ok_or_else(|| {
                            OrderError::InvalidComparison {
                                a: lhs.to_hex(),
                                b: rhs.to_hex(),
                            }
                        })?;
                    Ok(Order::from_head_and_vec(lh, t))
                } else {
                    // consecutive heads (lh + 1 == rh)
                    // try lhs head side first, if no space due to cap, try rhs side
                    let ltail = lhs.tail();
                    let lt = if lhs.tail_is_empty() {
                        None
                    } else {
                        Some(ltail.as_slice())
                    };
                    if let Some(t) = tail_between_capped(lt, None, ORDER_TAIL_INLINE_LENGTH) {
                        Ok(Order::from_head_and_vec(lh, t))
                    } else {
                        let rtail = rhs.tail();
                        let rt = if rhs.tail_is_empty() {
                            None
                        } else {
                            Some(rtail.as_slice())
                        };
                        let t = tail_between_capped(None, rt, ORDER_TAIL_INLINE_LENGTH)
                            .ok_or_else(|| OrderError::InvalidComparison {
                                a: lhs.to_hex(),
                                b: rhs.to_hex(),
                            })?;
                        Ok(Order::from_head_and_vec(rh, t))
                    }
                }
            }
        }
    }
}

/// Generate a key between `a` and `b` (open ends allowed).
pub fn get_order_between(a: Option<&Order>, b: Option<&Order>) -> Result<Order, OrderError> {
    Order::between(a, b)
}

/// Generate `n` evenly spread keys between `a` and `b` (inclusive-ish),
/// using recursive bisection (logarithmic fraction growth for tails, capped at 4 bytes).
pub fn get_orders_between(
    a: Option<&Order>,
    b: Option<&Order>,
    n: u32,
) -> Result<Vec<Order>, OrderError> {
    if n == 0 {
        return Ok(vec![]);
    }
    if n == 1 {
        return Ok(vec![get_order_between(a, b)?]);
    }
    if b.is_none() {
        let mut c = get_order_between(a, b)?;
        let mut result = vec![c];
        for _ in 0..(n - 1) {
            c = get_order_between(Some(&c), b)?;
            result.push(c);
        }
        return Ok(result);
    }
    if a.is_none() {
        let mut c = get_order_between(a, b)?;
        let mut result = vec![c];
        for _ in 0..(n - 1) {
            c = get_order_between(a, Some(&c))?;
            result.push(c);
        }
        result.reverse();
        return Ok(result);
    }
    let mid = n / 2;
    let c = get_order_between(a, b)?;
    let mut left = get_orders_between(a, Some(&c), mid)?;
    let right = get_orders_between(Some(&c), b, n - mid - 1)?;
    let mut res = Vec::with_capacity(left.len() + 1 + right.len());
    res.append(&mut left);
    res.push(c);
    res.extend(right);
    Ok(res)
}

impl From<Vec<u8>> for Order {
    /// For convenience/tests: head=0 with the provided tail (len <= 4).
    fn from(v: Vec<u8>) -> Self {
        debug_assert!(!v.contains(&0));
        debug_assert!(v.len() <= ORDER_TAIL_INLINE_LENGTH);
        Order::from_head_and_vec(0, v)
    }
}

impl PartialEq for Order {
    fn eq(&self, other: &Self) -> bool {
        self.head() == other.head() && self.tail().as_slice() == other.tail().as_slice()
    }
}
impl Eq for Order {}

impl PartialOrd for Order {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Order {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.head().cmp(&other.head()) {
            std::cmp::Ordering::Equal => self.tail().as_slice().cmp(other.tail().as_slice()),
            o => o,
        }
    }
}

impl Hash for Order {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.head().hash(state);
        self.tail().as_slice().hash(state);
    }
}

impl Order {
    #[inline]
    fn from_head_and_slice(head: u32, bytes: &[u8]) -> Self {
        debug_assert!(bytes.len() <= ORDER_TAIL_INLINE_LENGTH);
        let mut t: u32 = 0;
        for (i, &b) in bytes.iter().enumerate() {
            let shift = 24 - (i as u32) * 8;
            t |= (b as u32) << shift;
        }
        let v = ((head as u64) << 32) | (t as u64);
        Order(v)
    }

    #[inline]
    fn from_head_and_vec(head: u32, v: Vec<u8>) -> Self {
        debug_assert!(!v.contains(&0));
        debug_assert!(v.len() <= ORDER_TAIL_INLINE_LENGTH);
        Self::from_head_and_slice(head, &v)
    }
}

/// Tail midpoint helper (base-256, no 0x00 in real bytes), capped length.
fn tail_between_capped(a: Option<&[u8]>, b: Option<&[u8]>, cap: usize) -> Option<Vec<u8>> {
    let ab = a.unwrap_or(&[]);
    let bb = b.unwrap_or(&[]);
    // remove common prefix
    let mut i = 0usize;
    while i < ab.len() && i < bb.len() && ab[i] == bb[i] {
        i += 1;
    }
    // virtual digits
    let a_pad: u16 = if i < ab.len() { ab[i] as u16 } else { 0 };
    let b_pad: u16 = if b.is_some() {
        if i < bb.len() { bb[i] as u16 } else { 256 }
    } else {
        256
    };

    let mut out = Vec::with_capacity(i + 1);
    out.extend_from_slice(&ab[..i]);
    if out.len() > cap {
        return None;
    }

    if b_pad.saturating_sub(a_pad) >= 2 {
        let mid = (a_pad + b_pad).div_ceil(2);
        debug_assert!((1..=255).contains(&mid));
        out.push(mid as u8);
        if out.len() > cap {
            return None;
        }
        return Some(out);
    }

    if i < ab.len() {
        out.push(ab[i]);
        if out.len() > cap {
            return None;
        }
        let tail = tail_between_capped(Some(&ab[i + 1..]), None, cap - out.len())?;
        out.extend_from_slice(&tail);
        if out.len() > cap {
            return None;
        }
        return Some(out);
    }

    // a ended; minimal extension above a is to append 0x01
    out.push(1);
    if out.len() > cap {
        return None;
    }
    Some(out)
}

fn hex(b: &[u8]) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(b.len() * 2);
    for &x in b {
        s.push(H[(x >> 4) as usize] as char);
        s.push(H[(x & 0x0f) as usize] as char);
    }
    s
}

/// Tail small value object that views up to 4 bytes.
#[derive(Copy, Clone, Debug)]
pub struct OrderTail {
    buf: [u8; ORDER_TAIL_INLINE_LENGTH],
    len: u8,
}

impl OrderTail {
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self.buf[..self.len as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_is_constant_and_0x80() {
        // seed is head 0 and tail 0x80
        let o = get_order_between(None, None).unwrap();
        assert_eq!(o.head(), 0);
        assert_eq!(o.tail().as_slice(), ORDER_KEY_ZERO);
        assert_eq!(o.tail().as_slice(), &[0x80]);
    }

    #[test]
    fn test_open_right_appends_without_tail_growth() {
        // append to the right: head+1, empty tail
        let a = {
            let bytes: &[u8] = &[0x80];
            Order::new(0, bytes).unwrap()
        }; // head 0
        let b = get_order_between(Some(&a), None).unwrap();
        assert!(a < b);
        assert_eq!(b.head(), 1);
        assert!(b.tail_is_empty());
    }

    #[test]
    fn test_open_left_prefers_lower_head() {
        // open left: if b.head>0, choose head-1 with empty tail
        let b = Order::with_head_tail(5, [0x80]).unwrap();
        let a = get_order_between(None, Some(&b)).unwrap();
        assert!(a < b);
        assert_eq!(a.head(), 4);
        assert!(a.tail_is_empty());
    }

    #[test]
    fn test_open_left_when_b_head_zero_uses_tail() {
        // when no lower head, use tail midpoint within head 0
        let b = {
            let bytes: &[u8] = &[0x80];
            Order::new(0, bytes).unwrap()
        }; // head 0
        let a = get_order_between(None, Some(&b)).unwrap();
        assert!(a < b);
        assert_eq!(a.head(), 0);
        assert_eq!(a.tail().as_slice(), &[0x40]);
    }

    #[test]
    fn test_between_simple_gap_same_head_tail_midpoint() {
        let a = {
            let bytes: &[u8] = &[0x40];
            Order::new(0, bytes).unwrap()
        };
        let b = {
            let bytes: &[u8] = &[0x80];
            Order::new(0, bytes).unwrap()
        };
        let m = get_order_between(Some(&a), Some(&b)).unwrap();
        assert_eq!(m.head(), 0);
        assert!(a < m && m < b);
        assert_eq!(m.tail().as_slice(), &[0x60]); // ceil((0x40+0x80)/2)=0x60
    }

    #[test]
    fn test_between_consecutive_prefixes_tail_midpoint() {
        // a: 0x40,0xFF ; b: 0x41 in same head
        let a = {
            let bytes: &[u8] = &[0x40, 0xFF];
            Order::new(0, bytes).unwrap()
        };
        let b = {
            let bytes: &[u8] = &[0x41];
            Order::new(0, bytes).unwrap()
        };
        let m = get_order_between(Some(&a), Some(&b)).unwrap();
        assert_eq!(m.head(), 0);
        assert!(a < m && m < b);
        assert_eq!(m.tail().as_slice(), &[0x40, 0xFF, 0x80]); // midpoint-to-open for tail
    }

    #[test]
    fn test_between_gapped_heads_chooses_mid_head() {
        let a = Order::with_head_tail(10, []).unwrap();
        let b = Order::with_head_tail(20, []).unwrap();
        let m = get_order_between(Some(&a), Some(&b)).unwrap();
        assert!(a < m && m < b);
        assert!(m.tail_is_empty());
        assert_eq!(m.head(), 15);
    }

    #[test]
    fn test_between_consecutive_heads_stays_in_left_head_with_tail() {
        let a = Order::with_head_tail(10, []).unwrap();
        let b = Order::with_head_tail(11, []).unwrap();
        let m = get_order_between(Some(&a), Some(&b)).unwrap();
        assert!(a < m && m < b);
        assert_eq!(m.head(), 10);
        assert!(!m.tail_is_empty());
    }

    #[test]
    fn test_prev_and_next_semantics() {
        let a = {
            let bytes: &[u8] = &[0x80];
            Order::new(0, bytes).unwrap()
        }; // head 0
        let next = Order::next_after(&a);
        assert_eq!(next.head(), 1);
        assert!(next.tail_is_empty());

        let b = Order::with_head_tail(2, [0x81, 0x01, 0x01]).unwrap();
        let prev = Order::prev_before(&b).unwrap();
        assert!(prev < b);
        assert_eq!(prev.head(), 2);
        assert_eq!(prev.tail().as_slice(), &[0x80, 0xFF]);
    }

    #[test]
    fn test_rejects_zero_byte() {
        assert!(Order::new(0, [0x01, 0x00, 0x02]).is_err());
        assert!(Order::with_head_tail(7, [0x00]).is_err());
    }

    #[test]
    fn test_comparison_invariant() {
        let a = get_order_between(None, None).unwrap();
        let b = get_order_between(Some(&a), None).unwrap();
        assert!(Order::between(Some(&b), Some(&a)).is_err());
    }

    #[test]
    fn test_insert_between_full_tail() {
        let a = Order::new(0, [0xff, 0xff, 0xff, 0xff]).unwrap();
        let b = a;
        assert!(get_order_between(Some(&a), Some(&b)).is_err());
    }
}
