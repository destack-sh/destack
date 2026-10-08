import { expect, onTestFinished, test } from "@destack/test";
import type { Direction } from "@destack/locale";
import { createEffect, createRoot, createSignal, flush } from "@destack/view";
import { draw } from "@destack/view/test";
import { Collection, type CollectionSection } from "../collection/index.ts";
import { Focus, isTypeaheadKey, type KeyboardDelegate } from "./focus.ts";
import { GridDelegate, gridMoveOf } from "./grid.ts";
import { ListDelegate, type Orientation } from "./list.ts";

/** A button of a test toolbar. */
interface Tool {
    /** The tool's name, its key and text. */
    readonly name: string;
    /** Whether the tool is unavailable. */
    readonly isDisabled?: boolean;
}

/** The tools of a test toolbar, the third and fourth unavailable. */
const TOOLS: readonly Tool[] = [
    { name: "Bold" },
    { name: "Italic" },
    { name: "Strike", isDisabled: true },
    { name: "Code", isDisabled: true },
    { name: "Link" },
];

/** The sections of a test grid: five letters and four digits, laid out three to a row. */
const CELLS: readonly CollectionSection<string>[] = [
    { key: "letters", label: "Letters", items: ["a", "b", "c", "d", "e"] },
    { key: "digits", label: "Digits", items: ["1", "2", "3", "4"] },
    { key: "none", label: "None", items: [] },
];

/** Run a function inside a reactive root disposed after the test. */
function rooted<Value>(run: () => Value): Value {
    return createRoot((dispose) => {
        onTestFinished(dispose);

        return run();
    });
}

/** Make a cancelable key press with modifiers. */
function key(name: string, modifiers: KeyboardEventInit = {}): KeyboardEvent {
    return new KeyboardEvent("keydown", { key: name, cancelable: true, ...modifiers });
}

/** Create a focus over the test toolbar in a list layout. */
function createToolbar(
    orientation: Orientation,
    options: { readonly isLooping?: boolean; readonly isTypeahead?: boolean } = {},
): Focus<string> {
    return rooted(() => {
        const tools = new Collection<Tool>({
            sections: () => [{ key: "tools", items: TOOLS }],
            key: (tool) => tool.name,
            text: (tool) => tool.name,
            isDisabled: (tool) => tool.isDisabled === true,
        });
        const delegate = new ListDelegate(tools, {
            orientation,
            isLooping: options.isLooping ?? true,
            isTypeahead: options.isTypeahead ?? false,
        });

        return new Focus({ delegate, mode: "roving" });
    });
}

/** Press keys in turn, reading the key each lands the focus on and whether the press was taken. */
function walk<Key>(
    focus: Focus<Key>,
    keys: readonly (string | readonly [string, KeyboardEventInit])[],
    direction: Direction,
): (readonly [string, Key | undefined, boolean])[] {
    return keys.map((entry) => {
        const [name, modifiers] = typeof entry === "string" ? [entry, {}] : entry;
        const event = key(name, modifiers);
        focus.move(event, direction);
        flush();

        return [name, focus.active(), event.defaultPrevented] as const;
    });
}

test("move the focus through a vertical list on the vertical arrows, Home and End, past unavailable keys and wrapping", () => {
    expect(
        walk(
            createToolbar("vertical"),
            ["ArrowDown", "ArrowDown", "ArrowDown", "ArrowUp", "End", "Home", "ArrowRight", "a"],
            "ltr",
        ),
    ).toEqual([
        ["ArrowDown", "Italic", true],
        ["ArrowDown", "Link", true],
        ["ArrowDown", "Bold", true],
        ["ArrowUp", "Link", true],
        ["End", "Link", true],
        ["Home", "Bold", true],
        ["ArrowRight", "Bold", false],
        ["a", "Bold", false],
    ]);
});

test("move the focus along a horizontal list in the reading direction, mirrored in right-to-left text", () => {
    expect([
        walk(createToolbar("horizontal"), ["ArrowRight", "ArrowLeft", "ArrowDown"], "ltr"),
        walk(createToolbar("horizontal"), ["ArrowLeft", "ArrowRight", "ArrowUp"], "rtl"),
    ]).toEqual([
        [
            ["ArrowRight", "Italic", true],
            ["ArrowLeft", "Bold", true],
            ["ArrowDown", "Bold", false],
        ],
        [
            ["ArrowLeft", "Italic", true],
            ["ArrowRight", "Bold", true],
            ["ArrowUp", "Bold", false],
        ],
    ]);
});

test("move the focus through a list on every arrow when it runs both ways, stopping at the ends unless it loops", () => {
    expect([
        walk(createToolbar("both"), ["ArrowRight", "ArrowDown", "ArrowLeft", "ArrowUp"], "ltr"),
        walk(
            createToolbar("vertical", { isLooping: false }),
            ["ArrowUp", "End", "ArrowDown"],
            "ltr",
        ),
    ]).toEqual([
        [
            ["ArrowRight", "Italic", true],
            ["ArrowDown", "Link", true],
            ["ArrowLeft", "Italic", true],
            ["ArrowUp", "Bold", true],
        ],
        [
            ["ArrowUp", "Bold", false],
            ["End", "Link", true],
            ["ArrowDown", "Link", false],
        ],
    ]);
});

