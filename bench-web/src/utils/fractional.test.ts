// tests

import { expect, test } from "vitest";
import { BASE_62_DIGITS, generateOrderKey } from "./fractional";

test.each([
  [null, null, "a0"],
  [null, "a0", "Zz"],
  ["a0", null, "a1"],
  ["a0", "a1", "a0V"],
  ["a0V", "a1", "a0l"],
  ["Zz", "a0", "ZzV"],
  ["Zz", "a1", "a0"],
  [null, "Y00", "Xzzz"],
  ["bzz", null, "c000"],
  ["a0", "a0V", "a0G"],
  ["a0", "a0G", "a08"],
  ["b125", "b129", "b127"],
  ["a0", "a1V", "a1"],
  ["Zz", "a01", "a0"],
  [null, "a0V", "a0"],
  [null, "b999", "b99"],
  [null, "A00000000000000000000000000", "!error"],
  [null, "A000000000000000000000000001", "A000000000000000000000000000V"],
  ["zzzzzzzzzzzzzzzzzzzzzzzzzzy", null, "zzzzzzzzzzzzzzzzzzzzzzzzzzz"],
  ["zzzzzzzzzzzzzzzzzzzzzzzzzzz", null, "zzzzzzzzzzzzzzzzzzzzzzzzzzzV"],
  ["a00", null, "!error"],
  ["a00", "a1", "!error"],
  ["0", "1", "!error"],
  ["a1", "a0", "!error"],
])("test_fractional(%s, %s) -> %s", (a: string | null, b: string | null, expected: string) => {
  try {
    expect(generateOrderKey(a, b, BASE_62_DIGITS)).toBe(expected);
  } catch {
    expect(expected).toBe("!error");
  }
});
