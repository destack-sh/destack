/** Leave out a record's undefined values, as JSON leaves them out, for a schema that reads absent keys. */
export function defined(record: Readonly<Record<string, unknown>>): Record<string, unknown> {
    return Object.fromEntries(Object.entries(record).filter(([, value]) => value !== undefined));
}
