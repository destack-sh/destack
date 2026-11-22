import { expect, test } from "@destack/test";
import { Shortcut, type KeyPress } from "./shortcut.ts";

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
