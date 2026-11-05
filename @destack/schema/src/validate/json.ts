import * as z from "zod";
import type { JsonObject, JsonValue } from "../json/json.ts";

/** The JSON Schema of any JSON value: one of its six types, arrays and objects of any items. */
export const JSON_DESCRIPTION: z.core.JSONSchema.BaseSchema = {
    anyOf: [
        { type: "string" },
        { type: "number" },
        { type: "boolean" },
        { type: "null" },
        { type: "array" },
        { type: "object", propertyNames: { type: "string" } },
    ],
};

/** Validate a JSON value, checking its shape in one walk. */
export function json(): z.ZodType<JsonValue> {
    return z.custom<JsonValue>(isJson, { message: "not a JSON value" });
}

/** Validate a JSON object, as a record of JSON values. */
export function jsonObject(): z.ZodType<JsonObject> {
    return z.record(z.string(), json());
}

/** Report whether a schema is the JSON value check `json` makes. */
export function isJsonCheck(schema: z.core.$ZodType): boolean {
    const { def } = schema._zod;

    return def.type === "custom" && "fn" in def && def.fn === isJson;
}

/** Decide whether a value is JSON: scalars, and arrays and plain objects of JSON. */
function isJson(value: unknown): value is JsonValue {
    // take a primitive JSON value
    if (value === null || typeof value === "string" || typeof value === "boolean") {
        return true;
    } else if (typeof value === "number") {
        return Number.isFinite(value);
    }
    // take an array of JSON values
    else if (Array.isArray(value)) {
        return value.every(isJson);
    }
    // take a plain object of JSON values
    else if (typeof value === "object") {
        const prototype: unknown = Object.getPrototypeOf(value);

        return (
            (prototype === Object.prototype || prototype === null) &&
            Object.values(value).every(isJson)
        );
    }
    // refuse anything else
    else {
        return false;
    }
}
