/** Pair the items of two aligned lists, refusing lists of different lengths. */
export function zip<Left, Right>(left: readonly Left[], right: readonly Right[]): [Left, Right][] {
    // require equal lengths
    if (left.length !== right.length) {
        throw new RangeError(`aligned lists have ${left.length} and ${right.length} items`);
    }

    // walk both lists together
    const pairs: [Left, Right][] = [];
    const rights = right[Symbol.iterator]();
    for (const item of left) {
        const next = rights.next();
        if (next.done === true) {
            throw new RangeError("an aligned list ended early");
        }
        pairs.push([item, next.value]);
    }

    return pairs;
}

/** Read an item of a list aligned with another, refusing a missing one. */
export function aligned<Item>(list: readonly Item[], index: number): Item {
    const item = list[index];
    if (item === undefined) {
        throw new RangeError(`an aligned list of ${list.length} items lacks item ${index}`);
    }

    return item;
}
