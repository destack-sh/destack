import type { KeyPress } from "@destack/locale";
import { expect, test } from "@destack/test";
import { flush } from "solid-js";
import { type ShortcutBinding, ShortcutSequence } from "./sequence.ts";

/** The shortcuts of a page: one that goes on to two, and one alone. */
const BINDINGS: readonly ShortcutBinding<string>[] = [
    { key: "g m", target: "mine" },
    { key: "g i", target: "inbox" },
    { key: "m", target: "mark" },
    { key: "mod+k mod+s", target: "keys" },
];

/** Press a key at a time, with the command key held where asked. */
function press(key: string, timeStamp = 0, metaKey = false): KeyPress & { timeStamp: number } {
    const modifiers = { altKey: false, ctrlKey: false, shiftKey: false, metaKey };

    return { key, code: `Key${key.toUpperCase()}`, timeStamp, ...modifiers };
}

test("run a shortcut by its strokes, telling what may follow the first", () => {
    const sequence = new ShortcutSequence<string>();
    const first = sequence.press(press("g"), BINDINGS, true);
    flush();
    const progress = sequence.progress();
    const second = sequence.press(press("m", 100), BINDINGS, true);
    flush();

    expect({
        first,
        typed: progress?.typed,
        next: progress?.next.map(({ target }) => target),
        second,
        left: sequence.progress(),
        alone: sequence.press(press("m", 200), BINDINGS, true),
    }).toEqual({
        first: { isTaken: true },
        typed: "g",
        next: ["mine", "inbox"],
        second: { target: "mine", isTaken: true },
        left: undefined,
        alone: { target: "mark", isTaken: true },
    });
});

test("hold a modifier between strokes, and leave a key that follows none", () => {
    const sequence = new ShortcutSequence<string>();
    const pressed = [
        sequence.press(press("k", 0, true), BINDINGS, true),
        sequence.press(press("Meta", 10, true), BINDINGS, true),
        sequence.press(press("x", 20), BINDINGS, true),
        sequence.press(press("s", 30, true), BINDINGS, true),
    ];

    expect([pressed, sequence.isTyping()]).toEqual([
        [
            { isTaken: true },
            { isTaken: false },
            { isTaken: false },
            { target: "keys", isTaken: true },
        ],
        false,
    ]);
});

test("forget a stroke another does not follow in time", () => {
    const sequence = new ShortcutSequence<string>();
    sequence.press(press("g"), BINDINGS, true);

    expect(sequence.press(press("m", 2501), BINDINGS, true)).toEqual({
        target: "mark",
        isTaken: true,
    });
});

test("prefer the longer of two shortcuts that start the same", () => {
    const sequence = new ShortcutSequence<string>();
    const bindings = [...BINDINGS, { key: "g", target: "go" }];

    expect([
        sequence.press(press("g"), bindings, true),
        sequence.press(press("i", 10), bindings, true),
    ]).toEqual([{ isTaken: true }, { target: "inbox", isTaken: true }]);
});
