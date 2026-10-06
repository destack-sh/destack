import { Ciphertext, IdentityError } from "@destack/identity";

/** The bytes of a vault key, an AES-256 key. */
export const VAULT_KEY_BYTES = 32;

/** The key of one vault, decrypted for its secrets' values, which a value names by its identifier. */
export class VaultKey {
    /** The key's identifier, which each value it encrypts names. */
    readonly id: string;
    /** The imported, non-extractable key. */
    readonly #key: CryptoKey;

    /** Keep an imported vault key. */
    private constructor(id: string, key: CryptoKey) {
        this.id = id;
        this.#key = key;
    }

    /** Import a vault key's bytes as a key that encrypts and never leaves. */
    static async import(id: string, bytes: Uint8Array<ArrayBuffer>): Promise<VaultKey> {
        const key = await crypto.subtle.importKey("raw", bytes, "AES-GCM", false, [
            "encrypt",
            "decrypt",
        ]);

        return new VaultKey(id, key);
    }

    /** Encrypt bytes under a fresh data key the vault key wraps, binding an encryption context. */
    encrypt(
        plaintext: Uint8Array<ArrayBuffer>,
        context: Uint8Array<ArrayBuffer>,
    ): Promise<Ciphertext> {
        return Ciphertext.encrypt(
            plaintext,
            this.#key,
            { alg: "A256GCMKW", kid: this.id },
            context,
        );
    }

    /** Decrypt a ciphertext the vault key encrypted, refusing one under another key. */
    async decrypt(
        ciphertext: Ciphertext,
        context: Uint8Array<ArrayBuffer>,
    ): Promise<Uint8Array<ArrayBuffer>> {
        if (Ciphertext.keyId(ciphertext) !== this.id) {
            throw new IdentityError(
                "KEY_UNAVAILABLE",
                "the value is encrypted under another vault key",
            );
        }

        return Ciphertext.decrypt(ciphertext, this.#key, context);
    }
}
