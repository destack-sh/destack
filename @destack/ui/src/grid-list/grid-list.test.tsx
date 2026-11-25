import { t } from "@destack/locale";
import { defineAction } from "@destack/object";
import { expect, test } from "@destack/test";
import { createSignal, Errored, flush, handle, omit } from "@destack/view";
import { markup, render } from "@destack/view/test";
import {
    AutocompleteContext,
    AutocompleteControl,
    AutocompleteInput,
} from "../autocomplete/index.ts";
import { type CollectionSection, Load } from "../collection/index.ts";
import {
    GridList,
    GridListControl,
    GridListLoading,
    type GridListMove,
    GridListProvider,
} from "./index.ts";

/** The digits of a test grid list, four of them. */
const DIGITS: readonly CollectionSection<string>[] = [
    { key: "digits", label: "Digits", items: ["1", "2", "3", "4"] },
];

/** The symbols a test search finds by name, eight in one section. */
const SYMBOLS: readonly (readonly [string, string])[] = [
    ["±", "plus-minus"],
    ["×", "multiplication"],
    ["÷", "division"],
    ["≠", "not equal"],
    ["≈", "almost equal"],
    ["∞", "infinity"],
    ["∑", "summation"],
    ["√", "square root"],
];

test("render every row without a viewport: headings, and cells numbered, tabbed to one at a time and opened on a press", () => {
    // render the digits three to a row and click the second
    const picked: string[] = [];
    const { container } = render(() => {
        const grid = new GridListControl<string>({
            sections: () => DIGITS,
            key: (digit) => digit,
            text: (digit) => digit,
            columns: () => 3,
            onAction: (digit) => picked.push(digit),
        });

        return (
            <GridList
                control={grid}
                label={(digit) => `digit ${digit}`}
                sectionHeader={(heading) => (
                    <div {...omit(heading, "label", "section", "isSticky", "xstyle")}>
                        {heading.label}
                    </div>
                )}
            >
                {(cell) => <div {...omit(cell, "item", "isActive", "isSelected")}>{cell.item}</div>}
            </GridList>
        );
    });
    flush();
    container.querySelectorAll<HTMLElement>("[role=gridcell]")[1]?.click();

    expect({ markup: markup(container), picked }).toEqual({
        markup:
            '<div data-slot="grid-list" id="id-1" role="grid" aria-rowcount="3" aria-colcount="3">' +
            '<div data-index="0" role="row" aria-rowindex="1">Digits</div>' +
            '<div data-index="1" data-slot="grid-list-row" role="row" aria-rowindex="2">' +
            '<div style="touch-action: pan-x pan-y pinch-zoom;" id="id-2-1" role="gridcell" data-item="1" aria-colindex="1" aria-label="digit 1" tabindex="0">1</div>' +
            '<div style="touch-action: pan-x pan-y pinch-zoom;" id="id-2-2" role="gridcell" data-item="2" aria-colindex="2" aria-label="digit 2" tabindex="-1">2</div>' +
            '<div style="touch-action: pan-x pan-y pinch-zoom;" id="id-2-3" role="gridcell" data-item="3" aria-colindex="3" aria-label="digit 3" tabindex="-1">3</div></div>' +
            '<div data-index="2" data-slot="grid-list-row" role="row" aria-rowindex="3">' +
            '<div style="touch-action: pan-x pan-y pinch-zoom;" id="id-2-4" role="gridcell" data-item="4" aria-colindex="1" aria-label="digit 4" tabindex="-1">4</div></div></div>',
        picked: ["2"],
    });
});

test("mark a disabled cell and refuse to open it", () => {
    // render the digits with the second disabled, and click the first two
    const picked: string[] = [];
    const { container } = render(() => {
        const grid = new GridListControl<string>({
            sections: () => DIGITS,
            key: (digit) => digit,
            text: (digit) => digit,
            columns: () => 3,
            isDisabled: (digit) => digit === "2",
            onAction: (digit) => picked.push(digit),
        });

        return <GridList control={grid} label={(digit) => `digit ${digit}`} />;
    });
    flush();
    const cells = [...container.querySelectorAll<HTMLElement>("[role=gridcell]")];
    cells[0]?.click();
    cells[1]?.click();

    expect({
        disabled: cells.map((cell) => cell.getAttribute("aria-disabled")),
        marked: cells.map((cell) => cell.dataset["disabled"]),
        picked,
    }).toEqual({
        disabled: [null, "true", null, null],
        marked: [undefined, "", undefined, undefined],
        picked: ["1"],
    });
});

