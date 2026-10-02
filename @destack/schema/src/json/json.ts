import type * as schema from "../validate/index.ts";

/** A JSON value. */
export type JsonValue = schema.Infer<ReturnType<typeof schema.json>>;

/** Serialize a JSON value with object keys sorted by UTF-16 code units, omitting undefined fields. */
export function canonicalize(value: unknown): string {
    // write strings, booleans, null and finite numbers directly
    if (
        value === null ||
        typeof value === "string" ||
        typeof value === "boolean" ||
        (typeof value === "number" && Number.isFinite(value))
    ) {
        return JSON.stringify(value);
    }
    // write arrays element by element
    else if (Array.isArray(value)) {
        return `[${value.map(canonicalize).join(",")}]`;
    }
    // reject values JSON cannot represent
    else if (typeof value !== "object" || !isPlain(value)) {
        throw new TypeError(`value is not JSON: ${describe(value)}`);
    }

    // sort the defined fields by key and write each one
    const fields = Object.entries(value)
        .filter(([, field]) => field !== undefined)
        .toSorted(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0));

    return `{${fields.map(([key, field]) => `${JSON.stringify(key)}:${canonicalize(field)}`).join(",")}}`;
}

/** Name a value that is not JSON for an error message. */
function describe(value: unknown): string {
    return typeof value === "object" ? (value?.constructor?.name ?? "object") : typeof value;
}

/** Report whether an object is a plain record, made by a literal or without a prototype. */
function isPlain(value: object): boolean {
    const prototype: unknown = Object.getPrototypeOf(value);

    return prototype === Object.prototype || prototype === null;
}
