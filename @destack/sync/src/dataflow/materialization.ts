import type { Row } from "@destack/db";

/** One built result row: the held row it came from, its concealed columns, and the entry readers see. */
interface Built {
    /** The held row the entry was built from. */
    readonly source: Row;
    /** The concealed columns, joined. */
    readonly hidden: string;
    /** The entry, with its includes nested. */
    readonly entry: Readonly<Record<string, unknown>>;
}

/** One partition's last order: its keys, and the held rows it ordered them by. */
interface Ordered {
    /** The held keys in query order. */
    keys: readonly string[];
    /** The held row each key was ordered by. */
    readonly sources: Map<string, Row>;
}

/**
 * The entries a selection's last read built, kept so a read rebuilds only what changed.
 *
 * Unchanged rows keep their entry and unchanged partitions their list, so readers compare results by identity.
 */
export class Materialization {
    /** The built entries, by row key. */
    readonly #built = new Map<string, Built>();
    /** The last order of each partition, by name. */
    readonly #ordered = new Map<string, Ordered>();
    /** The last list of each partition's entries, by name. */
    readonly #listed = new Map<string, readonly Readonly<Record<string, unknown>>[]>();

    /** Order a partition's held keys, placing only rows that changed since the last read. */
    order(
        name: string,
        keys: readonly string[],
        rowOf: (key: string) => Row,
        compare: (left: Row, right: Row) => number,
    ): readonly string[] {
        // sort every key on a partition's first read
        const ordered = this.#ordered.get(name);
        if (ordered === undefined) {
            const sorted = [...keys].sort((left, right) => compare(rowOf(left), rowOf(right)));
            this.#ordered.set(name, {
                keys: sorted,
                sources: new Map(sorted.map((key) => [key, rowOf(key)])),
            });

            return sorted;
        }

        // keep the unchanged keys in their order, then place each row that changed or arrived since
        const { sources } = ordered;
        const present = new Set(keys);
        const changed = keys.filter((key) => sources.get(key) !== rowOf(key));
        const moved = new Set(changed);
        const order = ordered.keys.filter((key) => present.has(key) && !moved.has(key));
        for (const key of changed) {
            order.splice(Materialization.#position(order, rowOf(key), rowOf, compare), 0, key);
            sources.set(key, rowOf(key));
        }

        // forget the rows that left
        if (order.length < sources.size) {
            for (const key of sources.keys()) {
                if (!present.has(key)) {
                    sources.delete(key);
                }
            }
        }
        ordered.keys = order;

        return order;
    }

    /** Read the value a row's last entry held for an include, so an equal new value keeps its identity. */
    previous(key: string, property: string): unknown {
        return this.#built.get(key)?.entry[property];
    }

    /** Take a row's entry again when its row, concealment and nested values are unchanged, else build it. */
    entry(
        key: string,
        source: Row,
        hidden: readonly string[],
        nested: readonly (readonly [string, unknown])[],
    ): Readonly<Record<string, unknown>> {
        // take the last entry when nothing it was built from changed
        const joined = hidden.join(",");
        const built = this.#built.get(key);
        const isUnchanged =
            built !== undefined &&
            built.source === source &&
            built.hidden === joined &&
            nested.every(([property, value]) => built.entry[property] === value);
        if (isUnchanged) {
            return built.entry;
        }

        // build the entry, keeping the last one when its values are equal
        const entry: Record<string, unknown> = {};
        for (const [column, value] of Object.entries(source)) {
            if (!hidden.includes(column)) {
                entry[column] = value;
            }
        }
        for (const [property, value] of nested) {
            entry[property] = value;
        }
        const kept =
            built !== undefined && Materialization.isSame(built.entry, entry) ? built.entry : entry;
        this.#built.set(key, { source, hidden: joined, entry: kept });

        return kept;
    }

    /** Keep a partition's entries, taking the earlier list again when it holds the same entries in the same order. */
    list(
        name: string,
        entries: readonly Readonly<Record<string, unknown>>[],
    ): readonly Readonly<Record<string, unknown>>[] {
        // keep the earlier list when it holds the same entries in the same order
        const listed = this.#listed.get(name);
        const isSame =
            listed !== undefined &&
            listed.length === entries.length &&
            entries.every((entry, index) => entry === listed[index]);
        const kept = isSame ? listed : entries;
        this.#listed.set(name, kept);

        return kept;
    }

    /** Forget the rows and partitions the selection no longer holds, once fewer are held than were kept. */
    prune(held: ReadonlyMap<string, unknown>, partitions: ReadonlyMap<string, unknown>): void {
        // forget the entries of rows no partition holds
        if (this.#built.size > held.size) {
            for (const key of this.#built.keys()) {
                if (!held.has(key)) {
                    this.#built.delete(key);
                }
            }
        }

        // forget the closed partitions
        if (this.#ordered.size > partitions.size) {
            for (const name of this.#ordered.keys()) {
                if (!partitions.has(name)) {
                    this.#ordered.delete(name);
                    this.#listed.delete(name);
                }
            }
        }
    }

    /** Report whether two values are the same: the same object, or equal JSON values field by field. */
    static isSame(left: unknown, right: unknown): boolean {
        // take identical values, and compare objects by their fields
        if (Object.is(left, right)) {
            return true;
        } else if (
            typeof left !== "object" ||
            typeof right !== "object" ||
            left === null ||
            right === null
        ) {
            return false;
        }
        const leftKeys = Object.keys(left);
        const rightKeys = Object.keys(right);

        return (
            leftKeys.length === rightKeys.length &&
            leftKeys.every((key) =>
                Materialization.isSame(
                    (left as Record<string, unknown>)[key],
                    (right as Record<string, unknown>)[key],
                ),
            )
        );
    }

    /** Find where a row belongs among ordered keys, after every equal row. */
    static #position(
        order: readonly string[],
        row: Row,
        rowOf: (key: string) => Row,
        compare: (left: Row, right: Row) => number,
    ): number {
        // search the first key ordered after the row
        let low = 0;
        let high = order.length;
        while (low < high) {
            const middle = (low + high) >>> 1;
            if (compare(rowOf(order[middle]!), row) <= 0) {
                low = middle + 1;
            } else {
                high = middle;
            }
        }

        return low;
    }
}
