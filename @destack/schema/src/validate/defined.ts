/** A value with its undefined entries left out at every depth, as JSON leaves them out, each key that may be undefined optional. */
export type Defined<Value> = Value extends readonly (infer Entry)[]
    ? Value extends Entry[]
        ? Defined<Entry>[]
        : readonly Defined<Entry>[]
    : Value extends Uint8Array
      ? Value
      : Value extends object
        ? string extends keyof Value
            ? { [Key in keyof Value]: Defined<Exclude<Value[Key], undefined>> }
            : DefinedEntries<Value>
        : Value;

/** An object's entries with their undefined values left out, each key that may be undefined optional. */
type DefinedEntries<Value extends object> = {
    [Key in keyof Value as undefined extends Value[Key] ? never : Key]: Defined<Value[Key]>;
} & {
    [Key in keyof Value as undefined extends Value[Key] ? Key : never]?: Defined<
        Exclude<Value[Key], undefined>
    >;
};

/** Leave out a value's undefined entries at every depth, as JSON leaves them out. */
export function defined<Value>(value: Value): Defined<Value>;
/**
 * Leave out a value's undefined entries at every depth.
 *
 * @construct leaving out an undefined entry keeps every other entry, so the value has the type its undefined entries leave.
 */
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
