/** Read the only item of a list, refusing none or several. */
export function single<Item>(items: readonly Item[]): Item {
    const [item] = items;
    if (item === undefined || items.length !== 1) {
        throw new RangeError(`expected one item, found ${items.length}`);
    }

    return item;
}
