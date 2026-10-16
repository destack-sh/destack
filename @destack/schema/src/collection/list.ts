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

/** Read the item at an index, from the end for a negative one, refusing a missing one. */
export function nth<Item>(list: readonly Item[], index: number): Item {
    const item = list.at(index);
    if (item === undefined) {
        throw new RangeError(`a list of ${list.length} items has no item ${index}`);
    }

    return item;
}
