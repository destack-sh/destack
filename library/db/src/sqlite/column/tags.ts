import { schema } from "@destack/schema";
import { sql } from "drizzle-orm";
import { json } from "./json.ts";

/** User-defined labels indexed by name. */
export const Tags = schema.record(schema.string().min(1).max(128), schema.string().max(256));

/** User-defined labels indexed by name. */
export type Tags = schema.Infer<typeof Tags>;

/** Define a validated label map with an empty SQL default. */
export function tags(name = "tags") {
    return json(name, Tags).notNull().default(sql`'{}'`);
}
