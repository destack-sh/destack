import * as schema from "../validate/index.ts";

/** A Git commit's object ID: SHA-1 or SHA-256 as lowercase hexadecimal. */
export const Commit = schema
    .string()
    .regex(/^(?:[a-f0-9]{40}|[a-f0-9]{64})$/u)
    .brand<"Commit">();
/** A Git commit's object ID. */
export type Commit = schema.Infer<typeof Commit>;
