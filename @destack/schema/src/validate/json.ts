import * as z from "zod";
import type { JsonValue } from "../json/json.ts";

/** An object schema whose fields parse to JSON values. */
export type JsonObject = z.ZodObject<{ readonly [Field in string]: z.ZodType<JsonValue> }>;

/** Validate a JSON value, typed as readers see it. */
export function json(): z.ZodType<JsonValue> {
    return z.json();
}
