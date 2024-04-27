# see https://observablehq.com/@dgreensp/implementing-fractional-indexing
# (licensed as CC-0)
# sync with fractional.ts in frontend

from typing import Optional, Protocol, TypeVar, cast

from bench.utils.func import nextn

# base digits in lexicographical order
BASE_10_DIGITS = "0123456789"
BASE_62_DIGITS = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
BASE_95_DIGITS = (
    "!#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[]^_`abcdefghijklmnopqrstuvwxyz{|}~"
)

INTEGER_ZERO = "a0"
SMALLEST_INTEGER = "A00000000000000000000000000"
BIGGEST_INTEGER = "aZZZZZZZZZZZZZZZZZZZZZZZZZ"


def get_integer_length(head: str) -> int:
    if "a" <= head <= "z":
        return ord(head) - ord("a") + 2
    elif "A" <= head <= "Z":
        return ord("Z") - ord(head) + 2
    else:
        raise ValueError(f"invalid order key head: {head}")


def validate_integer(int: str) -> None:
    if len(int) != get_integer_length(int[0]):
        raise ValueError(f"invalid integer part of order key: {int}")


def midpoint(a: str, b: Optional[str], digits: str = BASE_95_DIGITS) -> str:
    """
    Gets the midpoint between two strings, `a` and `b`, in the given `digits` base.
    `a` may be empty string, `b` is null or non-empty string.
    `a < b` lexicographically if `b` is non-null.
    No trailing zeros allowed.
    """
    if b is not None and a >= b:
        raise ValueError(f"{a} >= {b}")
    if len(a) > 0 and a[-1] == "0" or (b is not None and b[-1] == "0"):
        raise ValueError("trailing zero")
    if b:
        # remove the longest common prefix.  pad `a` with 0s as we
        # go.  note that we don't need to pad `b`, because it can't
        # end before `a` while traversing the common prefix.
        n = 0
        while (a[n] if n < len(a) else "0") == b[n]:
            n += 1
        if n > 0:
            return b[:n] + midpoint(a[n:], b[n:], digits)
    # first digits (or lack of digit) are different
    digit_a = digits.index(a[0]) if a else 0
    digit_b = digits.index(b[0]) if b else len(digits)
    if digit_b - digit_a > 1:
        # use int(0.5 + ..) instead of round(..) because round(0.5) is 0 (??)
        mid_digit = int(0.5 + 0.5 * (digit_a + digit_b))
        return digits[mid_digit]
    else:
        # first digits are consecutive
        if b is not None and len(b) > 1:
            return b[0]
        else:
            # `b` is null or has length 1 (a single digit).
            # the first digit of `a` is the previous digit to `b`,
            # or 9 if `b` is null.
            # given, for example, midpoint('49', '5'), return
            # '4' + midpoint('9', null), which will become
            # '4' + '9' + midpoint('', null), which is '495'
            return digits[digit_a] + midpoint(a[1:], None, digits)


def increment_integer(x: str, digits: str = BASE_95_DIGITS) -> Optional[str]:
    """
    Increments the given integer `x` in the given `digits` base.
    Returns `None` if the result is too large.
    """
    validate_integer(x)
    head, *digs = x
    carry = True
    for i in range(len(digs) - 1, -1, -1):
        d = digits.index(digs[i]) + 1
        if d == len(digits):
            digs[i] = "0"
        else:
            digs[i] = digits[d]
            carry = False
            break
    if carry:
        if head == "Z":
            return "a0"
        if head == "z":
            return None
        h = chr(ord(head) + 1)
        if h > "a":
            digs.append("0")
        else:
            digs.pop()
        return h + "".join(digs)
    else:
        return head + "".join(digs)


def decrement_integer(x: str, digits: str = BASE_95_DIGITS) -> Optional[str]:
    """
    Decrements the given integer `x` in the given `digits` base.
    Returns `None` if the result is too small.
    """
    validate_integer(x)
    head, *digs = x
    borrow = True
    for i in range(len(digs) - 1, -1, -1):
        d = digits.index(digs[i]) - 1
        if d == -1:
            digs[i] = digits[-1]
        else:
            digs[i] = digits[d]
            borrow = False
    if borrow:
        if head == "a":
            return "Z" + digits[-1]
        if head == "A":
            return None
        h = chr(ord(head) - 1)
        if h < "Z":
            digs.append(digits[-1])
        else:
            digs.pop()
        return h + "".join(digs)
    else:
        return head + "".join(digs)


