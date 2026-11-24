import { nth } from "@destack/schema";

/** The gap between items of a layout that sets none, in pixels. */
const GAP = 8;

/** The smallest share of its height a row of photos shrinks to, below which it breaks before. */
const SHRINK = 0.5;

/** The columns of a collage: two for its first item, and two for the others. */
export const COLLAGE_COLUMNS = 4;

/** Where an item is, in pixels from the start of the first row. */
export interface Box {
    /** The distance along the row. */
    readonly x: number;
    /** The distance across the rows. */
    readonly y: number;
    /** The width. */
    readonly width: number;
    /** The height. */
    readonly height: number;
}

/** How the browser itself lays items out, before anything is measured. */
export type Flow =
    | { readonly kind: "grid"; readonly minimum: number; readonly ratio: number }
    | { readonly kind: "rows"; readonly height: number }
    | { readonly kind: "collage"; readonly count: number; readonly ratio: number };

/** A way to lay items out, which places each by its ratio of width to height. */
export interface Layout {
    /** The gap between items, in pixels. */
    readonly gap: number;
    /** How the browser lays the items out, absent for a layout it has none for. */
    readonly flow?: Flow;
    /** Place each item in a width, in pixels. */
    readonly place: (ratios: readonly number[], width: number) => readonly Box[];
}

/** The columns of a layout: as many of a least width as fit. */
interface Columns {
    /** How many there are. */
    readonly count: number;
    /** The width of each. */
    readonly width: number;
}

/** Equal cells in rows, such as files or people. */
export class GridLayout implements Layout {
    /** The least width of a cell. */
    readonly #minimum: number;
    /** The ratio of a cell's width to its height. */
    readonly #ratio: number;
    /** The gap between cells. */
    readonly gap: number;
    /** How the browser lays the cells out. */
    readonly flow: Flow;

    /** Lay cells out as wide as at least a width, square by default. */
    constructor(options: {
        readonly minimum: number;
        readonly ratio?: number;
        readonly gap?: number;
    }) {
        // keep its measures, and tell the browser its flow
        this.#minimum = options.minimum;
        this.#ratio = options.ratio ?? 1;
        this.gap = options.gap ?? GAP;
        this.flow = { kind: "grid", minimum: this.#minimum, ratio: this.#ratio };
    }

    /** Place each item in a width, in pixels. */
    place(ratios: readonly number[], width: number): readonly Box[] {
        const columns = columnsOf(width, this.#minimum, this.gap);
        const height = columns.width / this.#ratio;

        return ratios.map((_ratio, i) => ({
            x: (i % columns.count) * (columns.width + this.gap),
            y: Math.floor(i / columns.count) * (height + this.gap),
            width: columns.width,
            height,
        }));
    }
}

/** Rows of one height each that fill the width, their breaks chosen together, such as photos. */
export class RowLayout implements Layout {
    /** The height rows come near to. */
    readonly #height: number;
    /** The gap between items. */
    readonly gap: number;
    /** How the browser lays the items out. */
    readonly flow: Flow;

    /** Lay rows out near a height. */
    constructor(options: { readonly height: number; readonly gap?: number }) {
        this.#height = options.height;
        this.gap = options.gap ?? GAP;
        this.flow = { kind: "rows", height: this.#height };
    }

    /** Place each item in a width, in pixels. */
    place(ratios: readonly number[], width: number): readonly Box[] {
        // place the items of each row side by side, the last row no taller than the others
        const boxes: Box[] = [];
        let y = 0;
        for (const [start, end] of this.#rows(ratios, width)) {
            const full = this.#heightOf(ratios.slice(start, end), width);
            const height = end === ratios.length ? Math.min(full, this.#height) : full;
            let x = 0;
            for (const ratio of ratios.slice(start, end)) {
                boxes.push({ x, y, width: ratio * height, height });
                x += ratio * height + this.gap;
            }
            y += height + this.gap;
        }

        return boxes;
    }

    /** Choose the rows whose heights together come nearest to the one asked for. */
    #rows(ratios: readonly number[], width: number): readonly (readonly [number, number])[] {
        // find the least cost of the items before each, and where the row before it starts
        const costs = [0];
        const starts = [0];
        for (let end = 1; end <= ratios.length; end += 1) {
            let sum = 0;
            let least = Number.POSITIVE_INFINITY;
            let best = end - 1;
            for (let start = end - 1; start >= 0; start -= 1) {
                // end once a row gets too low, as longer ones only get lower
                sum += nth(ratios, start);
                const height = this.#fit(sum, end - start, width);
                if (height < this.#height * SHRINK && start < end - 1) {
                    break;
                }

                // leave a last row that does not fill the width as it is
                const isShort = end === ratios.length && height > this.#height;
                const away = isShort ? 0 : height / this.#height - 1;
                const cost = nth(costs, start) + away * away;
                if (cost < least) {
                    least = cost;
                    best = start;
                }
            }
            costs.push(least);
            starts.push(best);
        }

        // read the rows back from the last
        const rows: (readonly [number, number])[] = [];
        for (let end = ratios.length; end > 0; end = nth(starts, end)) {
            rows.push([nth(starts, end), end]);
        }

        return rows.toReversed();
    }

    /** Compute the height at which items fill a width. */
    #heightOf(ratios: readonly number[], width: number): number {
        const sum = ratios.reduce((total, ratio) => total + ratio, 0);

        return this.#fit(sum, ratios.length, width);
    }

    /** Compute the height at which a count of items of a sum of ratios fills a width. */
    #fit(sum: number, count: number, width: number): number {
        return Math.max(width - (count - 1) * this.gap, 1) / sum;
    }
}

/** Columns of one width, each item under the shortest, such as pins on a board. */
export class MasonryLayout implements Layout {
    /** The least width of a column. */
    readonly #minimum: number;
    /** The gap between items. */
    readonly gap: number;

