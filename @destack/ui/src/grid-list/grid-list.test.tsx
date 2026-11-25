import { expect, test } from "@destack/test";
import { Errored, flush, omit } from "@destack/view";
import { markup, render } from "@destack/view/test";
import {
    AutocompleteContext,
    AutocompleteControl,
    AutocompleteInput,
} from "../autocomplete/index.ts";
import { type CollectionSection, Load } from "../collection/index.ts";
import { GridList, GridListControl, GridListLoading, GridListProvider } from "./index.ts";

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
                sectionHeader={(heading) => <div {...omit(heading, "label")}>{heading.label}</div>}
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
            '<div aria-hidden="true" style="--x-height: 0px;"></div>' +
            '<div data-index="0" role="row" aria-rowindex="1">Digits</div>' +
            '<div data-index="1" data-slot="grid-list-row" role="row" aria-rowindex="2">' +
            '<div style="touch-action: pan-x pan-y pinch-zoom;" id="id-2-1" role="gridcell" aria-colindex="1" aria-label="digit 1" tabindex="0">1</div>' +
            '<div style="touch-action: pan-x pan-y pinch-zoom;" id="id-2-2" role="gridcell" aria-colindex="2" aria-label="digit 2" tabindex="-1">2</div>' +
            '<div style="touch-action: pan-x pan-y pinch-zoom;" id="id-2-3" role="gridcell" aria-colindex="3" aria-label="digit 3" tabindex="-1">3</div></div>' +
            '<div data-index="2" data-slot="grid-list-row" role="row" aria-rowindex="3">' +
            '<div style="touch-action: pan-x pan-y pinch-zoom;" id="id-2-4" role="gridcell" aria-colindex="1" aria-label="digit 4" tabindex="-1">4</div></div>' +
            '<div aria-hidden="true" style="--x-height: 0px;"></div></div>',
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
