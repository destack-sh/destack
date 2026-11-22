import type { Direction } from "@destack/locale";
import type { Collection } from "../collection/index.ts";
import type { KeyboardDelegate } from "./focus.ts";

/** The keys of a collection laid out as columns, one per section, as a board's columns are. */
export class ColumnsDelegate<Item> implements KeyboardDelegate<string> {
    /** The collection whose sections are the columns. */
    readonly collection: Collection<Item>;

    /** Lay out a collection's sections as columns side by side. */
    constructor(collection: Collection<Item>) {
        this.collection = collection;
    }

    /** Read the key a key press moves the focus to: along a column, or across to its neighbours. */
    target(event: KeyboardEvent, from: string, direction: Direction): string | undefined {
        // find the column and row the move starts from
        const columns = this.#columns();
        const column = columns.findIndex((keys) => keys.includes(from));
        const keys = columns[column];
        if (keys === undefined) {
            return undefined;
        }
        const row = keys.indexOf(from);

        // step along the column, or to its ends
        const forward = direction === "rtl" ? "ArrowLeft" : "ArrowRight";
        const backward = direction === "rtl" ? "ArrowRight" : "ArrowLeft";
        if (event.key === "ArrowDown") {
            return keys[Math.min(row + 1, keys.length - 1)];
        } else if (event.key === "ArrowUp") {
            return keys[Math.max(row - 1, 0)];
        } else if (event.key === "Home") {
            return keys[0];
        } else if (event.key === "End") {
            return keys.at(-1);
        }
        // step across to the nearest column with keys, at the same row or its last
        else if (event.key === forward || event.key === backward) {
            const step = event.key === forward ? 1 : -1;
            const neighbour = nearest(columns, column, step);

            return neighbour?.[Math.min(row, neighbour.length - 1)];
        }

        return undefined;
    }

    /** Read the first available key. */
    first(): string | undefined {
        return this.#columns().flat()[0];
    }

    /** Report whether a key is one of the columns' available keys. */
    has(key: string): boolean {
        return this.collection.has(key) && !this.collection.isDisabled(key);
    }

    /** List each column's available keys in order. */
    #columns(): string[][] {
        return this.collection
            .sections()
            .map((section) =>
                section.items
                    .map((item) => this.collection.keyOf(item))
                    .filter((key) => !this.collection.isDisabled(key)),
            );
    }
}

/** Find the nearest column with keys from one column in a direction, skipping empty ones. */
function nearest(columns: readonly string[][], from: number, step: 1 | -1): string[] | undefined {
    for (let index = from + step; index >= 0 && index < columns.length; index += step) {
        const keys = columns[index];
        if (keys !== undefined && keys.length > 0) {
            return keys;
        }
    }

    return undefined;
}
