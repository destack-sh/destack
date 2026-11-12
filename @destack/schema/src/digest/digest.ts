import * as schema from "../validate/index.ts";
import { canonicalize } from "../json/json.ts";

/** The format of a SHA-256 digest: 64 lowercase hexadecimal digits. */
const DIGEST_FORMAT = /^[0-9a-f]{64}$/u;

/** The schema of a SHA-256 digest as lowercase hexadecimal. */
const digestSchema = schema.string().regex(DIGEST_FORMAT).brand<"Digest">();

/** The hashing a digest runs on: WebCrypto's, taking a stream of chunks as Deno's does. */
export interface DigestCrypto {
    /** Hash bytes, or a stream of chunks, with an algorithm. */
    readonly digest: (
        algorithm: "SHA-256",
        data: Uint8Array<ArrayBuffer> | AsyncIterable<Uint8Array>,
    ) => Promise<ArrayBuffer>;
}

/** A SHA-256 digest as lowercase hexadecimal. */
export type Digest = schema.Infer<typeof digestSchema>;

/** A SHA-256 digest as lowercase hexadecimal. */
export const Digest = Object.assign(digestSchema, {
    /** Hash bytes, a stream of chunks, or the UTF-8 bytes of text. */
    async of(
        content: string | Uint8Array<ArrayBuffer> | AsyncIterable<Uint8Array>,
        crypto: DigestCrypto,
    ): Promise<Digest> {
        const data = typeof content === "string" ? new TextEncoder().encode(content) : content;
        const digest = await crypto.digest("SHA-256", data);

        // oxlint-disable-next-line typescript/consistent-type-assertions, typescript/no-unsafe-type-assertion -- a SHA-256 hash is 64 hexadecimal digits
        return new Uint8Array(digest).toHex() as Digest;
    },

    /** Report whether text has a digest's format. */
    is(value: string): value is Digest {
        return DIGEST_FORMAT.test(value);
    },

    /** Hash a JSON value's canonical form. */
    json(value: unknown, crypto: DigestCrypto): Promise<Digest> {
        return Digest.of(canonicalize(value), crypto);
    },
});