test("move the focused cell through the grid's rows and columns from a search field, and pick it on Enter", () => {
    // lay the symbols a search finds out three to a row
    const picked: string[] = [];
    const { container } = render(() => {
        const autocomplete = new AutocompleteControl({});
        const grid = new GridListControl<readonly [string, string]>({
            sections: () => [
                {
                    key: "math",
                    items: SYMBOLS.filter(([, name]) => name.includes(autocomplete.value())),
                },
            ],
            key: ([symbol]) => symbol,
            text: ([, name]) => name,
            columns: () => 3,
            autocomplete,
            onAction: ([, name]) => picked.push(name),
        });

        return (
            <AutocompleteContext value={autocomplete}>
                <AutocompleteInput aria-label="Symbol" />
                <GridList control={grid} aria-label="Symbols" label={([, name]) => name} />
                <output>{grid.active()?.[1]}</output>
            </AutocompleteContext>
        );
    });
    flush();
    const input = container.querySelector("input");
    const press = (key: string) => {
        input?.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
        flush();
    };
    press("ArrowDown");
    press("ArrowRight");
    const active = container.querySelector("[role=gridcell][aria-selected=true]");
    const isDescribed = input?.getAttribute("aria-activedescendant") === active?.id;
    const shown = container.querySelector("output")?.textContent;
    press("Enter");

    expect({ picked, isDescribed, shown }).toEqual({
        picked: ["almost equal"],
        isDescribed: true,
        shown: "almost equal",
    });
});

/** Render a search over a grid list whose items wait for a load, inside an error boundary. */
function drawLoading(load: Load<readonly string[]>): HTMLElement {
    return render(() => (
        <Errored fallback={(error) => <p role="alert">{String(error())}</p>}>
            {(() => {
                const autocomplete = new AutocompleteControl({});
                const grid = new GridListControl<string>({
                    load,
                    sections: () => [{ key: "digits", items: load.value() }],
                    key: (digit) => digit,
                    text: (digit) => digit,
                    columns: () => 3,
                    autocomplete,
                    onAction: () => undefined,
                });

                return (
                    <AutocompleteContext value={autocomplete}>
                        <GridListProvider control={grid}>
                            <AutocompleteInput aria-label="Digit" />
                            <GridListLoading>Loading digits</GridListLoading>
                            <GridList control={grid} aria-label="Digits" label={(digit) => digit} />
                        </GridListProvider>
                    </AutocompleteContext>
                );
            })()}
        </Errored>
    )).container;
}

test("fill the cell a search field is on, in a list fitted to its width", () => {
    const { container } = render(() => {
        const autocomplete = new AutocompleteControl({});
        const grid = new GridListControl<readonly [string, string]>({
            sections: () => [{ key: "math", items: SYMBOLS }],
            key: ([symbol]) => symbol,
            text: ([, name]) => name,
            autocomplete,
            onAction: () => undefined,
        });

        return (
            <AutocompleteContext value={autocomplete}>
                <AutocompleteInput aria-label="Symbol" />
                <GridList control={grid} aria-label="Symbols" label={([, name]) => name} />
            </AutocompleteContext>
        );
    });
    flush();
    const looks = [...container.querySelectorAll("[data-slot=grid-list-cell]")].map(
        (cell) => cell.className,
    );

    // the first cell alone is filled, the others look alike
    expect([new Set(looks.slice(1)).size, looks.slice(1).includes(looks[0] ?? "")]).toEqual([
        1,
        false,
    ]);
});

test("take typing in the search while the items load, then show the loaded items", async () => {
    // type before the load finishes, then let it finish
    const { promise, resolve } = Promise.withResolvers<readonly string[]>();
    const container = drawLoading(new Load(promise));
    flush();
    const input = container.querySelector("input");
    if (input !== null) {
        input.value = "1";
        input.dispatchEvent(new InputEvent("input", { bubbles: true }));
    }
    flush();
    const loading = {
        search: input?.value,
        note: container.querySelector("[role=status]")?.textContent,
        cells: container.querySelectorAll("[role=gridcell]").length,
    };
    resolve(["1", "2"]);
    await promise;
    flush();

    expect({
        loading,
        loaded: {
            note: container.querySelector("[role=status]"),
            cells: [...container.querySelectorAll("[role=gridcell]")].map(
                (cell) => cell.textContent,
            ),
        },
    }).toEqual({
        loading: { search: "1", note: "Loading digits", cells: 0 },
        loaded: { note: null, cells: ["1", "2"] },
    });
});

