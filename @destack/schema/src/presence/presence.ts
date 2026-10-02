/** Read a value that must be present, refusing null and undefined. */
export function present<Value>(value: Value | null | undefined, what: string): Value {
    if (value === null || value === undefined) {
        throw new TypeError(`${what} is missing`);
    }

    return value;
}
