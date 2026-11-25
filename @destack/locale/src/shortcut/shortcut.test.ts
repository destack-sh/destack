import { expect, expectTypeOf, test } from "@destack/test";
import { type KeyPress, Shortcut, type ShortcutText } from "./shortcut.ts";

/** Press a key with some modifiers held. */
function press(
    key: string,
    modifiers: Partial<Record<"altKey" | "ctrlKey" | "metaKey" | "shiftKey", boolean>>,
    code = `Key${key.toUpperCase()}`,
): KeyPress {
    return {
        key,
        code,
        altKey: false,
        ctrlKey: false,
        metaKey: false,
        shiftKey: false,
        ...modifiers,
    };
}

test("match key presses against shortcuts, mod being Command on Command platforms and Control elsewhere, and Option letters by their key", () => {
    expect([
        Shortcut.matches("mod+k", press("k", { metaKey: true }), true),
        Shortcut.matches("mod+k", press("k", { ctrlKey: true }), true),
        Shortcut.matches("mod+k", press("k", { ctrlKey: true }), false),
        Shortcut.matches("alt+space", press(" ", { altKey: true }), true),
        Shortcut.matches("mod+shift+k", press("K", { metaKey: true, shiftKey: true }), true),
        Shortcut.matches("mod+k", press("k", { metaKey: true, shiftKey: true }), true),
        Shortcut.matches("alt+d", press("∂", { altKey: true }, "KeyD"), true),
        Shortcut.matches("alt+q", press("a", { altKey: true }, "KeyQ"), false),
        Shortcut.safeParse("mod+shift+k").success,
        Shortcut.safeParse("hyper+k").success,
    ]).toEqual([true, false, true, true, true, false, true, false, true, false]);
});

test("write shortcuts with Command platform symbols, and with words joined by plus signs elsewhere", () => {
    expect([
        Shortcut.format("mod+k", true),
        Shortcut.format("mod+k", false),
        Shortcut.format("alt+shift+d", true),
        Shortcut.format("alt+shift+d", false),
        Shortcut.format("mod+enter", true),
        Shortcut.format("up", false),
    ]).toEqual(["⌘K", "Ctrl+K", "⌥⇧D", "Alt+Shift+D", "⌘↵", "↑"]);
});

test("label a shortcut's keys as each platform prints them, Command platforms by their user agent", () => {
    expect([
        Shortcut.labels("mod+shift+k", true),
        Shortcut.labels("mod+shift+k", false),
        Shortcut.format("mod+enter", false),
        Shortcut.format("alt+space", true),
        Shortcut.format("alt+space", false),
        Shortcut.isCommand("Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5)"),
        Shortcut.isCommand("Mozilla/5.0 (Windows NT 10.0; Win64; x64)"),
    ]).toEqual([
        ["⌘", "⇧", "K"],
        ["Ctrl", "Shift", "K"],
        "Ctrl+Enter",
        "⌥Space",
        "Alt+Space",
        true,
        false,
    ]);
});

test("match a press by one shortcut alone", () => {
    const presses = {
        backspace: press("Backspace", {}, "Backspace"),
        delete: press("Delete", {}, "Delete"),
        equals: press("=", {}, "Equal"),
        plus: press("+", {}, "BracketRight"),
        shiftedPlus: press("+", { shiftKey: true }, "Equal"),
    };
    const shortcuts = ["backspace", "delete", "=", "shift+=", "plus"];
    const matched = Object.entries(presses).map(([name, event]) => [
        name,
        shortcuts.filter((each) => Shortcut.matches(each, event, true)),
    ]);

    expect(Object.fromEntries(matched)).toEqual({
        backspace: ["backspace"],
        delete: ["delete"],
        equals: ["="],
        plus: ["plus"],
        shiftedPlus: ["plus"],
    });
});

test("write plus as a key, refusing a shift beside it", () => {
    expect([
        Shortcut.format("mod+plus", true),
        Shortcut.format("mod+plus", false),
        Shortcut.aria("mod+plus", false),
        Shortcut.aria("space", false),
        Shortcut.safeParse("mod+plus").success,
        Shortcut.safeParse("mod+shift+plus").success,
        Shortcut.safeParse("shift+mod+plus").success,
    ]).toEqual(["⌘+", "Ctrl++", "Control+Plus", "Space", true, false, false]);
});

test("read and write strokes pressed after one another", () => {
    expect({
        strokes: Shortcut.strokes("mod+k mod+s"),
        formats: [Shortcut.format("g m", true), Shortcut.format("mod+k mod+s", false)],
        aria: [Shortcut.aria("mod+k", false), Shortcut.aria("g m", false)],
        parsed: ["g m", "mod+k mod+s", "g  m", "g hyper+m", " g"].map(
            (each) => Shortcut.safeParse(each).success,
        ),
    }).toEqual({
        strokes: ["mod+k", "mod+s"],
        formats: ["G M", "Ctrl+K Ctrl+S"],
        aria: ["Control+K", undefined],
        parsed: [true, true, false, false, false],
    });
});

test("check literal strokes pressed after one another", () => {
    expectTypeOf<ShortcutText<"g m">>().toEqualTypeOf<"g m">();
    expectTypeOf<ShortcutText<"mod+k mod+s">>().toEqualTypeOf<"mod+k mod+s">();
    expectTypeOf<ShortcutText<"g hyper+m">>().toEqualTypeOf<never>();
    expectTypeOf<ShortcutText<"g  m">>().toEqualTypeOf<never>();
});
