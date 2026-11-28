/** Share a value's unchanged parts with an earlier value, as TanStack's replaceEqualDeep does. */
export function share<Value>(earlier: unknown, value: Value): Value {
    // build the shared value, deep-equal to the value by construction
    return shareUnknown(earlier, value) as Value;
}

/** Share a value's unchanged parts with an earlier value, of any shape. */
function shareUnknown(earlier: unknown, value: unknown): unknown {
    // keep an equal leaf
    if (Object.is(earlier, value)) {
        return value;
    }

    // share the elements of two arrays, each by its id where it has one, else by its index
    if (Array.isArray(earlier) && Array.isArray(value)) {
        return shareArray(earlier, value);
    }

    // share the entries of two plain objects
    if (isPlain(earlier) && isPlain(value)) {
        return shareObject(earlier, value);
    }

    return value;
}

/** Share an array's elements with an earlier array's, keeping it when all match. */
function shareArray(earlier: readonly unknown[], value: readonly unknown[]): readonly unknown[] {
    // match each element to the earlier one with its id, else to the one at its index
    const byId = new Map<unknown, unknown>();
    for (const element of earlier) {
        if (isPlain(element) && "id" in element) {
            byId.set(element["id"], element);
        }
    }
    const shared = value.map((element, index) =>
        shareUnknown(
            isPlain(element) && "id" in element ? byId.get(element["id"]) : earlier[index],
            element,
        ),
    );

    // keep the earlier array when every element is the same, in the same order
    const isSame =
        shared.length === earlier.length &&
        shared.every((element, index) => element === earlier[index]);

    return isSame ? earlier : shared;
}

/** Share a plain object's entries with an earlier one's, keeping it when all match. */
function shareObject(
    earlier: Readonly<Record<string, unknown>>,
    value: Readonly<Record<string, unknown>>,
): Readonly<Record<string, unknown>> {
    // share each entry with the earlier entry under its key
    const keys = Object.keys(value);
    const shared = Object.fromEntries(
        keys.map((key) => [key, shareUnknown(earlier[key], value[key])]),
    );

    // keep the earlier object when it has the same keys, each with the same value
    const isSame =
        keys.length === Object.keys(earlier).length &&
        keys.every((key) => Object.hasOwn(earlier, key) && shared[key] === earlier[key]);

    return isSame ? earlier : shared;
}

/** Report whether a value is a plain object, whose entries sharing compares. */
function isPlain(value: unknown): value is Readonly<Record<string, unknown>> {
    if (typeof value !== "object" || value === null) {
        return false;
    }
    const prototype: unknown = Object.getPrototypeOf(value);

    return prototype === Object.prototype || prototype === null;
}