def get_integer_part(key: str) -> str:
    """
    Gets the integer part of the given order key.
    """
    integer_part_length = get_integer_length(key[0])
    if integer_part_length > len(key):
        raise ValueError(f"invalid order key: {key}")
    return key[:integer_part_length]


def is_valid_order_key(key: str) -> bool:
    if key == SMALLEST_INTEGER:
        return False
    #   getIntegerPart will throw if the first character is bad,
    #   or the key is too short.  we'd call it to check these things
    #   even if we didn't need the result
    try:
        i = get_integer_part(key)
    except ValueError:
        return False
    f = key[len(i) :]
    if len(f) > 0 and f[-1] == "0":
        return False
    return True


def _validate_order_key(key: str) -> None:
    if not is_valid_order_key(key):
        raise ValueError(f"invalid order key: {key}")


def get_order_key(a: Optional[str], b: Optional[str], digits: str = BASE_95_DIGITS) -> str:
    """
    Generates a key between the given keys `a` and `b` (inclusive) with logarithmic fraction growth.
    """
    if a is not None:
        _validate_order_key(a)
    if b is not None:
        _validate_order_key(b)
    if a is not None and b is not None and a >= b:
        raise ValueError(f"{a} >= {b}")
    if a is None:
        if b is None:
            return INTEGER_ZERO
        ib = get_integer_part(b)
        fb = b[len(ib) :]
        if ib == SMALLEST_INTEGER:
            return ib + midpoint("", fb, digits)
        return ib if ib < b else cast(str, decrement_integer(ib, digits))
    if b is None:
        ia = get_integer_part(a)
        fa = a[len(ia) :]
        i = increment_integer(ia, digits)
        return i if i is not None else ia + midpoint(fa, None, digits)
    ia = get_integer_part(a)
    fa = a[len(ia) :]
    ib = get_integer_part(b)
    fb = b[len(ib) :]
    if ia == ib:
        return ia + midpoint(fa, fb, digits)
    i = cast(str, increment_integer(ia, digits))
    return i if i < b else ia + midpoint(fa, None, digits)


def get_order_keys(
    a: Optional[str], b: Optional[str], n: int, digits: str = BASE_95_DIGITS
) -> list[str]:
    """
    Generates evenly spread n keys between the given keys `a` and `b` (inclusive).
    """
    if n == 0:
        return []
    if n == 1:
        return [get_order_key(a, b, digits)]
    if b is None:
        c = get_order_key(a, b, digits)
        result = [c]
        for i in range(n - 1):
            c = get_order_key(c, b, digits)
            result.append(c)
        return result
    if a is None:
        c = get_order_key(a, b, digits)
        result = [c]
        for i in range(n - 1):
            c = get_order_key(a, c, digits)
            result.append(c)
        result.reverse()
        return result
    mid = n // 2
    c = get_order_key(a, b, digits)
    return get_order_keys(a, c, mid, digits) + [c] + get_order_keys(c, b, n - mid - 1, digits)


INTEGER_MINUS_ONE = get_order_key(None, INTEGER_ZERO)


class HasOrderKey(Protocol):
    order_key: str


ElementT = TypeVar("ElementT", bound=HasOrderKey)


def get_key_bounds(
    elements: list[ElementT] | tuple[ElementT, ...],
    after: ElementT | None = None,
    before: ElementT | None = None,
) -> tuple[Optional[str], Optional[str]]:
    """Gets the order key bounds after the given (default to last)."""
    if after is not None:
        next_ok = nextn(e.order_key for e in elements if e.order_key > after.order_key)
        return after.order_key, next_ok
    elif before is not None:
        last_ok = nextn(e.order_key for e in reversed(elements) if e.order_key < before.order_key)
        return last_ok, before.order_key
    else:
        last_ok = nextn((e.order_key for e in reversed(elements)))
        return last_ok, None
