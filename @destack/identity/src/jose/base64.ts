import { schema } from "@destack/schema";
import { IdentityError } from "../error/error.ts";

/** A JSON object of a segment. */
const JSON_OBJECT = schema.record(schema.string(), schema.unknown());

/** The characters of an unpadded base64url segment (RFC 7515 2). */
const SEGMENT = /^[A-Za-z0-9_-]*$/u;

/** The message of an invalid base64url segment. */
const INVALID_SEGMENT = "the token has an invalid base64url segment";

/** Base64url segments of compact serializations, unpadded. */
export const Base64Url = {
    /** Encode bytes, or the UTF-8 bytes of text. */
    encode(value: Uint8Array | string): string {
        const bytes = typeof value === "string" ? new TextEncoder().encode(value) : value;

        return bytes.toBase64({ alphabet: "base64url", omitPadding: true });
    },

    /** Decode a segment, refusing one with other characters. */
    decode(segment: string): Uint8Array<ArrayBuffer> {
        if (!SEGMENT.test(segment)) {
            throw IdentityError.of("TOKEN_MALFORMED", INVALID_SEGMENT);
        }
        try {
            return Uint8Array.fromBase64(segment, { alphabet: "base64url" });
        } catch (error) {
            throw IdentityError.of("TOKEN_MALFORMED", INVALID_SEGMENT, { cause: error });
        }
    },

    /** Decode a segment's JSON object, refusing anything else. */
    object(segment: string): Record<string, unknown> {
        return Base64Url.json(Base64Url.decode(segment));
    },

    /** Read UTF-8 bytes as a JSON object, refusing anything else. */
    json(bytes: Uint8Array): Record<string, unknown> {
        // parse the UTF-8 JSON, refusing invalid text
        let value: unknown;
        try {
            value = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes));
        } catch (error) {
            throw IdentityError.of("TOKEN_MALFORMED", "the token has an invalid JSON segment", {
                cause: error,
            });
        }

        // refuse a value other than an object
        const parsed = JSON_OBJECT.safeParse(value);
        if (!parsed.success || Array.isArray(value)) {
            throw IdentityError.of("TOKEN_MALFORMED", "the token's JSON segment is not an object");
        }

        return parsed.data;
    },
};
