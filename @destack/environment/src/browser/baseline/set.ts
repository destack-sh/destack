import type { NativeVersions, Shim } from "./shim.ts";

/** The browsers shipping `Set`'s set-algebra members, per browser-compat-data. */
const SETS: NativeVersions = { chrome: 122, edge: 122, firefox: 127, safari: 17 };

/** The set-algebra members of `Set.prototype`. */
export const SET_SHIMS: readonly Shim[] = [
    {
        feature: "javascript.builtins.Set.union",
        owner: Set.prototype,
        name: "union",
        implementation: union,
        native: SETS,
    },
    {
        feature: "javascript.builtins.Set.intersection",
        owner: Set.prototype,
        name: "intersection",
        implementation: intersection,
        native: SETS,
    },
    {
        feature: "javascript.builtins.Set.difference",
        owner: Set.prototype,
        name: "difference",
        implementation: difference,
        native: SETS,
    },
    {
        feature: "javascript.builtins.Set.symmetricDifference",
        owner: Set.prototype,
        name: "symmetricDifference",
        implementation: symmetricDifference,
        native: SETS,
    },
    {
        feature: "javascript.builtins.Set.isSubsetOf",
        owner: Set.prototype,
        name: "isSubsetOf",
        implementation: isSubsetOf,
        native: SETS,
    },
    {
        feature: "javascript.builtins.Set.isSupersetOf",
        owner: Set.prototype,
        name: "isSupersetOf",
        implementation: isSupersetOf,
        native: SETS,
    },
    {
        feature: "javascript.builtins.Set.isDisjointFrom",
        owner: Set.prototype,
        name: "isDisjointFrom",
        implementation: isDisjointFrom,
        native: SETS,
    },
];

/** A set-like value as the set-algebra members read it: its size, membership and keys. */
interface SetRecord {
    /** Its size, a non-negative integer or infinity. */
    readonly size: number;
    /** Report whether it has a member, through its own `has`. */
    readonly has: (value: unknown) => boolean;
    /** Read its keys, through its own `keys` iterator. */
    readonly keys: () => unknown[];
}

/** Combine a set with a set-like value, as `Set.prototype.union` does. */
function union(this: Set<unknown>, other: unknown): Set<unknown> {
    // add each of the other's keys to a copy
    const record = setRecord(other);
    const result = new Set(this);
    for (const key of record.keys()) {
        result.add(key);
    }

    return result;
}

/** Keep the members a set shares with a set-like value, as `Set.prototype.intersection` does. */
function intersection(this: Set<unknown>, other: unknown): Set<unknown> {
    // walk this set when it is the smaller side, keeping the members the other has
    const record = setRecord(other);
    const result = new Set<unknown>();
    if (this.size <= record.size) {
        for (const value of this) {
            if (record.has(value)) {
                result.add(value);
            }
        }
    }
    // walk the other's keys otherwise, keeping the ones this set has
    else {
        for (const key of record.keys()) {
            if (this.has(key)) {
                result.add(key);
            }
        }
    }

    return result;
}

/** Keep the members a set-like value lacks, as `Set.prototype.difference` does. */
function difference(this: Set<unknown>, other: unknown): Set<unknown> {
    // walk this set when it is the smaller side, removing the shared members from a copy
    const record = setRecord(other);
    const result = new Set(this);
    if (this.size <= record.size) {
        for (const value of this) {
            if (record.has(value)) {
                result.delete(value);
            }
        }
    }
    // walk the other's keys otherwise, removing each from the copy
    else {
        for (const key of record.keys()) {
            result.delete(key);
        }
    }

    return result;
}

/** Keep the members only one side holds, as `Set.prototype.symmetricDifference` does. */
function symmetricDifference(this: Set<unknown>, other: unknown): Set<unknown> {
    // remove each shared key from a copy, and add each key only the other holds
    const record = setRecord(other);
    const result = new Set(this);
    for (const key of record.keys()) {
        if (this.has(key)) {
            result.delete(key);
        } else {
            result.add(key);
        }
    }

    return result;
}

/** Report whether a set-like value holds every member, as `Set.prototype.isSubsetOf` does. */
function isSubsetOf(this: Set<unknown>, other: unknown): boolean {
    const record = setRecord(other);

    return this.size <= record.size && Array.from(this).every((value) => record.has(value));
}

/** Report whether a set holds every member of a set-like value, as `isSupersetOf` does. */
function isSupersetOf(this: Set<unknown>, other: unknown): boolean {
    const record = setRecord(other);

    return this.size >= record.size && record.keys().every((key) => this.has(key));
}

/** Report whether a set shares no member with a set-like value, as `isDisjointFrom` does. */
function isDisjointFrom(this: Set<unknown>, other: unknown): boolean {
    // walk the smaller side for a shared member
    const record = setRecord(other);
    const isShared =
        this.size <= record.size
            ? Array.from(this).some((value) => record.has(value))
            : record.keys().some((key) => this.has(key));

    return !isShared;
}

/** Read a set-like value as GetSetRecord does, refusing an invalid one. */
function setRecord(other: unknown): SetRecord {
    // require an object with a numeric size
    if (typeof other !== "object" || other === null) {
        throw new TypeError("set-like value must be an object");
    }
    const size = Number(Reflect.get(other, "size"));
    if (Number.isNaN(size)) {
        throw new TypeError("set-like value's size must be a number");
    }

    // require a non-negative integral size, and callable membership and keys
    const integral = Math.trunc(size);
    if (integral < 0) {
        throw new RangeError("set-like value's size must not be negative");
    }
    const has: unknown = Reflect.get(other, "has");
    const keys: unknown = Reflect.get(other, "keys");
    if (typeof has !== "function" || typeof keys !== "function") {
        throw new TypeError("set-like value's has and keys must be functions");
    }

    return {
        size: integral,
        has: (value) => Boolean(Reflect.apply(has, other, [value])),
        keys: () => keysOf(Reflect.apply(keys, other, [])),
    };
}

/** Read the values an iterator a set-like value's `keys` returned steps through. */
function keysOf(iterator: unknown): unknown[] {
    // require an iterator with a next method
    const next: unknown =
        typeof iterator === "object" && iterator !== null
            ? Reflect.get(iterator, "next")
            : undefined;
    if (typeof next !== "function") {
        throw new TypeError("set-like value's keys must return an iterator");
    }

    // step it until it is done, refusing a step that is not an object
    const values: unknown[] = [];
    let step: unknown = Reflect.apply(next, iterator, []);
    while (typeof step === "object" && step !== null && Reflect.get(step, "done") !== true) {
        values.push(Reflect.get(step, "value"));
        step = Reflect.apply(next, iterator, []);
    }
    if (typeof step !== "object" || step === null) {
        throw new TypeError("set-like value's iterator must step with objects");
    }

    return values;
}
