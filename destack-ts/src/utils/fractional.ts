// see https://observablehq.com/@dgreensp/implementing-fractional-indexing
//  (which is licensed as CC-0)
// sync with fractional.py in backend

// base digits in lexiographical order
const _BASE_95_DIGITS =
  "!#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[]^_`abcdefghijklmnopqrstuvwxyz{|}~";

export const INTEGER_ZERO = "a0";
export const SMALLEST_INTEGER = "A00000000000000000000000000";

function _getIntegerLength(head: string) {
  if (head >= "a" && head <= "z") {
    return head.charCodeAt(0) - "a".charCodeAt(0) + 2;
  } else if (head >= "A" && head <= "Z") {
    return "Z".charCodeAt(0) - head.charCodeAt(0) + 2;
  } else {
    throw new Error(`invalid order key head: ${head}`);
  }
}

function _validateInteger(int: string) {
  if (int.length !== _getIntegerLength(int.charAt(0))) {
    throw new Error(`invalid integer part of order key: ${int}`);
  }
}

// `a` may be empty string, `b` is null or non-empty string.
// `a < b` lexicographically if `b` is non-null.
// no trailing zeros allowed.
function _midpoint(a: string, b: string | null): string {
  if (b !== null && a >= b) {
    throw new Error(`${a} >= ${b}`);
  }
  if (a.slice(-1) === "0" || (b && b.slice(-1) === "0")) {
    throw new Error("trailing zero");
  }
  if (b) {
    // remove longest common prefix.  pad `a` with 0s as we
    // go.  note that we don't need to pad `b`, because it can't
    // end before `a` while traversing the common prefix.
    let n = 0;
    while ((a.charAt(n) || "0") === b.charAt(n)) {
      n++;
    }
    if (n > 0) {
      return b.slice(0, n) + _midpoint(a.slice(n), b.slice(n));
    }
  }
  // first digits (or lack of digit) are different
  const digitA = a ? _BASE_95_DIGITS.indexOf(a.charAt(0)) : 0;
  const digitB = b !== null ? _BASE_95_DIGITS.indexOf(b.charAt(0)) : _BASE_95_DIGITS.length;
  if (digitB - digitA > 1) {
    const midDigit = Math.round(0.5 * (digitA + digitB));
    return _BASE_95_DIGITS.charAt(midDigit);
  } else {
    // first digits are consecutive
    if (b && b.length > 1) {
      return b.slice(0, 1);
    } else {
      // `b` is null or has length 1 (a single digit).
      // the first digit of `a` is the previous digit to `b`,
      // or 9 if `b` is null.
      // given, for example, midpoint('49', '5'), return
      // '4' + midpoint('9', null), which will become
      // '4' + '9' + midpoint('', null), which is '495'
      return _BASE_95_DIGITS.charAt(digitA) + _midpoint(a.slice(1), null);
    }
  }
}

// note that this may return null, as there is a largest integer
export function incrementInteger(x: string): string | null {
  _validateInteger(x);
  const [head, ...digs] = x.split("");
  let carry = true;
  for (let i = digs.length - 1; carry && i >= 0; i--) {
    const d = _BASE_95_DIGITS.indexOf(digs[i]) + 1;
    if (d === _BASE_95_DIGITS.length) {
      digs[i] = "0";
    } else {
      digs[i] = _BASE_95_DIGITS.charAt(d);
      carry = false;
    }
  }
  if (carry) {
    if (head === "Z") {
      return "a0";
    }
    if (head === "z") {
      return null;
    }
    const h = String.fromCharCode(head.charCodeAt(0) + 1);
    if (h > "a") {
      digs.push("0");
    } else {
      digs.pop();
    }
    return h + digs.join("");
  } else {
    return head + digs.join("");
  }
}

