import { defineSchema, schema } from "@destack/schema";
import { compactDecrypt, CompactEncrypt, decodeProtectedHeader, errors } from "jose";
import { IdentityError } from "../error/error.ts";

/** The five base64url parts of a compact JWE, the encrypted key empty for a key agreed directly. */
const COMPACT_JWE = /^[\w-]+\.[\w-]*\.[\w-]+\.[\w-]+\.[\w-]+$/u;

/** The content encryption of every ciphertext. */
const ENCRYPTION = "A256GCM";

/** How a ciphertext's content key reaches its holder: wrapped under a symmetric key, or wrapped under a key agreed with a recipient's public key (RFC 7518 4.6, 4.7). */
export type KeyManagement = "A256GCMKW" | "ECDH-ES+A256KW";

/** Bytes encrypted as a compact JWE (RFC 7516), naming the key that decrypts them and binding the context they were encrypted under, as a KMS ciphertext does. */
export const Ciphertext = Object.assign(defineSchema(schema.string().regex(COMPACT_JWE)), {
    /** Encrypt bytes under a fresh content key that a key wraps or a recipient's key agrees, binding a context. */
    async encrypt(
        plaintext: Uint8Array<ArrayBuffer>,
        key: CryptoKey,
        management: { readonly alg: KeyManagement; readonly kid?: string },
        context: Uint8Array<ArrayBuffer>,
    ): Promise<Ciphertext> {
        return new CompactEncrypt(plaintext)
            .setProtectedHeader({ ...management, enc: ENCRYPTION, ctx: await digest(context) })
            .encrypt(key);
    },

    /** Read the identifier of the key a ciphertext names, absent for none. */
    keyId(ciphertext: Ciphertext): string | undefined {
        return decodeProtectedHeader(ciphertext).kid;
    },

    /** Decrypt a ciphertext with its key, refusing one encrypted under another context. */
    async decrypt(
        ciphertext: Ciphertext,
        key: CryptoKey,
        context: Uint8Array<ArrayBuffer>,
    ): Promise<Uint8Array<ArrayBuffer>> {
        // decrypt it without reporting its inputs
        const decrypted = await compactDecrypt(ciphertext, key, {
            contentEncryptionAlgorithms: [ENCRYPTION],
        }).catch((error: unknown) => {
            if (!(error instanceof errors.JOSEError)) {
                throw error;
            }
            throw new IdentityError("DECRYPTION_FAILED", "ciphertext authentication failed");
        });

        // require the context it was encrypted under
        if (decrypted.protectedHeader["ctx"] !== (await digest(context))) {
            throw new IdentityError("DECRYPTION_FAILED", "ciphertext authentication failed");
        }

        return new Uint8Array(decrypted.plaintext);
    },
});
/** Bytes encrypted as a compact JWE. */
export type Ciphertext = string;

/** Digest an encryption context for a ciphertext's protected header, which authenticates it without revealing it. */
async function digest(context: Uint8Array<ArrayBuffer>): Promise<string> {
    const hash = await crypto.subtle.digest("SHA-256", context);

    return new Uint8Array(hash).toBase64({ alphabet: "base64url", omitPadding: true });
}