test("move the focus to the next available key starting with a typed letter, wrapping around", () => {
    expect(
        walk(
            createToolbar("vertical", { isTypeahead: true }),
            ["i", "l", "c", "b", "B", ["i", { ctrlKey: true }]],
            "ltr",
        ),
    ).toEqual([
        ["i", "Italic", true],
        ["l", "Link", true],
        ["c", "Link", false],
        ["b", "Bold", true],
        ["B", "Bold", true],
        ["i", "Bold", false],
    ]);
});

test("move nothing in an empty list", () => {
    const focus = rooted(() => {
        const empty = new Collection<Tool>({
            sections: () => [],
            key: (tool) => tool.name,
            text: (tool) => tool.name,
        });
        const delegate = new ListDelegate(empty, {
            orientation: "vertical",
            isLooping: true,
            isTypeahead: false,
        });

        return new Focus({ delegate, mode: "roving" });
    });
    const event = key("ArrowDown");

    expect([focus.move(event, "ltr"), focus.active(), event.defaultPrevented]).toEqual([
        undefined,
        undefined,
        false,
    ]);
});

test("start on the initial key, follow a controlled key, and return to the initial key on reset", () => {
    // start on the initial key, then let an owner hold the focus
    const [held, setHeld] = createSignal<string | undefined>(undefined);
    const changes: string[] = [];
    const focus = rooted(() => {
        const tools = new Collection<Tool>({
            sections: () => [{ key: "tools", items: TOOLS }],
            key: (tool) => tool.name,
            text: (tool) => tool.name,
            isDisabled: (tool) => tool.isDisabled === true,
        });
        const delegate = new ListDelegate(tools, {
            orientation: "vertical",
            isLooping: true,
            isTypeahead: false,
        });

        return new Focus({
            delegate,
            mode: "virtual",
            initial: () => "Link",
            controlled: held,
            onChange: (next) => changes.push(next),
        });
    });
    const initial = focus.active();
    focus.focus("Italic");
    flush();
    const focused = focus.active();
    setHeld("Bold");
    flush();
    focus.focus("Italic");
    flush();
    const controlled = focus.active();
    setHeld(undefined);
    focus.reset();
    flush();
    const reset = focus.active();
    focus.focus("Strike");
    flush();

    // an unavailable key falls back to the first, and the owner hears of each change
    expect({
        initial,
        focused,
        controlled,
        reset,
        unavailable: focus.active(),
        changes,
        ids: [focus.descendant(), focus.id("Bold") === `${focus.prefix}-Bold`],
    }).toEqual({
        initial: "Link",
        focused: "Italic",
        controlled: "Bold",
        reset: "Link",
        unavailable: "Bold",
        changes: ["Italic", "Italic", "Strike"],
        ids: [`${focus.prefix}-Bold`, true],
    });
});

test("rerun only the readers of the keys a move leaves and lands on", () => {
    // follow how often each tool's reader runs while the focus moves twice
    const focus = createToolbar("vertical");
    const runs: Record<string, number> = {};
    rooted(() => {
        for (const tool of TOOLS) {
            runs[tool.name] = 0;
            createEffect(
                () => focus.isActive(tool.name),
                () => {
                    runs[tool.name] = (runs[tool.name] ?? 0) + 1;
                },
            );
        }
    });
    flush();
    walk(focus, ["ArrowDown", "ArrowDown"], "ltr");

    expect(runs).toEqual({ Bold: 2, Italic: 3, Strike: 1, Code: 1, Link: 2 });
});

test("give the DOM focus to the focused key's element while the collection holds the focus", () => {
    // render a toolbar whose buttons bind to the focus
    const focus = createToolbar("horizontal");
    const container = draw(() => (
        <div role="toolbar" onFocusOut={(event) => focus.focusOut(event)}>
            {TOOLS.map((tool) => {
                let element: HTMLButtonElement | undefined;
                focus.bind(
                    () => tool.name,
                    () => element,
                );

                return (
                    <button
                        ref={(button) => (element = button)}
                        onFocus={() => focus.focusIn(tool.name)}
                    >
                        {tool.name}
                    </button>
                );
            })}
        </div>
    ));
    const outside = draw(() => <input />).querySelector("input");

    // a move outside the collection leaves the DOM focus, one inside carries it along
    focus.focus("Italic");
    flush();
    const isAway = document.activeElement === document.body;
    container.querySelector("button")?.focus();
    flush();
    walk(focus, ["ArrowRight"], "ltr");
    const moved = document.activeElement?.textContent;
    outside?.focus();
    flush();
    const left = focus.isFocused();
    focus.enter("Link");
    flush();

    expect({ isAway, moved, left, entered: document.activeElement?.textContent }).toEqual({
        isAway: true,
        moved: "Italic",
        left: false,
        entered: "Link",
    });
});

