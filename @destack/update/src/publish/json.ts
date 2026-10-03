import type { Timestamp } from "@tufjs/models";

/** A JSON object as the TUF metadata readers take it. */
type JSONObject = Parameters<typeof Timestamp.fromJSON>[0];

/** A JSON value as the TUF metadata readers take it. */
type JSONValue = JSONObject[string];

/** Parse a metadata document into the JSON object its typed reader validates. */
export function parseObject(text: string): JSONObject {
    // require an object at the top of the document
    const value: unknown = JSON.parse(text);
    if (!isObject(value)) {
        throw new TypeError("expected a JSON object document");
    }

    return value;
}

/** Report whether a parsed value is a JSON object. */
function isObject(value: unknown): value is JSONObject {
    return (
        typeof value === "object" &&
        value !== null &&
        !Array.isArray(value) &&
        Object.values(value).every(isValue)
    );
}

/** Report whether a parsed value is a JSON value. */
function isValue(value: unknown): value is JSONValue {
    // accept scalars
    if (
        value === null ||
        typeof value === "boolean" ||
        typeof value === "number" ||
        typeof value === "string"
    ) {
        return true;
    }
    // accept arrays of JSON values
    else if (Array.isArray(value)) {
        return value.every(isValue);
    }
    // accept objects of JSON values
    else {
        return isObject(value);
    }
}