test("throw a failed load to the nearest error boundary", async () => {
    const failed = Promise.reject(new TypeError("the digits are offline"));
    const container = drawLoading(new Load(failed));
    await failed.catch(() => undefined);
    flush();

    expect(container.querySelector("[role=alert]")?.textContent).toBe(
        "TypeError: the digits are offline",
    );
});

/** Press and release a key on the focused element. */
function pressKey(key: string, options: KeyboardEventInit = {}): void {
    for (const type of ["keydown", "keyup"]) {
        document.activeElement?.dispatchEvent(
            new KeyboardEvent(type, { key, bubbles: true, cancelable: true, ...options }),
        );
    }
    flush();
}

test("move through three columns of cards by row, select with Space and shift, and open with Enter", () => {
    // seven cards in rows of three that select several, opened ones collected
    const opened: string[] = [];
    const { container } = render(() => {
        const grid = new GridListControl<string>({
            sections: () => [{ key: "cards", items: ["a", "b", "c", "d", "e", "f", "g"] }],
            key: (card) => card,
            text: (card) => card,
            columns: () => 3,
            selection: { multiple: true, behavior: "replace" },
            onAction: (card) => opened.push(card),
        });

        return <GridList control={grid} label={(card) => card} />;
    });
    flush();
    const cards = [...container.querySelectorAll<HTMLElement>("[role=gridcell]")];
    const selected = () =>
        cards
            .filter((card) => card.getAttribute("aria-selected") === "true")
            .map((card) => card.textContent);
    cards[0]?.focus();

    // step down a row and right, select it, extend down a row, then open the focused card
    const visits: (string | null | undefined)[] = [];
    for (const key of ["ArrowDown", "ArrowRight"]) {
        pressKey(key);
        visits.push(document.activeElement?.textContent);
    }
    pressKey(" ");
    pressKey("ArrowDown", { shiftKey: true });
    const extended = selected();
    pressKey("Enter");
    const grid = container.querySelector("[role=grid]");

    expect({
        visits,
        extended,
        opened,
        isMultiple: grid?.getAttribute("aria-multiselectable"),
    }).toEqual({
        visits: ["d", "e"],
        extended: ["e", "f", "g"],
        opened: ["g"],
        isMultiple: "true",
    });
});

/** Send a mouse's pointer event to a card. */
function point(card: HTMLElement | undefined, type: string, options: PointerEventInit = {}): void {
    card?.dispatchEvent(
        new PointerEvent(type, {
            bubbles: true,
            cancelable: true,
            pointerId: 1,
            pointerType: "mouse",
            width: 1,
            height: 1,
            ...options,
        }),
    );
}

test("select a card on a press with a modifier, and open it on a plain press", () => {
    // press the second card with command held, then the third plainly
    const opened: string[] = [];
    const { container } = render(() => {
        const grid = new GridListControl<string>({
            sections: () => [{ key: "cards", items: ["a", "b", "c"] }],
            key: (card) => card,
            text: (card) => card,
            selection: { multiple: true, behavior: "replace" },
            onAction: (card) => opened.push(card),
        });

        return <GridList control={grid} label={(card) => card} />;
    });
    flush();
    const cards = [...container.querySelectorAll<HTMLElement>("[role=gridcell]")];
    for (const [card, options] of [
        [cards[1], { metaKey: true }],
        [cards[2], {}],
    ] as const) {
        point(card, "pointerdown", options);
        point(card, "pointerup", options);
    }
    flush();

    expect({
        selected: cards.map((card) => card.getAttribute("aria-selected")),
        opened,
        columns: container.querySelector("[role=grid]")?.getAttribute("aria-colcount"),
    }).toEqual({ selected: ["false", "true", "false"], opened: ["c"], columns: "1" });
});

/** Draw letters in sections whose rows move, and record what happens to them. */
function drawLetters(order: "manual" | "sorted" = "manual") {
    const moves: GridListMove[] = [];
    const peeked: string[] = [];
    const [isEmpty, setEmpty] = createSignal(false);
    const peek = defineAction({ name: "peek", title: t`Peek`, keybinding: "space" });
    const { container } = render(() => {
        const grid = new GridListControl<string>({
            sections: () => [
                { key: "first", label: "First", items: isEmpty() ? [] : ["a", "b", "c"] },
                { key: "none", label: "None", items: [] },
                { key: "last", label: "Last", items: ["d", "e"] },
            ],
            key: (letter) => letter,
            text: (letter) => letter,
            selection: { multiple: true, behavior: "replace" },
            hasEmptySections: true,
            order,
            onMove: (move) => void moves.push(move),
        });

        return (
            <div
                ref={handle(peek, {
                    isEnabled: (element) => grid.keyAt(element) !== undefined,
                    run: ({ element }) => void peeked.push(grid.keyAt(element) ?? "none"),
                })}
            >
                <GridList control={grid} label={(letter) => letter} />
            </div>
        );
    });
    flush();
    const focus = (letter: string) => {
        container.querySelector<HTMLElement>(`[data-item="${letter}"]`)?.focus();
        flush();
    };

    return { container, moves, peeked, setEmpty, focus };
}

