import { VaultError } from "../error/index.ts";
import type { SecretEnvelope } from "./envelope.ts";

/** Versioned wrapping keys used to encrypt and decrypt data keys. */
export interface Keyring {
    /** Encrypt a data key under the active root key. */
    wrap(
        key: Uint8Array<ArrayBuffer>,
        context: Uint8Array<ArrayBuffer>,
    ): Promise<Pick<SecretEnvelope, "keyId" | "wrappedKey" | "keyNonce">>;
    /** Recover a data key under its recorded root key. */
    unwrap(
        envelope: Pick<SecretEnvelope, "keyId" | "wrappedKey" | "keyNonce">,
        context: Uint8Array<ArrayBuffer>,
    ): Promise<Uint8Array<ArrayBuffer>>;
}

/** Wrap data keys locally using root keys supplied by the host. */
export class LocalKeyring implements Keyring {
    /** Active version for new encryption. */
    readonly active: string;
    /** Non-extractable imported keys retained for older records. */
    readonly #keys: ReadonlyMap<string, CryptoKey>;

    /** Retain imported keys after validating the active version. */
    private constructor(active: string, keys: ReadonlyMap<string, CryptoKey>) {
        this.active = active;
        this.#keys = keys;
    }

    /** Import independently generated 256-bit root keys. */
    static async import(
        active: string,
        values: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
    ): Promise<LocalKeyring> {
        if (!active || !values.has(active)) {
            throw new VaultError("INVALID_KEY", "active root key is missing");
        }

        // validate all versions before accepting the deployment
        const keys = new Map<string, CryptoKey>();
        for (const [id, value] of values) {
            if (!id || value.length !== 32) {
                throw new VaultError("INVALID_KEY", "root keys require a version and 32 bytes");
            }
            keys.set(
                id,
                await crypto.subtle.importKey("raw", value, "AES-GCM", false, [
                    "encrypt",
                    "decrypt",
                ]),
            );
        }

        return new LocalKeyring(active, keys);
    }

    /** Protect a fresh data key under the active root key. */
    async wrap(value: Uint8Array<ArrayBuffer>, context: Uint8Array<ArrayBuffer>) {
        // encrypt the key with a fresh nonce under the active root key
        const nonce = crypto.getRandomValues(new Uint8Array(12));
        const key = this.#get(this.active);
        const wrapped = await crypto.subtle.encrypt(
            { name: "AES-GCM", iv: nonce, additionalData: context },
            key,
            value,
        );

        return {
            keyId: this.active,
            keyNonce: nonce.toBase64(),
            wrappedKey: new Uint8Array(wrapped).toBase64(),
        };
    }

    /** Authenticate a protected key under its original version. */
    async unwrap(
        envelope: Pick<SecretEnvelope, "keyId" | "wrappedKey" | "keyNonce">,
        context: Uint8Array<ArrayBuffer>,
    ) {
        const key = this.#get(envelope.keyId);
        if (!envelope.keyNonce) {
            throw new VaultError("DECRYPTION_FAILED", "protected data key nonce is missing");
        }
        try {
            const value = await crypto.subtle.decrypt(
                {
                    name: "AES-GCM",
                    iv: Uint8Array.fromBase64(envelope.keyNonce),
                    additionalData: context,
                },
                key,
                Uint8Array.fromBase64(envelope.wrappedKey),
            );

            return new Uint8Array(value);
        } catch {
            throw new VaultError("DECRYPTION_FAILED", "data key authentication failed");
        }
    }

    /** Reject unavailable root-key versions explicitly. */
    #get(id: string): CryptoKey {
        const key = this.#keys.get(id);
        if (!key) {
            throw new VaultError("KEY_UNAVAILABLE", "required root key is unavailable");
        }

        return key;
    }
}
