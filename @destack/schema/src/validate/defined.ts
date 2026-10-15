/** Leave out a value's undefined entries at every depth, as JSON leaves them out. */
export function defined(value: unknown): unknown {
    // descend into lists, and into plain objects leaving out their undefined entries
    if (Array.isArray(value)) {
        const entries: readonly unknown[] = value;

        return entries.map((entry) => defined(entry));
    } else if (
        typeof value === "object" &&
        value !== null &&
        Object.getPrototypeOf(value) === Object.prototype
    ) {
        return Object.fromEntries(
            Object.entries(value).flatMap(([key, entry]) =>
                entry === undefined ? [] : [[key, defined(entry)]],
            ),
        );
    }

    return value;
}
