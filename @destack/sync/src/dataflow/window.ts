import type { Row } from "@destack/db";

/**
 * The most entries one chunk of a window holds before it splits.
 *
 * Inserting into a chunk moves at most this many pointers, well under a microsecond, while 100k entries span 200 chunks to search.
 */
const CHUNK_ENTRIES = 512;

/**
 * One partition of a limited node in its order, holding the first rows up to its limit.
 *
 * A bounded window keeps only the held rows and knows every candidate up to its last one; it knows none follow only while exhaustive.
 * An arrangement keeps every candidate's sort values in sorted chunks, so the rows following the held ones are always known.
 */
export class Window {
    /** How the rows sort, completely, since the order ends with the key. */
    readonly #compare: (left: Row, right: Row) => number;
    /** The most rows the window holds. */
    readonly #limit: number;
    /** The known rows in order, in chunks: the held ones first, then in an arrangement every other candidate. */
    readonly #chunks: Entry[][] = [];
    /** The row each known key sorts as, which locates its entry. */
    readonly #rows = new Map<string, Row>();
    /** The join value of the partition, absent for a root's one partition. */
    readonly value: unknown;
    /** Whether the window keeps every candidate, holding the first up to its limit. */
    readonly isArranged: boolean;
    /** Whether no candidate follows the last known row, so that one entering after it belongs in a window with room. */
    #isExhaustive: boolean;

    /**
     * Order the rows of a partition, holding the first up to a limit.
     *
     * A bounded window takes the first rows of the order, at most the limit; an arrangement takes every candidate's sort values.
     */
    constructor(
        compare: (left: Row, right: Row) => number,
        limit: number,
        value: unknown,
        entries: readonly Entry[],
        isArranged: boolean,
    ) {
        // know every candidate in an arrangement, or none beyond fewer rows than the limit
        this.#compare = compare;
        this.#limit = limit;
        this.value = value;
        this.isArranged = isArranged;
        this.#isExhaustive = isArranged || entries.length < limit;

        // chunk the entries in their order
        const sorted = [...entries].sort((left, right) => compare(left.row, right.row));
        for (let start = 0; start < sorted.length; start += CHUNK_ENTRIES) {
            this.#chunks.push(sorted.slice(start, start + CHUNK_ENTRIES));
        }
        for (const entry of sorted) {
            this.#rows.set(entry.key, entry.row);
        }
    }

    /** Whether the window holds a row, among the first of its order up to its limit. */
    holds(key: string): boolean {
        const row = this.#rows.get(key);

        return row !== undefined && this.#isHeldAt(...this.#locate(row));
    }

    /** The keys of the held rows, in order. */
    held(): string[] {
        // walk the chunks up to the limit
        const keys: string[] = [];
        for (const chunk of this.#chunks) {
            for (const entry of chunk) {
                if (keys.length === this.#limit) {
                    return keys;
                }
                keys.push(entry.key);
            }
        }

        return keys;
    }

    /** The keys of the rows the window knows. */
    keys(): IterableIterator<string> {
        return this.#rows.keys();
    }

    /** The rows the window knows, which count against the feed's capacity. */
    get size(): number {
        return this.#rows.size;
    }

    /** Whether the window holds its limit of rows. */
    get isFull(): boolean {
        return this.#rows.size >= this.#limit;
    }

    /** Whether no candidate follows the last known row. */
    get isExhaustive(): boolean {
        return this.#isExhaustive;
    }

    /** The last known row, which rows must precede to enter a full bounded window. */
    get last(): Row | undefined {
        return this.#chunks.at(-1)?.at(-1)?.row;
    }