test("lay sections out as headed rows of a column count, skipping empty sections", () => {
    const grid = rooted(() => {
        const cells = new Collection<string>({
            sections: () => CELLS,
            key: (cell) => cell,
            text: (cell) => cell,
        });

        return new GridDelegate(cells, () => 3);
    });

    expect({ rows: grid.rows(), place: grid.place("4") }).toEqual({
        rows: [
            { kind: "heading", key: "letters", label: "Letters" },
            { kind: "cells", key: "letters-0", keys: ["a", "b", "c"] },
            { kind: "cells", key: "letters-3", keys: ["d", "e"] },
            { kind: "heading", key: "digits", label: "Digits" },
            { kind: "cells", key: "digits-0", keys: ["1", "2", "3"] },
            { kind: "cells", key: "digits-3", keys: ["4"] },
        ],
        place: { row: 5, column: 0 },
    });
});

/** Create a focus over the test grid, three cells to a row. */
function createGrid(): Focus<string> {
    return rooted(() => {
        const cells = new Collection<string>({
            sections: () => CELLS,
            key: (cell) => cell,
            text: (cell) => cell,
        });
        const delegate: KeyboardDelegate<string> = new GridDelegate(cells, () => 3);

        return new Focus({ delegate, mode: "virtual" });
    });
}

test("move the focus along rows, across rows in its column and to the ends of the row and the grid", () => {
    expect(
        walk(
            createGrid(),
            [
                "ArrowLeft",
                "ArrowRight",
                "ArrowDown",
                "ArrowDown",
                "ArrowDown",
                "ArrowDown",
                "ArrowUp",
                "ArrowLeft",
                "Home",
                "End",
                ["End", { ctrlKey: true }],
                "ArrowRight",
                ["Home", { metaKey: true }],
                "ArrowUp",
                "PageDown",
                "PageUp",
                "Enter",
            ],
            "ltr",
        ),
    ).toEqual([
        ["ArrowLeft", "a", true],
        ["ArrowRight", "b", true],
        ["ArrowDown", "e", true],
        ["ArrowDown", "2", true],
        ["ArrowDown", "4", true],
        ["ArrowDown", "4", false],
        ["ArrowUp", "1", true],
        ["ArrowLeft", "e", true],
        ["Home", "d", true],
        ["End", "e", true],
        ["End", "4", true],
        ["ArrowRight", "4", true],
        ["Home", "a", true],
        ["ArrowUp", "a", false],
        ["PageDown", "a", false],
        ["PageUp", "a", false],
        ["Enter", "a", false],
    ]);
});

test("mirror the arrows along a grid's row in right-to-left text", () => {
    expect(walk(createGrid(), ["ArrowLeft", "ArrowLeft", "ArrowRight"], "rtl")).toEqual([
        ["ArrowLeft", "b", true],
        ["ArrowLeft", "c", true],
        ["ArrowRight", "b", true],
    ]);
});

test("read the grid move of each key, mirrored in right-to-left text and to the grid's ends with Control or Command", () => {
    // read each key in both directions
    const keys: readonly (readonly [string, KeyboardEventInit])[] = [
        ["ArrowRight", {}],
        ["ArrowLeft", {}],
        ["ArrowDown", {}],
        ["ArrowUp", {}],
        ["PageDown", {}],
        ["PageUp", {}],
        ["Home", {}],
        ["End", {}],
        ["Home", { ctrlKey: true }],
        ["End", { metaKey: true }],
        ["Enter", {}],
    ];
    const moves = (direction: Direction) =>
        keys.map(([name, modifiers]) => gridMoveOf(key(name, modifiers), direction));

    expect({ ltr: moves("ltr"), rtl: moves("rtl") }).toEqual({
        ltr: [
            "next",
            "previous",
            "down",
            "up",
            "pageDown",
            "pageUp",
            "rowStart",
            "rowEnd",
            "gridStart",
            "gridEnd",
            undefined,
        ],
        rtl: [
            "previous",
            "next",
            "down",
            "up",
            "pageDown",
            "pageUp",
            "rowStart",
            "rowEnd",
            "gridStart",
            "gridEnd",
            undefined,
        ],
    });
});

test("take printable keys for typeahead, leaving Space, named keys and shortcuts alone", () => {
    expect([
        isTypeaheadKey(key("a")),
        isTypeaheadKey(key("Z")),
        isTypeaheadKey(key("7")),
        isTypeaheadKey(key(" ")),
        isTypeaheadKey(key("Enter")),
        isTypeaheadKey(key("a", { ctrlKey: true })),
        isTypeaheadKey(key("a", { metaKey: true })),
        isTypeaheadKey(key("a", { altKey: true })),
    ]).toEqual([true, true, true, false, false, false, false, false]);
});
