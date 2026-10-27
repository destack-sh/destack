/** The set-algebra methods of `Set.prototype`, as the TC39 Set methods proposal defines them. */
export const SetMethods = {
    /** Combine a set with a set-like value, as `Set.prototype.union` does. */
    union<Value>(this: Set<Value>, other: ReadonlySetLike<Value>): Set<Value> {
        return new Set([...this, ...keysOf(other)]);
    },

    /** Keep the members a set shares with a set-like value, as `intersection` does. */
    intersection<Value>(this: Set<Value>, other: ReadonlySetLike<Value>): Set<Value> {
        return new Set([...this].filter((value) => other.has(value)));
    },

    /** Keep the members a set-like value lacks, as `Set.prototype.difference` does. */
    difference<Value>(this: Set<Value>, other: ReadonlySetLike<Value>): Set<Value> {
        return new Set([...this].filter((value) => !other.has(value)));
    },

    /** Keep the members only one side holds, as `Set.prototype.symmetricDifference` does. */
    symmetricDifference<Value>(this: Set<Value>, other: ReadonlySetLike<Value>): Set<Value> {
        const others = keysOf(other);

        return new Set([
            ...[...this].filter((value) => !others.includes(value)),
            ...others.filter((value) => !this.has(value)),
        ]);
    },

    /** Report whether a set-like value holds every member, as `Set.prototype.isSubsetOf` does. */
    isSubsetOf<Value>(this: Set<Value>, other: ReadonlySetLike<Value>): boolean {
        return [...this].every((value) => other.has(value));
    },

    /** Report whether a set holds every member of a set-like value, as `isSupersetOf` does. */
    isSupersetOf<Value>(this: Set<Value>, other: ReadonlySetLike<Value>): boolean {
        return keysOf(other).every((value) => this.has(value));
    },

    /** Report whether a set shares no member with a set-like value, as `isDisjointFrom` does. */
    isDisjointFrom<Value>(this: Set<Value>, other: ReadonlySetLike<Value>): boolean {
        return [...this].every((value) => !other.has(value));
    },
};

/** Read a set-like value's members by stepping its iterator, as the set-algebra methods do. */
function keysOf<Value>(other: ReadonlySetLike<Value>): Value[] {
    // step the iterator into an array
    const values: Value[] = [];
    const iterator = other.keys();
    for (let step = iterator.next(); step.done !== true; step = iterator.next()) {
        values.push(step.value);
    }

    return values;
}