    /**
     * Place a row where the order puts it, returning whether the window holds it and the key it pushed out of the held rows.
     *
     * A bounded window forgets a pushed out row and leaves a row beyond its last to a refill; an arrangement keeps both.
     */
    place(key: string, row: Row): { readonly isHeld: boolean; readonly evicted?: string } {
        // take the row out, and leave a row beyond a bounded window's last one to a refill unless nothing follows and the window has room
        const boundary = this.last;
        const wasHeld = this.remove(key);
        const isBeyond =
            boundary === undefined ? !this.#isExhaustive : this.#compare(row, boundary) > 0;
        if (!this.isArranged && isBeyond && (!this.#isExhaustive || this.isFull)) {
            this.#isExhaustive &&= !this.isFull;

            return { isHeld: false };
        }

        // insert the row where the order puts it
        const [chunk, index] = this.#locate(row);
        const isHeld = this.#isHeldAt(chunk, index);
        this.#insert(chunk, index, { key, row });

        // push the row after the held ones out of them when the row entered, which a bounded window forgets
        if (!isHeld || wasHeld || this.#rows.size <= this.#limit) {
            return { isHeld };
        }
        const evicted = this.#at(this.#limit)!;
        if (!this.isArranged) {
            this.remove(evicted.key);
            this.#isExhaustive = false;
        }

        return { isHeld, evicted: evicted.key };
    }

    /** Append the rows a refill of a bounded window read after its last row, knowing none follow once it read fewer than it asked for. */
    extend(entries: readonly Entry[], requested: number): void {
        for (const entry of entries) {
            if (!this.#rows.has(entry.key)) {
                const [chunk, index] = this.#locate(entry.row);
                this.#insert(chunk, index, entry);
            }
        }
        this.#isExhaustive = entries.length < requested;
    }

    /** Take a row out, returning whether the window held it, so that the row following the held ones takes its place. */
    remove(key: string): boolean {
        // find the row by the values it sorts as
        const row = this.#rows.get(key);
        if (row === undefined) {
            return false;
        }
        const [chunk, index] = this.#locate(row);
        const wasHeld = this.#isHeldAt(chunk, index);

        // take it out in place, dropping an emptied chunk
        const entries = this.#chunks[chunk]!;
        entries.splice(index, 1);
        if (entries.length === 0) {
            this.#chunks.splice(chunk, 1);
        }
        this.#rows.delete(key);

        return wasHeld;
    }

    /** Find the chunk and index where a row sorts: its own entry when known, since the order is complete. */
    #locate(row: Row): [number, number] {
        // find the first chunk whose last entry does not sort before the row, or the last chunk
        let low = 0;
        let high = this.#chunks.length - 1;
        while (low < high) {
            const middle = (low + high) >>> 1;
            if (this.#compare(this.#chunks[middle]!.at(-1)!.row, row) < 0) {
                low = middle + 1;
            } else {
                high = middle;
            }
        }

        // find the row's index within it
        const entries = this.#chunks[low] ?? [];
        let start = 0;
        let end = entries.length;
        while (start < end) {
            const middle = (start + end) >>> 1;
            if (this.#compare(entries[middle]!.row, row) < 0) {
                start = middle + 1;
            } else {
                end = middle;
            }
        }

        return [low, start];
    }

    /** Insert an entry at a chunk's index, splitting a chunk that outgrew its size. */
    #insert(chunk: number, index: number, entry: Entry): void {
        // start the first chunk, or insert into the found one
        this.#rows.set(entry.key, entry.row);
        const entries = this.#chunks[chunk];
        if (entries === undefined) {
            this.#chunks.push([entry]);

            return;
        }
        entries.splice(index, 0, entry);

        // split a full chunk in halves
        if (entries.length > CHUNK_ENTRIES) {
            this.#chunks.splice(chunk + 1, 0, entries.splice(entries.length >>> 1));
        }
    }

    /** Whether the entry at a chunk's index sorts among the held rows, counting the chunks before it up to the limit. */
    #isHeldAt(chunk: number, index: number): boolean {
        let position = index;
        for (let before = 0; before < chunk && position < this.#limit; before += 1) {
            position += this.#chunks[before]!.length;
        }

        return position < this.#limit;
    }

    /** Read the entry at a position in the order. */
    #at(position: number): Entry | undefined {
        let rest = position;
        for (const chunk of this.#chunks) {
            if (rest < chunk.length) {
                return chunk[rest];
            }
            rest -= chunk.length;
        }

        return undefined;
    }
}

/** A row a window orders: its key, and the row or, in an arrangement, the values it sorts by. */
interface Entry {
    /** The row's key. */
    readonly key: string;
    /** The row with its computed values, or the values it sorts by in an arrangement. */
    readonly row: Row;
}
