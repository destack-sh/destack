import * as schema from "../validate/index.ts";
import { canonicalize } from "../json/json.ts";

/** The schema of a SHA-256 digest as lowercase hexadecimal. */
const digestSchema = schema
    .string()
    .regex(/^[0-9a-f]{64}$/u)
    .brand<"Digest">();

/** The WebCrypto hashing a digest runs on. */
export interface DigestCrypto {
    /** Hash bytes with an algorithm. */
    readonly digest: (algorithm: "SHA-256", data: Uint8Array<ArrayBuffer>) => Promise<ArrayBuffer>;
}

/** A SHA-256 digest as lowercase hexadecimal. */
export type Digest = schema.Infer<typeof digestSchema>;

/** A SHA-256 digest as lowercase hexadecimal. */
export const Digest = Object.assign(digestSchema, {
    /** Hash bytes, or the UTF-8 bytes of text. */
    async of(content: string | Uint8Array<ArrayBuffer>, crypto: DigestCrypto): Promise<Digest> {
        const bytes = typeof content === "string" ? new TextEncoder().encode(content) : content;
        const digest = await crypto.digest("SHA-256", bytes);

        return digestSchema.parse(new Uint8Array(digest).toHex());
    },

    /** Hash a JSON value's canonical form. */
    json(value: unknown, crypto: DigestCrypto): Promise<Digest> {
        return Digest.of(canonicalize(value), crypto);
    },
});
