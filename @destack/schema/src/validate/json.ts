import * as z from "zod";
import type { JsonValue } from "../json/json.ts";

/** Validate a JSON value, typed as readers see it. */
export function json(): z.ZodType<JsonValue> {
    return z.json();
}
