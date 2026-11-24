import type { LineRange } from "../source/index.ts";

/** A place past every other, the largest a list of 32-bit numbers stores. */
const FARTHEST = 2_147_483_647;

/** The least steps a search takes before it settles for a good split. */
const LEAST_COST = 256;

/** Lines of one source that stand in place of lines of another. */
export interface Change {
    /** The lines replaced, or where none are. */
    readonly before: LineRange;
    /** The lines in their place, or where none are. */
    readonly after: LineRange;
}

/** A part of both lists still to compare. */
interface Part {
    /** The first place of the old list. */
    readonly beforeFrom: number;
    /** The place after the last of the old list. */
    readonly beforeTo: number;
    /** The first place of the new list. */
    readonly afterFrom: number;
    /** The place after the last of the new list. */
    readonly afterTo: number;
}

/** Compare two lists of numbers, split at those both have once, then where both ends meet. */
export function compare(before: Int32Array, after: Int32Array): Change[] {
    // leave items one list has alone out of the search, as they changed for sure
    const comparison = new Comparison(before, after);
    comparison.run();

    return comparison.changes();
}

/** The comparison of two lists, marking what is in one of them alone. */
class Comparison {
    /** The old list. */
    readonly #before: Int32Array;
    /** The new list. */
    readonly #after: Int32Array;
    /** Whether each item of the old list is removed. */
    readonly #removed: Uint8Array;
    /** Whether each item of the new list is added. */
    readonly #added: Uint8Array;

    /** Compare two lists. */
    constructor(before: Int32Array, after: Int32Array) {
        // start without a mark
        this.#before = before;
        this.#after = after;
        this.#removed = new Uint8Array(before.length);
        this.#added = new Uint8Array(after.length);
    }

