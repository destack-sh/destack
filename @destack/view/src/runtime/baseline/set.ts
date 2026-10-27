/** Combine a set with a set-like value, as `Set.prototype.union` does. */
export function union<T>(this: Set<T>, other: ReadonlySetLike<T>): Set<T> {
    return new Set([...this, ...keysOf(other)]);
}

/** Keep the members a set shares with a set-like value, as `Set.prototype.intersection` does. */
export function intersection<T>(this: Set<T>, other: ReadonlySetLike<T>): Set<T> {
    return new Set([...this].filter((value) => other.has(value)));
}

/** Keep the members a set-like value lacks, as `Set.prototype.difference` does. */
export function difference<T>(this: Set<T>, other: ReadonlySetLike<T>): Set<T> {
    return new Set([...this].filter((value) => !other.has(value)));
}

/** Keep the members only one side holds, as `Set.prototype.symmetricDifference` does. */
export function symmetricDifference<T>(this: Set<T>, other: ReadonlySetLike<T>): Set<T> {
    const others = keysOf(other);

    return new Set([
        ...[...this].filter((value) => !others.includes(value)),
        ...others.filter((value) => !this.has(value)),
    ]);
}

/** Report whether a set-like value holds every member, as `Set.prototype.isSubsetOf` does. */
export function isSubsetOf<T>(this: Set<T>, other: ReadonlySetLike<T>): boolean {
    return [...this].every((value) => other.has(value));
}

/** Report whether a set holds every member of a set-like value, as `isSupersetOf` does. */
export function isSupersetOf<T>(this: Set<T>, other: ReadonlySetLike<T>): boolean {
    return keysOf(other).every((value) => this.has(value));
}

/** Report whether a set shares no member with a set-like value, as `isDisjointFrom` does. */
export function isDisjointFrom<T>(this: Set<T>, other: ReadonlySetLike<T>): boolean {
    return [...this].every((value) => !other.has(value));
}

/** Read a set-like value's members by stepping its iterator, as the set-algebra methods do. */
function keysOf<T>(other: ReadonlySetLike<T>): T[] {
    const values: T[] = [];
    const iterator = other.keys();
    for (let step = iterator.next(); step.done !== true; step = iterator.next()) {
        values.push(step.value);
    }

    return values;
}
