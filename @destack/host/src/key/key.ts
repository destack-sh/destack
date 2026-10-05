import { defineSchema, schema } from "@destack/schema";

/** A key wrapped under a keyring: the wrapping key's version, and the wrapped bytes with their nonce. */
export const WrappedKey = defineSchema(
    schema.object({
        /** The version of the key it is wrapped under. */
        keyId: schema.string().min(1),
        /** The base64 wrapped bytes with their authentication tag. */
        wrappedKey: schema.base64(),
        /** The base64 nonce. */
        keyNonce: schema.base64(),
    }),
);
/** A key wrapped under a keyring. */
export type WrappedKey = schema.Infer<typeof WrappedKey>;
