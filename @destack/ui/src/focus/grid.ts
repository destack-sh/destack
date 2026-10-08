import type { Direction } from "@destack/locale";
import { type Accessor, createMemo } from "@destack/view";
import type { Collection, CollectionSection } from "../collection/index.ts";
import type { KeyboardDelegate } from "./focus.ts";

/** The step a key moves the focus by within a grid, or the end of the row or grid it moves the focus to. */
export type GridMove =
    | "next"
    | "previous"
    | "up"
    | "down"
    | "pageUp"
    | "pageDown"
    | "rowStart"
    | "rowEnd"
    | "gridStart"
    | "gridEnd";

/** A row of a grid: a section's heading, or a row of its keys a column count wide. */
export type GridRow =
    | { readonly kind: "heading"; readonly key: string; readonly label: string }
    | { readonly kind: "cells"; readonly key: string; readonly keys: readonly string[] };

/** A key's place in a grid: its row among every row and its column. */
export interface GridPlace {
    /** The row among every row, headings included. */
    readonly row: number;
    /** The column within the row. */
    readonly column: number;
}

/** The rows of a grid and the place of each key. */
interface GridLayout {
    /** The rows: each section's heading, then its keys a column count at a time. */
    readonly rows: readonly GridRow[];
    /** The place of each key. */
    readonly places: ReadonlyMap<string, GridPlace>;
}

/** The keys of a collection laid out by section in rows of a column count. */
export class GridDelegate<Item> implements KeyboardDelegate<string> {
    /** The collection whose keys the grid lays out. */
    readonly collection: Collection<Item>;
    /** The rows and the place of each key, laid out again as the keys or columns change. */
    readonly #layout: Accessor<GridLayout>;

    /** Lay out a collection's sections in rows of a column count. */
    constructor(collection: Collection<Item>, columns: Accessor<number>) {
        this.collection = collection;
        this.#layout = createMemo(() =>
            layOut(collection.sections(), collection, Math.max(1, columns())),
        );
    }

    /** The rows: each section's heading, then its keys a column count at a time. */
    rows(): readonly GridRow[] {
        return this.#layout().rows;
    }

    /** Read a key's place in the grid, undefined for a key of no item. */
    place(key: string): GridPlace | undefined {
        return this.#layout().places.get(key);
    }

    /** Read the key a key press moves the focus to: along the keys, across rows in the column, and to the ends of the row or grid. */
    target(event: KeyboardEvent, from: string, direction: Direction): string | undefined {
        // read the place the move starts from
        const move = gridMoveOf(event, direction);
        const index = this.collection.index(from);
        const place = this.place(from);
        if (move === undefined || index === undefined || place === undefined) {
            return undefined;
        }

        // step through the keys in order, and across the rows in the column
        const last = this.collection.size() - 1;
        if (move === "next") {
            return this.collection.at(Math.min(index + 1, last));
        } else if (move === "previous") {
            return this.collection.at(Math.max(index - 1, 0));
        } else if (move === "down") {
            return this.#inColumn(place.row, 1, place.column);
        } else if (move === "up") {
            return this.#inColumn(place.row, -1, place.column);
        }
        // the ends of the row or of the grid
        else if (move === "rowStart") {
            return this.#cells(place.row)?.[0];
        } else if (move === "rowEnd") {
            return this.#cells(place.row)?.at(-1);
        } else if (move === "gridStart") {
            return this.collection.at(0);
        } else if (move === "gridEnd") {
            return this.collection.at(last);
        }
        // page moves a grid leaves to its viewport
        else {
            return undefined;
        }
    }

    /** Read the first key. */
    first(): string | undefined {
        return this.collection.at(0);
    }

    /** Report whether a key is one of the grid's. */
    has(key: string): boolean {
        return this.collection.has(key);
    }

    /** Read the keys of a row of cells, undefined for a heading or past the ends. */
    #cells(row: number): readonly string[] | undefined {
        const found = this.rows()[row];

        return found?.kind === "cells" ? found.keys : undefined;
    }

    /** Read the key in a column of the next row of cells up or down, the row's last for a row too short. */
    #inColumn(row: number, step: 1 | -1, column: number): string | undefined {
        // skip the headings between rows of cells
        const rows = this.rows();
        for (let next = row + step; next >= 0 && next < rows.length; next += step) {
            const keys = this.#cells(next);
            if (keys !== undefined) {
                return keys[Math.min(column, keys.length - 1)];
            }
        }

        return undefined;
    }
}

/** Read the move a key asks for in a grid, mirrored left and right in right-to-left text, undefined for a key that moves nothing. */
export function gridMoveOf(event: KeyboardEvent, direction: Direction): GridMove | undefined {
    // step along a row in the reading direction, or across the rows
    const forward = direction === "rtl" ? "ArrowLeft" : "ArrowRight";
    const backward = direction === "rtl" ? "ArrowRight" : "ArrowLeft";
    const isControl = event.ctrlKey || event.metaKey;
    if (event.key === forward) {
        return "next";
    } else if (event.key === backward) {
        return "previous";
    } else if (event.key === "ArrowDown") {
        return "down";
    } else if (event.key === "ArrowUp") {
        return "up";
    }
    // jump a page of rows
    else if (event.key === "PageDown") {
        return "pageDown";
    } else if (event.key === "PageUp") {
        return "pageUp";
    }
    // the ends of the row, or of the grid with Control
    else if (event.key === "Home") {
        return isControl ? "gridStart" : "rowStart";
    } else if (event.key === "End") {
        return isControl ? "gridEnd" : "rowEnd";
    }
    // keys a grid leaves alone
    else {
        return undefined;
    }
}

/** Lay sections out as rows: each section's heading, then its keys a width at a time. */
function layOut<Item>(
    sections: readonly CollectionSection<Item>[],
    collection: Collection<Item>,
    width: number,
): GridLayout {
    // lay out every section, placing each key in its row and column
    const rows: GridRow[] = [];
    const places = new Map<string, GridPlace>();
    for (const section of sections) {
        // head a section that has items with its name
        const keys = section.items.map((item) => collection.keyOf(item));
        if (section.label !== undefined && keys.length > 0) {
            rows.push({ kind: "heading", key: section.key, label: section.label });
        }

        // split its keys into rows of the width
        for (let start = 0; start < keys.length; start += width) {
            const row = keys.slice(start, start + width);
            for (const [column, key] of row.entries()) {
                places.set(key, { row: rows.length, column });
            }
            rows.push({ kind: "cells", key: `${section.key}-${String(start)}`, keys: row });
        }
    }

    return { rows, places };
}
