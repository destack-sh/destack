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

test("render every row without a viewport, headings and cells numbered for assistive technology and picked on click", () => {
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
                components={{
                    SectionHeader: (heading) => (
                        <div {...omit(heading, "label")}>{heading.label}</div>
                    ),
                    Row: (row) => <div {...row} />,
                    Cell: (cell) => <div {...omit(cell, "item", "isActive")}>{cell.item}</div>,
                }}
            />
        );
    });
    flush();
    container.querySelectorAll<HTMLElement>("[role=gridcell]")[1]?.click();

    expect({ markup: markup(container), picked }).toEqual({
        markup:
            '<div id="id-1" role="grid" tabindex="0" aria-rowcount="3" aria-colcount="3" aria-activedescendant="id-2-1" data-slot="grid-list">' +
            '<div aria-hidden="true" style="--x-height: 0px;"></div>' +
            '<div role="row" aria-rowindex="1" data-index="0">Digits</div>' +
            '<div role="row" aria-rowindex="2" data-index="1">' +
            '<div id="id-2-1" role="gridcell" aria-colindex="1" aria-selected="true" aria-label="digit 1">1</div>' +
            '<div id="id-2-2" role="gridcell" aria-colindex="2" aria-selected="false" aria-label="digit 2">2</div>' +
            '<div id="id-2-3" role="gridcell" aria-colindex="3" aria-selected="false" aria-label="digit 3">3</div></div>' +
            '<div role="row" aria-rowindex="3" data-index="2">' +
            '<div id="id-2-4" role="gridcell" aria-colindex="1" aria-selected="false" aria-label="digit 4">4</div></div>' +
            '<div aria-hidden="true" style="--x-height: 0px;"></div></div>',
        picked: ["2"],
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
                    items: SYMBOLS.filter(([, name]) => name.includes(autocomplete.search())),
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
