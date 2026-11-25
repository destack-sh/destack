import { nth } from "@destack/schema";
import type { Direction } from "@destack/locale";
import { type Accessor, createMemo } from "@destack/view";
import type { Collection, CollectionSection } from "../collection/index.ts";
import type { KeyboardDelegate } from "./focus.ts";

/** The step a key moves the focus by within a grid. */
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

/** The size of a row's cells, in pixels. */
export interface GridSize {
    /** The height of the row. */
    readonly height: number;
    /** The width of each cell. */
    readonly widths: readonly number[];
    /** The gap between them. */
    readonly gap: number;
}

/** A row of a grid: a section's heading, or a row of its keys. */
export type GridRow =
    | { readonly kind: "heading"; readonly key: string; readonly label: string }
    | {
          readonly kind: "cells";
          readonly key: string;
          readonly keys: readonly string[];
          /** The size of its cells, absent for cells of one width. */
          readonly size: GridSize | undefined;
      };

/** Some of a section's keys that stand in a row, and the size of their cells. */
export interface GridLine {
    /** The keys. */
    readonly keys: readonly string[];
    /** The size of their cells, absent for cells of one width. */
    readonly size: GridSize | undefined;
}

/** Break a section's keys into rows. */
export type GridBreaks = (keys: readonly string[]) => readonly GridLine[];

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

    /** Lay a collection's sections out in rows of a column count, or as they are broken. */
    constructor(
        collection: Collection<Item>,
        columns: Accessor<number>,
        breaks?: GridBreaks,
        hasEmptySections = false,
    ) {
        // break the keys of each section into rows
        this.collection = collection;
        this.#layout = createMemo(() =>
            layOut(
                collection.sections(),
                collection,
                breaks ?? ((keys) => linesOf(keys, Math.max(1, columns()))),
                hasEmptySections,
            ),
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

    /** Read the key a key press moves the focus to. */
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
            return this.#inColumn(place, 1);
        } else if (move === "up") {
            return this.#inColumn(place, -1);
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

    /** Read the keys in order. */
    keys(): readonly string[] {
        return this.collection.keys();
    }

    /** Report whether a key is one of the grid's. */
    has(key: string): boolean {
        return this.collection.has(key);
    }

    /** Read the keys of a row of cells, undefined for a heading or past the ends. */
    #cells(row: number): readonly string[] | undefined {
        return this.#line(row)?.keys;
    }

    /** Read a row of cells, undefined for a heading or past the ends. */
    #line(row: number): GridLine | undefined {
        const found = this.rows()[row];

        return found?.kind === "cells" ? found : undefined;
    }

    /** Read the key in a place's column of the next row of cells up or down. */
    #inColumn({ row, column }: GridPlace, step: 1 | -1): string | undefined {
        // skip the headings between rows of cells
        const from = this.#line(row);
        const rows = this.rows();
        for (let next = row + step; next >= 0 && next < rows.length; next += step) {
            const line = this.#line(next);
            if (line !== undefined) {
                return line.keys[columnBeside(line, from, column)];
            }
        }

        return undefined;
    }
}

/** Read the column of a row beside a column of another, by their widths where cells have any. */
function columnBeside(line: GridLine, from: GridLine | undefined, column: number): number {
    // take the same column among cells of one width
    if (line.size === undefined || from?.size === undefined) {
        return Math.min(column, line.keys.length - 1);
    }

    // find the cell the middle of the other stands at, the last where the row is shorter
    const { widths, gap } = from.size;
    const before = widths.slice(0, column).reduce((sum, width) => sum + width + gap, 0);
    const middle = before + nth(widths, column) / 2;
    let end = 0;
    const found = line.size.widths.findIndex((width) => {
        end += width + gap;

        return middle < end;
    });

    return found === -1 ? line.keys.length - 1 : found;
}

/** Break keys into rows of a count. */
function linesOf(keys: readonly string[], count: number): readonly GridLine[] {
    return Array.from({ length: Math.ceil(keys.length / count) }, (_line, i) => ({
        keys: keys.slice(i * count, (i + 1) * count),
        size: undefined,
    }));
}

/** Read the move a key asks for in a grid. */
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

/** Lay sections out as rows: each section's heading, then its keys as they are broken. */
function layOut<Item>(
    sections: readonly CollectionSection<Item>[],
    collection: Collection<Item>,
    breaks: GridBreaks,
    hasEmptySections: boolean,
): GridLayout {
    // lay out every section, placing each key in its row and column
    const rows: GridRow[] = [];
    const places = new Map<string, GridPlace>();
    for (const section of sections) {
        // head a section that has items with its name, and one without where empty ones show
        const keys = section.items.map((item) => collection.keyOf(item));
        if (section.label !== undefined && (keys.length > 0 || hasEmptySections)) {
            rows.push({ kind: "heading", key: section.key, label: section.label });
        }

        // place each key in its row
        let start = 0;
        for (const line of breaks(keys)) {
            for (const [column, key] of line.keys.entries()) {
                places.set(key, { row: rows.length, column });
            }
            rows.push({ kind: "cells", key: `${section.key}-${String(start)}`, ...line });
            start += line.keys.length;
        }
    }

    return { rows, places };
}