test("head sections without items where empty ones show", () => {
    const { container, setEmpty } = drawLetters();
    const rows = () =>
        [...container.querySelectorAll("[role=row]")].map((each) => each.textContent);
    const full = rows();
    setEmpty(true);
    flush();

    expect([full, rows()]).toEqual([
        ["First", "a", "b", "c", "None", "Last", "d", "e"],
        ["First", "None", "Last", "d", "e"],
    ]);
});

test("step through the rows by j and k, and select one by x", () => {
    const { container, focus } = drawLetters();
    focus("c");
    pressKey("j");
    const next = document.activeElement?.textContent;
    pressKey("k");
    pressKey("k");
    pressKey("x");

    expect([
        next,
        document.activeElement?.textContent,
        [...container.querySelectorAll("[aria-selected=true]")].map((each) => each.textContent),
    ]).toEqual(["d", "b", ["b"]]);
});

test("move a row past the next one, and into the next section at its near end", () => {
    const { moves, focus } = drawLetters();
    const keys = { altKey: true, shiftKey: true };
    for (const [letter, key] of [
        ["a", "ArrowDown"],
        ["c", "ArrowDown"],
        ["c", "ArrowUp"],
        ["d", "ArrowUp"],
        ["a", "ArrowUp"],
    ] as const) {
        focus(letter);
        pressKey(key, keys);
    }

    expect(moves).toEqual([
        { id: "a", section: "first", previous: "b", next: "c" },
        { id: "c", section: "none", previous: undefined, next: undefined },
        { id: "c", section: "first", previous: "a", next: "b" },
        { id: "d", section: "none", previous: undefined, next: undefined },
    ]);
});

test("move the selected rows together, in their order", () => {
    const { moves, focus } = drawLetters();
    focus("a");
    pressKey("x");
    pressKey("ArrowDown", { shiftKey: true });
    pressKey("ArrowDown", { altKey: true, shiftKey: true });

    expect(moves).toEqual([
        { id: "a", section: "first", previous: "c", next: undefined },
        { id: "b", section: "first", previous: "a", next: undefined },
    ]);
});

test("move sorted rows between sections alone", () => {
    const { moves, focus } = drawLetters("sorted");
    focus("a");
    pressKey("ArrowDown", { altKey: true, shiftKey: true });

    expect(moves).toEqual([{ id: "a", section: "none", previous: undefined, next: undefined }]);
});

test("leave Space to an action bound to it around the rows", () => {
    const { container, peeked, focus } = drawLetters();
    focus("b");
    pressKey(" ");

    expect([peeked, container.querySelectorAll("[aria-selected=true]").length]).toEqual([["b"], 0]);
});

test("take the focus under the pointer from a button, but not from a field or an open overlay", () => {
    // move a mouse over the first item from each
    const { container } = render(() => {
        const grid = new GridListControl({
            sections: () => DIGITS,
            key: (digit) => digit,
            text: (digit) => digit,
            selection: { multiple: true },
            shouldFocusOnHover: true,
        });

        return (
            <>
                <button>Display</button>
                <input />
                <dialog open>
                    <button>Inside</button>
                </dialog>
                <GridListProvider control={grid}>
                    <GridList control={grid} aria-label="Digits" label={(digit) => digit} />
                </GridListProvider>
            </>
        );
    });
    flush();
    const focusAfter = (held: string): string | undefined => {
        container.querySelector<HTMLElement>(held)?.focus();
        container
            .querySelector("[data-item='1']")
            ?.dispatchEvent(
                new PointerEvent("pointermove", { bubbles: true, pointerType: "mouse" }),
            );
        flush();
        const focused = document.activeElement;

        return focused?.tagName === "INPUT" ? "field" : (focused?.textContent ?? undefined);
    };

    expect({
        button: focusAfter("button"),
        field: focusAfter("input"),
        overlay: focusAfter("dialog button"),
    }).toEqual({ button: "1", field: "field", overlay: "Inside" });
});
