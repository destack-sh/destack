import * as z from "zod";
import type { JsonObject, JsonValue } from "../json/json.ts";

/** Validate a JSON value, typed as readers see it. */
export function json(): z.ZodType<JsonValue> {
    return z.json();
}

/** Validate a JSON object, as zod's record of JSON values. */
export function jsonObject(): z.ZodType<JsonObject> {
    return z.record(z.string(), json());
}
