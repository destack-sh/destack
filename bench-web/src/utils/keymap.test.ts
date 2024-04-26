import { parseKeymapSignature, renderKeymapSignature } from "@/utils/keymap";
import { describe, expect, test } from "vitest";

describe("keymap", () => {
  test.each([
    // empty/dangling
    ["", "error"],
    [" ", "error"],
    ["a+", "error"],
    ["+a", "error"],
    ["a ", "error"],
    // invalid key
    ["shift", "error"],
    ["sqasdf", "error"],
    // valid
    ["a", { chords: [{ key: "a", modifiers: [] }] }],
    ["ctrl+a", { chords: [{ key: "a", modifiers: ["ctrl"] }] }],
    ["/", { chords: [{ key: "/", modifiers: [] }] }],
    ["shift+ctrl+a", { chords: [{ key: "a", modifiers: ["shift", "ctrl"] }] }],
    [
      "shift+ctrl+a b",
      {
        chords: [
          { key: "a", modifiers: ["shift", "ctrl"] },
          { key: "b", modifiers: [] },
        ],
      },
    ],
    ["ctrl+space", { chords: [{ key: "space", modifiers: ["ctrl"] }] }],
  ] as [string, ReturnType<typeof parseKeymapSignature> | "error"][])("parseKeymapKey(%s)", (input, expected) => {
    try {
      expect(parseKeymapSignature(input)).toEqual(expected);
    } catch {
      expect(expected).toBe("error");
    }
  });

  test.each([
    "a",
    "ctrl+a",
    "/",
    "shift+ctrl+a",
    "shift+ctrl+a b",
    "ctrl+space",
    "shift+ctrl+space",
    "shift+ctrl+a shift+ctrl+b shift+ctrl+c",
  ])("renderKeymapKey(parseKeymapKey(%s)) == %s", (input) => {
    expect(input).toBe(renderKeymapSignature(parseKeymapSignature(input)));
  });
});
