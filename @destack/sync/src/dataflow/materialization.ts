import type { Row } from "@destack/db";
import { aligned } from "@destack/schema";
import type { Item, AggregateRow } from "../query/query.ts";
import type { Member, Partition } from "./pipeline.ts";

/** One built result row: the selected row it came from, its concealed columns, and the entry readers see. */
interface ResultRow {
    /** The selected row the entry was built from. */
    readonly source: Row;
    /** The concealed columns, joined. */
    readonly hidden: string;
    /** The entry, with its includes nested. */
    readonly entry: Item;
}

/** One partition's last order: its keys, and the selected rows it ordered them by. */
interface PartitionOrder {
    /** The selected keys in query order. */
    keys: readonly string[];
    /** The selected row each key was ordered by. */
    readonly sources: Map<string, Row>;
}

/**
 * The entries a selection's last read built, kept so a read rebuilds only what changed.
 *
 * Unchanged rows keep their entry and unchanged partitions their list, so readers compare results by identity.
 */
export class Materialization {
    /** The built entries, by row key. */
    readonly #built = new Map<string, ResultRow>();
    /** The last order of each partition, by name. */
    readonly #ordered = new Map<string, PartitionOrder>();
    /** The last list of each partition's entries, by name. */
    readonly #listed = new Map<string, readonly Item[]>();

    /** Order a partition's selected keys, placing only rows that changed since the last read. */
    order(
        name: string,
        keys: readonly string[],
        rowOf: (key: string) => Row,
        compare: (left: Row, right: Row) => number,
    ): readonly string[] {
        // sort every key on a partition's first read
        const ordered = this.#ordered.get(name);
        if (ordered === undefined) {
            const sorted = keys.toSorted((left, right) => compare(rowOf(left), rowOf(right)));
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

    /** Read the groups a row's last entry had for an aggregate include, so equal new groups keep their identity. */
    previous(key: string, name: string): readonly AggregateRow[] | undefined {
        return this.#built.get(key)?.entry.measures[name];
    }

    /** Take a row's entry again when its row, concealment and nested results are unchanged, else build it. */
    entry(
        key: string,
        source: Row,
        hidden: readonly string[],
        included: readonly (readonly [string, readonly Item[]])[],
        measured: readonly (readonly [string, readonly AggregateRow[]])[],
    ): Item {
        // take the last entry when nothing it was built from changed
        const joined = hidden.join(",");
        const built = this.#built.get(key);
        const isUnchanged =
            built !== undefined &&
            built.source === source &&
            built.hidden === joined &&
            included.every(([name, items]) => built.entry.with[name] === items) &&
            measured.every(([name, groups]) => built.entry.measures[name] === groups);
        if (isUnchanged) {
            return built.entry;
        }

        // build the entry, keeping the last one when its values are equal
        const entry: Item = {
            row: Object.fromEntries(
                Object.entries(source).filter(([column]) => !hidden.includes(column)),
            ),
            with: Object.fromEntries(included),
            measures: Object.fromEntries(measured),
        };
        const kept =
            built !== undefined && Materialization.isSame(built.entry, entry) ? built.entry : entry;
        this.#built.set(key, { source, hidden: joined, entry: kept });

        return kept;
    }

    /** Keep a partition's entries, taking the earlier list again when it has the same entries in the same order. */
    list(name: string, entries: readonly Item[]): readonly Item[] {
        // keep the earlier list when it has the same entries in the same order
        const listed = this.#listed.get(name);
        const isSame =
            listed !== undefined &&
            listed.length === entries.length &&
            entries.every((entry, index) => entry === listed[index]);
        const kept = isSame ? listed : entries;
        this.#listed.set(name, kept);

        return kept;
    }

    /** Forget the rows and partitions the selection no longer has, once fewer are selected than were kept. */
    prune(members: ReadonlyMap<string, Member>, partitions: ReadonlyMap<string, Partition>): void {
        // forget the entries of rows no partition has
        if (this.#built.size > members.size) {
            for (const key of this.#built.keys()) {
                if (!members.has(key)) {
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
        } else if (!isObject(left) || !isObject(right)) {
            return false;
        }
        const leftKeys = Object.keys(left);
        const rightKeys = Object.keys(right);

        return (
            leftKeys.length === rightKeys.length &&
            leftKeys.every((key) => Materialization.isSame(left[key], right[key]))
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
            if (compare(rowOf(aligned(order, middle)), row) <= 0) {
                low = middle + 1;
            } else {
                high = middle;
            }
        }

        return low;
    }
}

/** Report whether a value is an object or array whose fields compare one by one. */
function isObject(value: unknown): value is Readonly<Record<string, unknown>> {
    return typeof value === "object" && value !== null;
}
