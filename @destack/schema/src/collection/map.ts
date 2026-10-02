/** Read the value a map keeps for a key it must have, refusing a missing one. */
export function found<Key, Value>(map: ReadonlyMap<Key, Value>, key: Key): Value {
    const value = map.get(key);
    if (value === undefined) {
        throw new RangeError(`a map of ${map.size} entries lacks a key it must have`);
    }

    return value;
}
