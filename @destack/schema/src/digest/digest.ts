import * as schema from "../validate/index.ts";
import { defineSchema } from "../inspect/schema.ts";
import { canonicalize } from "../json/json.ts";

/** The schema of a SHA-256 digest as lowercase hexadecimal. */
const digestSchema = defineSchema(schema.string().regex(/^[0-9a-f]{64}$(?![\s\S])/u));

/** A SHA-256 digest as lowercase hexadecimal. */
export type Digest = schema.Infer<typeof digestSchema>;

/** A SHA-256 digest as lowercase hexadecimal. */
export const Digest = Object.assign(digestSchema, {
    /** Hash bytes, or the UTF-8 bytes of text. */
    of: hash,

    /** Hash a JSON value's canonical form. */
    json(value: unknown): Promise<Digest> {
        return hash(canonicalize(value));
    },
});

/** Hash bytes, or the UTF-8 bytes of text, with SHA-256. */
async function hash(content: string | Uint8Array<ArrayBuffer>): Promise<Digest> {
    const bytes = typeof content === "string" ? new TextEncoder().encode(content) : content;

    return new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)).toHex();
}

/** A committed Git object identifier: a SHA-1 or SHA-256 digest in hexadecimal. */
export const Commit = defineSchema(
    schema.string().regex(/^(?:[a-f0-9]{40}|[a-f0-9]{64})$(?![\s\S])/u),
);
/** A committed Git object identifier. */
export type Commit = schema.Infer<typeof Commit>;
