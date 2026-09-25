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
    // reject values JSON cannot hold
    else if (
        typeof value !== "object" ||
        ![Object.prototype, null].includes(Object.getPrototypeOf(value))
    ) {
        throw new TypeError(`value is not JSON: ${describe(value)}`);
    }

    // sort the defined fields by key and write each one
    const fields = Object.entries(value)
        .filter(([, field]) => field !== undefined)
        .sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0));

    return `{${fields.map(([key, field]) => `${JSON.stringify(key)}:${canonicalize(field)}`).join(",")}}`;
}

/** Hash the canonical form of a JSON value as lowercase hexadecimal SHA-256. */
export async function digest(value: unknown): Promise<string> {
    const bytes = new TextEncoder().encode(canonicalize(value));

    return new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)).toHex();
}

/** Name a value that is not JSON for an error message. */
function describe(value: unknown): string {
    return typeof value === "object" ? (value?.constructor?.name ?? "object") : typeof value;
}
