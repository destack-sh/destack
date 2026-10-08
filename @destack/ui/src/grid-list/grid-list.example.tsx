import { defineExample } from "@destack/package/declare";
import { createSignal } from "@destack/view";
import type { CollectionSection } from "../collection/index.ts";
import {
    GridList,
    GridListControl,
    GridListEmpty,
    GridListProvider,
    GridListViewport,
} from "./grid-list.tsx";

/** A symbol a grid list offers. */
interface MathSymbol {
    /** The symbol. */
    readonly symbol: string;
    /** Its name. */
    readonly name: string;
}

/** Math symbols in two sections, five and three of them. */
const SYMBOLS: readonly CollectionSection<MathSymbol>[] = [
    {
        key: "operators",
        label: "Operators",
        items: [
            { symbol: "±", name: "plus-minus" },
            { symbol: "×", name: "multiplication" },
            { symbol: "÷", name: "division" },
            { symbol: "≠", name: "not equal" },
            { symbol: "≈", name: "almost equal" },
        ],
    },
    {
        key: "others",
        label: "Others",
        items: [
            { symbol: "∞", name: "infinity" },
            { symbol: "∑", name: "summation" },
            { symbol: "√", name: "square root" },
        ],
    },
];

/** Render a grid list of symbol sections, three to a row, reporting the symbol it picks. */
function SymbolGrid(properties: { readonly sections: readonly CollectionSection<MathSymbol>[] }) {
    // lay the symbols out three to a row, reporting the one picked
    const [picked, setPicked] = createSignal("");
    const grid = new GridListControl<MathSymbol>({
        sections: () => properties.sections,
        key: (entry) => entry.symbol,
        text: (entry) => entry.name,
        columns: () => 3,
        onAction: (entry) => setPicked(entry.name),
    });

    return (
        <GridListProvider control={grid}>
            <GridListViewport>
                <GridListEmpty>No symbols</GridListEmpty>
                <GridList control={grid} aria-label="Symbols" label={(entry) => entry.name} />
            </GridListViewport>
            <output>{picked()}</output>
        </GridListProvider>
    );
}

/** Math symbols by section in a grid of three columns, which the arrow keys move through and Enter picks from. */
export const gridListSymbols = defineExample({
    of: GridList,
    name: "symbols",
    description:
        "math symbols by section in a grid of three columns, which the arrow keys move through and Enter picks from",
    render: () => <SymbolGrid sections={SYMBOLS} />,
});

/** A grid list without symbols, showing its empty note. */
export const gridListEmpty = defineExample({
    of: GridList,
    name: "empty",
    description: "a grid list without symbols, showing its empty note",
    render: () => <SymbolGrid sections={[]} />,
});