    /** Lay columns out as wide as at least a width. */
    constructor(options: { readonly minimum: number; readonly gap?: number }) {
        this.#minimum = options.minimum;
        this.gap = options.gap ?? GAP;
    }

    /** Place each item in a width, in pixels. */
    place(ratios: readonly number[], width: number): readonly Box[] {
        const columns = columnsOf(width, this.#minimum, this.gap);
        const ends = Array.from({ length: columns.count }, () => 0);

        return ratios.map((ratio) => {
            // place it under the first shortest column
            const y = Math.min(...ends);
            const column = ends.indexOf(y);
            const height = columns.width / ratio;
            ends[column] = y + height + this.gap;

            return { x: column * (columns.width + this.gap), y, width: columns.width, height };
        });
    }
}

/** A few items in one frame, the first the largest, such as the photos of a message. */
export class CollageLayout implements Layout {
    /** The most items it places. */
    readonly #count: number;
    /** The ratio of the frame. */
    readonly #ratio: number;
    /** The gap between items. */
    readonly gap: number;
    /** How the browser lays the items out. */
    readonly flow: Flow;

    /** Lay out up to a count of items in a frame, 3:2 by default. */
    constructor(options: {
        readonly count: number;
        readonly ratio?: number;
        readonly gap?: number;
    }) {
        // keep its measures, and tell the browser its flow
        this.#count = options.count;
        this.#ratio = options.ratio ?? 1.5;
        this.gap = options.gap ?? GAP;
        this.flow = { kind: "collage", count: this.#count, ratio: this.#ratio };
    }

    /** Count the rows of a collage of a count of items. */
    static rows(count: number): number {
        const others = count - 1;

        return Math.max(1, others > 2 ? Math.ceil(others / 2) : others);
    }

    /** Place an item of a count in columns and rows of one each. */
    static area(index: number, count: number): Box {
        // give the frame to one item, and half to the first of several
        const rows = CollageLayout.rows(count);
        const half = COLLAGE_COLUMNS / 2;
        if (index === 0) {
            return { x: 0, y: 0, width: count === 1 ? COLLAGE_COLUMNS : half, height: rows };
        }

        // fill the other half row by row, a last one alone as wide as its row
        const at = index - 1;
        const across = count > 3 ? half : 1;
        const isAlone = index === count - 1 && at % across === 0;

        return {
            x: half + (at % across),
            y: Math.floor(at / across),
            width: isAlone || across === 1 ? half : 1,
            height: 1,
        };
    }

    /** Place the items it has room for in a width, in pixels. */
    place(ratios: readonly number[], width: number): readonly Box[] {
        // stretch each item's columns and rows, and the gaps between them
        const count = Math.min(ratios.length, this.#count);
        const rows = CollageLayout.rows(count);
        const height = width / this.#ratio;
        const column = (width - (COLLAGE_COLUMNS - 1) * this.gap) / COLLAGE_COLUMNS;
        const row = (height - (rows - 1) * this.gap) / rows;

        return Array.from({ length: count }, (_item, i) => {
            const area = CollageLayout.area(i, count);

            return {
                x: area.x * (column + this.gap),
                y: area.y * (row + this.gap),
                width: area.width * (column + this.gap) - this.gap,
                height: area.height * (row + this.gap) - this.gap,
            };
        });
    }
}

/** Fit as many columns of a least width as there is room for, one at least. */
function columnsOf(width: number, minimum: number, gap: number): Columns {
    const count = Math.max(1, Math.floor((width + gap) / (minimum + gap)));

    return { count, width: (width - (count - 1) * gap) / count };
}