    /** Mark every removed and added item. */
    run(): void {
        // count each number in both lists
        const largest = Math.max(largestOf(this.#before), largestOf(this.#after));
        const olds = countsOf(this.#before, largest);
        const news = countsOf(this.#after, largest);

        // mark the items one list has alone, and search the rest
        const keptBefore = this.#keep(this.#before, news, this.#removed);
        const keptAfter = this.#keep(this.#after, olds, this.#added);
        const search = new Search(
            keptBefore.map((place) => this.#before[place] ?? 0),
            keptAfter.map((place) => this.#after[place] ?? 0),
            largest,
            (place) => {
                this.#removed[keptBefore[place] ?? 0] = 1;
            },
            (place) => {
                this.#added[keptAfter[place] ?? 0] = 1;
            },
        );
        search.run();
    }

    /** Read the changes off the marks. */
    changes(): Change[] {
        // walk both lists together
        const changes: Change[] = [];
        let before = 0;
        let after = 0;
        while (before < this.#before.length || after < this.#after.length) {
            // pass what both have
            if (this.#removed[before] !== 1 && this.#added[after] !== 1) {
                before += 1;
                after += 1;
                continue;
            }

            // take all that is removed and added here
            const from = { before, after };
            while (this.#removed[before] === 1) {
                before += 1;
            }
            while (this.#added[after] === 1) {
                after += 1;
            }
            changes.push({
                before: { start: from.before, end: before },
                after: { start: from.after, end: after },
            });
        }

        return changes;
    }

    /** Mark the items of a list the other lacks, returning the places of the rest. */
    #keep(list: Int32Array, others: Int32Array, marks: Uint8Array): Int32Array {
        // pass each item to the marks or to the places
        const places = new Int32Array(list.length);
        let count = 0;
        for (let place = 0; place < list.length; place += 1) {
            if (others[list[place] ?? 0] === 0) {
                marks[place] = 1;
            } else {
                places[count] = place;
                count += 1;
            }
        }

        return places.subarray(0, count);
    }
}

/** A search for what two lists have in common. */
class Search {
    /** The old list. */
    readonly #before: Int32Array;
    /** The new list. */
    readonly #after: Int32Array;
    /** Mark an item of the old list as removed. */
    readonly #remove: (place: number) => void;
    /** Mark an item of the new list as added. */
    readonly #add: (place: number) => void;
    /** How often each number is in the old list's part, zero between parts. */
    readonly #olds: Int32Array;
    /** How often each number is in the new list's part, zero between parts. */
    readonly #news: Int32Array;
    /** The place of each number in the new list's part, read where it is there once. */
    readonly #places: Int32Array;
    /** How far each diagonal comes from the start. */
    readonly #forward: Int32Array;
    /** How far each diagonal comes from the end. */
    readonly #backward: Int32Array;

    /** Search two lists of numbers up to a largest one. */
    constructor(
        before: Int32Array,
        after: Int32Array,
        largest: number,
        remove: (place: number) => void,
        add: (place: number) => void,
    ) {
        // make room once for every part
        this.#before = before;
        this.#after = after;
        this.#remove = remove;
        this.#add = add;
        this.#olds = new Int32Array(largest + 1);
        this.#news = new Int32Array(largest + 1);
        this.#places = new Int32Array(largest + 1);
        this.#forward = new Int32Array(before.length + after.length + 3);
        this.#backward = new Int32Array(before.length + after.length + 3);
    }

    /** Mark what is not in common, part after part. */
    run(): void {
        // start with everything, and take the parts each split leaves
        const parts: Part[] = [
            {
                beforeFrom: 0,
                beforeTo: this.#before.length,
                afterFrom: 0,
                afterTo: this.#after.length,
            },
        ];
        for (let part = parts.pop(); part !== undefined; part = parts.pop()) {
            const rest = this.#trim(part);
            if (rest.beforeFrom === rest.beforeTo || rest.afterFrom === rest.afterTo) {
                this.#mark(rest);
            } else {
                parts.push(...(this.#anchored(rest) ?? this.#halved(rest)));
            }
        }
    }

    /** Leave out what a part starts and ends with in both lists. */
    #trim(part: Part): Part {
        // step over equal items from the start
        let { beforeFrom, beforeTo, afterFrom, afterTo } = part;
        while (
            beforeFrom < beforeTo &&
            afterFrom < afterTo &&
            this.#before[beforeFrom] === this.#after[afterFrom]
        ) {
            beforeFrom += 1;
            afterFrom += 1;
        }

        // then from the end
        while (
            beforeFrom < beforeTo &&
            afterFrom < afterTo &&
            this.#before[beforeTo - 1] === this.#after[afterTo - 1]
        ) {
            beforeTo -= 1;
            afterTo -= 1;
        }

        return { beforeFrom, beforeTo, afterFrom, afterTo };
    }

    /** Mark a part as removed and added. */
    #mark({ beforeFrom, beforeTo, afterFrom, afterTo }: Part): void {
        for (let place = beforeFrom; place < beforeTo; place += 1) {
            this.#remove(place);
        }
        for (let place = afterFrom; place < afterTo; place += 1) {
            this.#add(place);
        }
    }

    /** Split a part at the items each list has once, absent where it has none in order. */
    #anchored(part: Part): Part[] | undefined {
        // count the part's numbers
        const { beforeFrom, beforeTo, afterFrom, afterTo } = part;
        for (let place = beforeFrom; place < beforeTo; place += 1) {
            const item = this.#before[place] ?? 0;
            this.#olds[item] = (this.#olds[item] ?? 0) + 1;
        }
        for (let place = afterFrom; place < afterTo; place += 1) {
            const item = this.#after[place] ?? 0;
            this.#news[item] = (this.#news[item] ?? 0) + 1;
            this.#places[item] = place;
        }

        // list those each list has once
        const anchors: { readonly before: number; readonly after: number }[] = [];
        for (let place = beforeFrom; place < beforeTo; place += 1) {
            const item = this.#before[place] ?? 0;
            if (this.#olds[item] === 1 && this.#news[item] === 1) {
                anchors.push({ before: place, after: this.#places[item] ?? 0 });
            }
        }

        // forget the counts
        for (let place = beforeFrom; place < beforeTo; place += 1) {
            this.#olds[this.#before[place] ?? 0] = 0;
        }
        for (let place = afterFrom; place < afterTo; place += 1) {
            this.#news[this.#after[place] ?? 0] = 0;
        }
        if (anchors.length === 0) {
            return undefined;
        }

        // split at the longest run of them in order
        const parts: Part[] = [];
        let last = { before: beforeFrom, after: afterFrom };
        for (const { before, after } of risingOf(anchors)) {
            parts.push({
                beforeFrom: last.before,
                beforeTo: before,
                afterFrom: last.after,
                afterTo: after,
            });
            last = { before: before + 1, after: after + 1 };
        }
        parts.push({ beforeFrom: last.before, beforeTo, afterFrom: last.after, afterTo });

        return parts;
    }

    /** Split a part where the fewest steps from its start meet the fewest from its end. */
    #halved(part: Part): Part[] {
        const { beforeFrom, beforeTo, afterFrom, afterTo } = part;
        const { before, after } = this.#middle(part);

        return [
            { beforeFrom, beforeTo: before, afterFrom, afterTo: after },
            { beforeFrom: before, beforeTo, afterFrom: after, afterTo },
        ];
    }

    /** Find where the searches from both ends of a part meet, or have come far enough. */
    #middle(part: Part): { readonly before: number; readonly after: number } {
        // number the diagonals by how far the old list is ahead of the new one
        const { beforeFrom, beforeTo, afterFrom, afterTo } = part;
        const [forward, backward] = [this.#forward, this.#backward];
        const zero = this.#after.length + 1;
        const [lowest, highest] = [beforeFrom - afterTo, beforeTo - afterFrom];
        const [start, end] = [beforeFrom - afterFrom, beforeTo - afterTo];
        const isOdd = ((start - end) & 1) !== 0;
        const most = Math.max(LEAST_COST, Math.ceil(Math.sqrt(highest - lowest)));
        let [startLow, startHigh, endLow, endHigh] = [start, start, end, end];
        forward[zero + start] = beforeFrom;
        backward[zero + end] = beforeTo;

        // step on from both ends by turns until they meet
        for (let cost = 1; ; cost += 1) {
            // step every diagonal on from the start
            if (startLow > lowest) {
                startLow -= 1;
                forward[zero + startLow - 1] = -1;
            } else {
                startLow += 1;
            }
            if (startHigh < highest) {
                startHigh += 1;
                forward[zero + startHigh + 1] = -1;
            } else {
                startHigh -= 1;
            }
            for (let diagonal = startHigh; diagonal >= startLow; diagonal -= 2) {
                // come from the neighbour that is further, then along what both have
                const below = forward[zero + diagonal - 1] ?? -1;
                const above = forward[zero + diagonal + 1] ?? -1;
                let before = below >= above ? below + 1 : above;
                while (
                    before < beforeTo &&
                    before - diagonal < afterTo &&
                    this.#before[before] === this.#after[before - diagonal]
                ) {
                    before += 1;
                }
                forward[zero + diagonal] = before;
                if (
                    isOdd &&
                    endLow <= diagonal &&
                    diagonal <= endHigh &&
                    (backward[zero + diagonal] ?? 0) <= before
                ) {
                    return { before, after: before - diagonal };
                }
            }

            // step every diagonal back from the end
            if (endLow > lowest) {
                endLow -= 1;
                backward[zero + endLow - 1] = FARTHEST;
            } else {
                endLow += 1;
            }
            if (endHigh < highest) {
                endHigh += 1;
                backward[zero + endHigh + 1] = FARTHEST;
            } else {
                endHigh -= 1;
            }
            for (let diagonal = endHigh; diagonal >= endLow; diagonal -= 2) {
                const below = backward[zero + diagonal - 1] ?? 0;
                const above = backward[zero + diagonal + 1] ?? 0;
                let before = below < above ? below : above - 1;
                while (
                    before > beforeFrom &&
                    before - diagonal > afterFrom &&
                    this.#before[before - 1] === this.#after[before - diagonal - 1]
                ) {
                    before -= 1;
                }
                backward[zero + diagonal] = before;
                if (
                    !isOdd &&
                    startLow <= diagonal &&
                    diagonal <= startHigh &&
                    before <= (forward[zero + diagonal] ?? 0)
                ) {
                    return { before, after: before - diagonal };
                }
            }

            // settle for the place furthest along, from whichever end came further
            if (cost >= most) {
                let best = { before: beforeFrom, after: afterFrom, reach: -1 };
                for (let diagonal = startHigh; diagonal >= startLow; diagonal -= 2) {
                    const before = Math.min(
                        forward[zero + diagonal] ?? 0,
                        beforeTo,
                        afterTo + diagonal,
                    );
                    const reach = before - beforeFrom + (before - diagonal - afterFrom);
                    if (reach > best.reach) {
                        best = { before, after: before - diagonal, reach };
                    }
                }
                for (let diagonal = endHigh; diagonal >= endLow; diagonal -= 2) {
                    const before = Math.max(
                        backward[zero + diagonal] ?? 0,
                        beforeFrom,
                        afterFrom + diagonal,
                    );
                    const reach = beforeTo - before + (afterTo - before + diagonal);
                    if (reach > best.reach) {
                        best = { before, after: before - diagonal, reach };
                    }
                }

                return best;
            }
        }
    }
}

/** Find the longest run of anchors whose new places rise, by patience sorting. */
function risingOf<Anchor extends { readonly after: number }>(
    anchors: readonly Anchor[],
): readonly Anchor[] {
    // put each anchor on the leftmost pile whose top is past it, noting the top of the pile before
    const tops: number[] = [];
    const before = new Int32Array(anchors.length);
    for (const [place, anchor] of anchors.entries()) {
        let low = 0;
        let high = tops.length;
        while (low < high) {
            const middle = (low + high) >>> 1;
            if ((anchors[tops[middle] ?? 0]?.after ?? 0) < anchor.after) {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        before[place] = low === 0 ? -1 : (tops[low - 1] ?? -1);
        tops[low] = place;
    }

    // walk back from the last pile's top
    const rising: Anchor[] = [];
    for (let place = tops.at(-1) ?? -1; place >= 0; place = before[place] ?? -1) {
        const anchor = anchors[place];
        if (anchor !== undefined) {
            rising.push(anchor);
        }
    }

    return rising.toReversed();
}

/** Find the largest number of a list, zero for an empty one. */
function largestOf(list: Int32Array): number {
    let largest = 0;
    for (const item of list) {
        largest = Math.max(largest, item);
    }

    return largest;
}

/** Count how often each number is in a list. */
function countsOf(list: Int32Array, largest: number): Int32Array {
    const counts = new Int32Array(largest + 1);
    for (const item of list) {
        counts[item] = (counts[item] ?? 0) + 1;
    }

    return counts;
}
