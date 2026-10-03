import { defineSchema, schema } from "@destack/schema";
import { v7 } from "uuid";
import type { Keychain } from "./keychain.ts";

/** The bytes of a root key, an AES-256 key. */
const ROOT_KEY_BYTES = 32;

/** The bytes of an AES-GCM nonce, the 96 bits NIST SP 800-38D recommends. */
const NONCE_BYTES = 12;

/** The root keys a keychain keeps under one name: each version's bytes, and the active version. */
const Keyset = schema.object({
    /** The version new data keys are wrapped under. */
    active: schema.string().min(1),
    /** The base64 bytes of each version. */
    keys: schema.record(schema.string(), schema.base64()),
});
/** The root keys a keychain keeps under one name. */
type Keyset = schema.Infer<typeof Keyset>;

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

/** A failure of a keyring's keys, without their bytes. */
export class KeyringError extends Error {
    /** The failure's category. */
    readonly code: "KEY_UNAVAILABLE" | "DECRYPTION_FAILED" | "INVALID_KEY";

    /** Report a failure without its cryptographic inputs. */
    constructor(code: KeyringError["code"], message: string) {
        super(message);
        this.name = "KeyringError";
        this.code = code;
    }
}

/** Versioned wrapping keys used to encrypt and decrypt data keys. */
export interface Keyring {
    /** Encrypt a data key under the active root key. */
    wrap(key: Uint8Array<ArrayBuffer>, context: Uint8Array<ArrayBuffer>): Promise<WrappedKey>;
    /** Recover a data key under its recorded root key. */
    unwrap(wrapped: WrappedKey, context: Uint8Array<ArrayBuffer>): Promise<Uint8Array<ArrayBuffer>>;
}

/** Wrap data keys locally using root keys supplied by the host. */
export class LocalKeyring implements Keyring {
    /** Active version for new encryption. */
    readonly active: string;
    /** Non-extractable imported keys retained for older records. */
    readonly #keys: ReadonlyMap<string, CryptoKey>;

    /** Keep imported keys under their versions with the active one. */
    private constructor(active: string, keys: ReadonlyMap<string, CryptoKey>) {
        this.active = active;
        this.#keys = keys;
    }

    /** Open the root keys a keychain keeps under a name, generating the first one when none is kept. */
    static async open(keychain: Keychain, name: string): Promise<LocalKeyring> {
        // keep a first key when the keychain has none
        const kept = await keychain.load(name);
        const keys = kept === undefined ? LocalKeyring.#generate() : Keyset.parse(JSON.parse(kept));
        if (kept === undefined) {
            await keychain.save(name, JSON.stringify(keys));
        }

        return LocalKeyring.#from(keys);
    }

    /** Keep a new active root key beside the others. */
    static async rotate(keychain: Keychain, name: string): Promise<LocalKeyring> {
        const keys = LocalKeyring.#generate(await LocalKeyring.#load(keychain, name));
        await keychain.save(name, JSON.stringify(keys));

        return LocalKeyring.#from(keys);
    }

    /** Forget a root key, refusing the active one and one the keychain does not keep. */
    static async retire(keychain: Keychain, name: string, id: string): Promise<LocalKeyring> {
        // refuse forgetting the active key or an unknown one
        const keys = await LocalKeyring.#load(keychain, name);
        if (keys.active === id) {
            throw new KeyringError("INVALID_KEY", "cannot retire the active root key");
        } else if (!Object.hasOwn(keys.keys, id)) {
            throw new KeyringError("INVALID_KEY", `no root key ${id} is kept as ${name}`);
        }

        // keep the other keys
        const { [id]: _retired, ...others } = keys.keys;
        const kept = { active: keys.active, keys: others };
        await keychain.save(name, JSON.stringify(kept));

        return LocalKeyring.#from(kept);
    }

    /** Import independently generated 256-bit root keys. */
    static async import(
        active: string,
        values: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
    ): Promise<LocalKeyring> {
        if (active === "" || !values.has(active)) {
            throw new KeyringError("INVALID_KEY", "active root key is missing");
        }

        // validate all versions before accepting the deployment
        const keys = new Map<string, CryptoKey>();
        for (const [id, value] of values) {
            if (id === "" || value.length !== ROOT_KEY_BYTES) {
                throw new KeyringError("INVALID_KEY", "root keys require a version and 32 bytes");
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
        const nonce = crypto.getRandomValues(new Uint8Array(NONCE_BYTES));
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
    async unwrap(wrapped: WrappedKey, context: Uint8Array<ArrayBuffer>) {
        // decrypt the key under the version it names
        const key = this.#get(wrapped.keyId);
        try {
            const value = await crypto.subtle.decrypt(
                {
                    name: "AES-GCM",
                    iv: Uint8Array.fromBase64(wrapped.keyNonce),
                    additionalData: context,
                },
                key,
                Uint8Array.fromBase64(wrapped.wrappedKey),
            );

            return new Uint8Array(value);
        } catch (error) {
            // report a failed authentication without its inputs
            if (!(error instanceof DOMException && error.name === "OperationError")) {
                throw error;
            }
            throw new KeyringError("DECRYPTION_FAILED", "data key authentication failed");
        }
    }

    /** Read the root keys a keychain keeps under an existing name. */
    static async #load(keychain: Keychain, name: string): Promise<Keyset> {
        const kept = await keychain.load(name);
        if (kept === undefined) {
            throw new KeyringError("KEY_UNAVAILABLE", `no root keys are kept as ${name}`);
        }

        return Keyset.parse(JSON.parse(kept));
    }

    /** Import a kept keyset. */
    static #from(keys: Keyset): Promise<LocalKeyring> {
        return LocalKeyring.import(
            keys.active,
            new Map(
                Object.entries(keys.keys).map(([id, value]) => [id, Uint8Array.fromBase64(value)]),
            ),
        );
    }

    /** Add a fresh active root key to a keyset, or start one with it. */
    static #generate(keys?: Keyset): Keyset {
        const id = v7();
        const value = crypto.getRandomValues(new Uint8Array(ROOT_KEY_BYTES));

        return { active: id, keys: { ...keys?.keys, [id]: value.toBase64() } };
    }

    /** Reject unavailable root-key versions explicitly. */
    #get(id: string): CryptoKey {
        const key = this.#keys.get(id);
        if (key === undefined) {
            throw new KeyringError("KEY_UNAVAILABLE", "required root key is unavailable");
        }

        return key;
    }
}
