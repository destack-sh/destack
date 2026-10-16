import { defineSchema, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import type { ObjectReference } from "@destack/sync";

/** A token an authorization server issued (RFC 6749 5.1), as its connection's vault stores it. */
export const Token = defineSchema(
    schema.object({
        /** The access token. */
        accessToken: schema.string().min(1),
        /** The token type the Authorization header sends as its scheme, such as Bearer. */
        tokenType: schema.string().min(1),
        /** The refresh token, absent where the server issues none. */
        refreshToken: schema.string().min(1).exactOptional(),
        /** The access token's expiry in UTC epoch milliseconds, if the server gives one. */
        expiresAt: schema.number().int().exactOptional(),
        /** The scopes the server granted, absent where they equal the requested ones. */
        scopes: schema.array(schema.string()).exactOptional(),
    }),
);
/** A token an authorization server issued. */
export type Token = schema.Infer<typeof Token>;

/** The long-lived material a connection's owner supplies, by the connection's kind. */
export const Material = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** An API key sent in a header. */
            kind: schema.literal("key"),
            /** The key. */
            key: schema.string().min(1),
        }),
        schema.object({
            /** A key signing HTTP messages (RFC 9421). */
            kind: schema.literal("signature"),
            /** The HMAC secret in base64url, or the Ed25519 private key as a JWK's JSON. */
            key: schema.string().min(1),
        }),
        schema.object({
            /** AWS credentials signing with Signature Version 4. */
            kind: schema.literal("sigv4"),
            /** The access key id. */
            accessKeyId: schema.string().min(1),
            /** The secret access key. */
            secretAccessKey: schema.string().min(1),
            /** The session token of temporary credentials. */
            sessionToken: schema.string().min(1).exactOptional(),
        }),
        schema.object({
            /** A private key signing JWT assertions (RFC 7523). */
            kind: schema.literal("jwt"),
            /** The private key as a JWK's JSON. */
            privateKey: schema.string().min(1),
        }),
        schema.object({
            /** A TLS client certificate (RFC 8705). */
            kind: schema.literal("mtls"),
            /** The certificate chain in PEM. */
            certificate: schema.string().min(1),
            /** The private key in PEM. */
            privateKey: schema.string().min(1),
        }),
    ]),
);
/** The long-lived material a connection's owner supplies. */
export type Material = schema.Infer<typeof Material>;

/** What a connection stores in its vault secret. */
const credential = schema.object({
    /** The supplied material, absent for kinds an authorization server issues tokens to alone. */
    material: Material.exactOptional(),
    /** The current token, absent for kinds that send their material. */
    token: Token.exactOptional(),
});

/** What a connection stores in its vault secret, read and written as the secret's JSON value. */
export const Credential = Object.assign(defineSchema(credential), {
    /** Read a secret's value as a credential. */
    read(value: string): Credential {
        return credential.parse(JSON.parse(value));
    },

    /** Write a credential as a secret's value. */
    write(value: Credential): string {
        return JSON.stringify(value);
    },

    /** Require a vault of the scope itself. */
    requireVault(vault: ObjectReference, scope: string): void {
        if (vault.scope !== scope) {
            throw new CallError("BAD_REQUEST", {
                message: `a credential of ${scope} lives in a vault of ${scope}, not of ${vault.scope}`,
            });
        }
    },
});
/** What a connection stores in its vault secret. */
export type Credential = schema.Infer<typeof credential>;
