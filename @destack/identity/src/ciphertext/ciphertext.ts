import type { Crypto, Environment } from "@destack/environment";
import { schema } from "@destack/schema";
import { IdentityError } from "../error/error.ts";
import { Base64Url } from "../jose/base64.ts";
import { Jwe, type KeyManagement } from "../jose/jwe.ts";
import { Label } from "../label/label.ts";

/** The five base64url segments of a compact JWE under a wrapped content key. */
const COMPACT_JWE = /^[\w-]+\.[\w-]+\.[\w-]+\.[\w-]+\.[\w-]+$/u;

/** Bytes encrypted as a compact JWE (RFC 7516), like a KMS ciphertext. */
export const Ciphertext = Object.assign(schema.string().regex(COMPACT_JWE), {
    /** Encrypt bytes under a fresh content key, binding a context. */
    async encrypt(
        plaintext: Uint8Array<ArrayBuffer>,
        key: CryptoKey,
        management: { readonly alg: KeyManagement; readonly kid?: string },
        context: Label,
        environment: Pick<Environment, "crypto" | "random">,
    ): Promise<Ciphertext> {
        const digested = await digest(Label.encode(context), environment.crypto);
        const header = { ...management, enc: "A256GCM" as const, ctx: digested };

        return Jwe.encrypt(plaintext, header, key, environment);
    },

    /** Read the identifier of the key a ciphertext references, absent for none. */
    keyId(ciphertext: Ciphertext): string | undefined {
        const { kid } = Jwe.header(ciphertext);

        return typeof kid === "string" ? kid : undefined;
    },

    /** Decrypt a ciphertext with its key, refusing one encrypted under another context. */
    async decrypt(
        ciphertext: Ciphertext,
        key: CryptoKey,
        context: Label,
        crypto: Crypto,
    ): Promise<Uint8Array<ArrayBuffer>> {
        // decrypt it, then require the context it was encrypted under
        const decrypted = await Jwe.decrypt(ciphertext, key, crypto);
        if (decrypted.header["ctx"] !== (await digest(Label.encode(context), crypto))) {
            throw IdentityError.of("DECRYPTION_FAILED", "ciphertext authentication failed");
        }

        return decrypted.plaintext;
    },
});
/** Bytes encrypted as a compact JWE. */
export type Ciphertext = string;

/** Digest an encryption context for a ciphertext's protected header. */
async function digest(context: Uint8Array<ArrayBuffer>, crypto: Crypto): Promise<string> {
    return Base64Url.encode(new Uint8Array(await crypto.digest("SHA-256", context)));
}
