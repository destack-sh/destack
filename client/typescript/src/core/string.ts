import type { StringId, StringPoolData } from "../_generated/core/string.js";

/** Pool for resolving stable string ids to strings. */
export class StringPool {
    readonly #textById: ReadonlyMap<StringId, string>;
    readonly #idByText: ReadonlyMap<string, StringId>;

    /** Create one string pool from exact entries. */
    constructor(strings: Iterable<readonly [StringId, string]>) {
        const textById = new Map<StringId, string>();
        const idByText = new Map<string, StringId>();

        // validate all entries while building both lookup directions
        for (const [id, text] of strings) {
            const existingText = textById.get(id);
            if (existingText !== undefined && existingText !== text) {
                throw new Error(`string id collision: ${id}`);
            }

            const existingId = idByText.get(text);
            if (existingId !== undefined && existingId !== id) {
                throw new Error(`duplicate string text with different ids: ${text}`);
            }

            textById.set(id, text);
            idByText.set(text, id);
        }

        this.#textById = textById;
        this.#idByText = idByText;
    }

    /** Create one string pool from serialized string pool data. */
    static fromData(data: StringPoolData): StringPool {
        return new StringPool(data.strings);
    }

    /** Return whether one string id is present. */
    has(id: StringId): boolean {
        return this.#textById.has(id);
    }

    /** Return the string for one string id. */
    get(id: StringId): string {
        const text = this.#textById.get(id);
        if (text === undefined) {
            throw new Error(`unknown string id: ${id}`);
        }

        return text;
    }

    /** Return the string for one string id when present. */
    getMaybe(id: StringId | undefined): string | undefined {
        return id === undefined ? undefined : this.#textById.get(id);
    }

    /** Return the stable string id for one already interned string. */
    intern(text: string): StringId {
        const id = this.#idByText.get(text);
        if (id === undefined) {
            throw new Error(`unknown string text: ${text}`);
        }

        return id;
    }

    /** Return all strings for a sequence of string ids. */
    values(ids: Iterable<StringId>): string[] {
        const values = [];

        // resolve ids in caller-provided order
        for (const id of ids) {
            values.push(this.get(id));
        }

        return values;
    }

    /** Join all strings for a sequence of string ids. */
    join(ids: Iterable<StringId>, separator = ""): string {
        return this.values(ids).join(separator);
    }

    /** Return this pool as serialized data. */
    data(): StringPoolData {
        const strings = Array.from(this.#textById.entries());

        // sort by stable string id for reproducible serialization
        strings.sort(([left], [right]) => {
            if (left < right) {
                return -1;
            } else if (left > right) {
                return 1;
            } else {
                return 0;
            }
        });

        return { strings };
    }
}
