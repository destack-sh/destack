import type { Metadata } from "@tufjs/models";

/** A signed metadata document as the TUF models read it. */
export type MetadataDocument = Parameters<typeof Metadata.fromJSON>[1];

/** One value inside a signed metadata document. */
type MetadataValue = MetadataDocument[string];

/** Parse a signed metadata document, or throw when its text is not a JSON object. */
export function parseDocument(text: string): MetadataDocument {
    const value: unknown = JSON.parse(text);
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
        throw new TypeError("a metadata document is not a JSON object");
    }

    return fieldsOf(value);
}

/** Copy a parsed JSON object's fields as metadata values. */
function fieldsOf(value: object): MetadataDocument {
    return Object.fromEntries(
        Object.entries(value).map(([key, item]: [string, unknown]) => [key, valueOf(item)]),
    );
}

/** Copy one parsed JSON value as a metadata value. */
function valueOf(value: unknown): MetadataValue {
    if (
        value === null ||
        typeof value === "string" ||
        typeof value === "number" ||
        typeof value === "boolean"
    ) {
        return value;
    }
    if (Array.isArray(value)) {
        return value.map((item: unknown) => valueOf(item));
    }
    if (typeof value === "object") {
        return fieldsOf(value);
    }

    // reject values that JSON cannot hold
    throw new TypeError(`a metadata document holds a ${typeof value}`);
}
