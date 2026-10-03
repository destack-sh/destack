/** Leave out a record's undefined values, as JSON leaves them out, for a schema that reads absent keys. */
export function defined<Value>(
    record: Readonly<Record<string, Value | undefined>>,
): Record<string, Value> {
    return Object.fromEntries(
        Object.entries(record).flatMap(([key, value]): [string, Value][] =>
            value === undefined ? [] : [[key, value]],
        ),
    );
}