// note that this may return null, as there is a smallest integer
export function decrementInteger(x: string): string | null {
  _validateInteger(x);
  const [head, ...digs] = x.split("");
  let borrow = true;
  for (let i = digs.length - 1; borrow && i >= 0; i--) {
    const d = _BASE_95_DIGITS.indexOf(digs[i]) - 1;
    if (d === -1) {
      digs[i] = _BASE_95_DIGITS.slice(-1);
    } else {
      digs[i] = _BASE_95_DIGITS.charAt(d);
      borrow = false;
    }
  }
  if (borrow) {
    if (head === "a") {
      return `Z${_BASE_95_DIGITS.slice(-1)}`;
    }
    if (head === "A") {
      return null;
    }
    const h = String.fromCharCode(head.charCodeAt(0) - 1);
    if (h < "Z") {
      digs.push(_BASE_95_DIGITS.slice(-1));
    } else {
      digs.pop();
    }
    return h + digs.join("");
  } else {
    return head + digs.join("");
  }
}

function _getIntegerPart(key: string) {
  const integerPartLength = _getIntegerLength(key.charAt(0));
  if (integerPartLength > key.length) {
    throw new Error(`invalid order key: ${key}`);
  }
  return key.slice(0, integerPartLength);
}

export function isValidOrderKey(key: string) {
  if (key === SMALLEST_INTEGER) return false;
  // getIntegerPart will throw if the first character is bad,
  // or the key is too short.  we'd call it to check these things
  // even if we didn't need the result
  try {
    const i = _getIntegerPart(key);
    const f = key.slice(i.length);
    if (f.slice(-1) === "0") return false;
    return true;
  } catch (e) {
    return false;
  }
}

export function validateOrderKey(key: string) {
  if (!isValidOrderKey(key)) {
    throw new Error(`invalid order key: ${key}`);
  }
}

// `a` is an order key or null (START).
// `b` is an order key or null (END).
// `a < b` lexicographically if both are non-null.
export function getOrderKey(
  a: string | null,
  b: string | null,
): string {
  if (a != null) validateOrderKey(a);
  if (b != null) validateOrderKey(b);
  if (a != null && b != null && a >= b) throw new Error(`${a} >= ${b}`);
  if (a == null && b == null) return INTEGER_ZERO;

  if (a == null) {
    b = b as string; // b can't be null here (see if above)
    const ib = _getIntegerPart(b);
    const fb = b.slice(ib.length);
    if (ib === SMALLEST_INTEGER) {
      return ib + _midpoint("", fb);
    }
    // decrement(ib) can't be null here since ib != SMALLEST_INTEGER
    return ib < b ? ib : (decrementInteger(ib) as string);
  }
  if (b == null) {
    const ia = _getIntegerPart(a);
    const fa = a.slice(ia.length);
    const i = incrementInteger(ia);
    return i === null ? ia + _midpoint(fa, null) : i;
  }
  const ia = _getIntegerPart(a);
  const fa = a.slice(ia.length);
  const ib = _getIntegerPart(b);
  const fb = b.slice(ib.length);
  if (ia === ib) {
    return ia + _midpoint(fa, fb);
  }
  // increment(ia) can'tbe null here since ia < ib < END
  const i = incrementInteger(ia) as string;
  return i < b ? i : ia + _midpoint(fa, null);
}

// same preconditions as generateKeysBetween.
// n >= 0.
// Returns an array of n distinct keys in sorted order.
// If a and b are both null, returns [a0, a1, ...]
// If one or the other is null, returns consecutive "integer" keys.
// Otherwise, returns relatively short keys between a and b.
export function getOrderKeys(
  a: string | null,
  b: string | null,
  n: number,
): string[] {
  if (n === 0) return [];
  if (n === 1) return [getOrderKey(a, b)];

  if (b === null) {
    let c = getOrderKey(a, b);
    const result = [c];
    for (let i = 0; i < n - 1; i++) {
      c = getOrderKey(c, b);
      result.push(c);
    }
    return result;
  }
  if (a === null) {
    let c = getOrderKey(a, b);
    const result = [c];
    for (let i = 0; i < n - 1; i++) {
      c = getOrderKey(a, c);
      result.push(c);
    }
    result.reverse();
    return result;
  }
  const mid = Math.floor(n / 2);
  const c = getOrderKey(a, b);
  return [...getOrderKeys(a, c, mid), c, ...getOrderKeys(c, b, n - mid - 1)];
}
