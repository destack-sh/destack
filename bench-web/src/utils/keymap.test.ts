import { parseKeymapKey, renderKeymapKey } from "@/utils/keymap";
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
    ["ctrl+shift+a", { chords: [{ key: "a", modifiers: ["ctrl", "shift"] }] }],
    [
      "ctrl+shift+a b",
      {
        chords: [
          { key: "a", modifiers: ["ctrl", "shift"] },
          { key: "b", modifiers: [] },
        ],
      },
    ],
    ["ctrl+space", { chords: [{ key: "space", modifiers: ["ctrl"] }] }],
  ] as [string, ReturnType<typeof parseKeymapKey> | "error"][])("parseKeymapKey(%s)", (input, expected) => {
    try {
      expect(parseKeymapKey(input)).toEqual(expected);
    } catch {
      expect(expected).toBe("error");
    }
  });

  test.each([
    "a",
    "ctrl+a",
    "/",
    "ctrl+shift+a",
    "ctrl+shift+a b",
    "ctrl+space",
    "ctrl+shift+space",
    "ctrl+shift+a ctrl+shift+b ctrl+shift+c",
  ])("renderKeymapKey(parseKeymapKey(%s)) == %s", (input) => {
    expect(input).toBe(renderKeymapKey(parseKeymapKey(input)));
  });
});
