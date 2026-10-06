import type { JWK } from "jose";
import { schema } from "@destack/schema";
import { Ciphertext } from "../ciphertext/ciphertext.ts";
import { Derivation, type Deriver } from "../derivation/derivation.ts";
import { IdentityError } from "../error/error.ts";
import { Keychain } from "../keychain/keychain.ts";

/** The bytes of a root key, an AES-256 key. */
const ROOT_KEY_BYTES = 32;

/** The label of the key a root key encrypts under, apart from every secret it derives. */
const ENCRYPTION_LABEL = "destack keyring encryption v1";

/** The root keys a keychain keeps under one name: each version's bytes, and the active version. */
const Keyset = schema.object({
    /** The version new ciphertexts are encrypted under. */
    active: schema.number().int().positive(),
    /** The base64 bytes of each version, by its decimal version. */
    keys: schema.record(schema.string().regex(/^[1-9]\d*$/u), schema.base64()),
});
/** The root keys a keychain keeps under one name. */
type Keyset = schema.Infer<typeof Keyset>;

/** One root key version: the key it encrypts under and the HKDF key its secrets derive from, both non-extractable. */
interface RootKey {
    /** The key encrypting under the version. */
    readonly encryption: CryptoKey;
    /** The key the version's secrets derive from. */
    readonly root: CryptoKey;
}

/** A process's one root of custody: versioned root keys that encrypt its keys at rest and derive its other secrets. */
export interface Keyring extends Deriver {
    /** Encrypt bytes under the active root key, binding an encryption context. */
    encrypt(
        plaintext: Uint8Array<ArrayBuffer>,
        context: Uint8Array<ArrayBuffer>,
    ): Promise<Ciphertext>;
    /** Decrypt a ciphertext under the root key version it names. */
    decrypt(
        ciphertext: Ciphertext,
        context: Uint8Array<ArrayBuffer>,
    ): Promise<Uint8Array<ArrayBuffer>>;
    /** Derive a secret under a label from every root key version, newest first, so what an earlier version's secret protects still opens. */
    deriveEach(
        label: string,
    ): Promise<readonly { readonly version: number; readonly secret: Uint8Array<ArrayBuffer> }[]>;
}

/** A keyring over root keys the process holds itself, kept in a keychain or a secret. */
export class LocalKeyring implements Keyring {
    /** The version new ciphertexts are encrypted under. */
    readonly active: number;
    /** Each version's keys. */
    readonly #keys: ReadonlyMap<number, RootKey>;

    /** Keep imported keys under their versions with the active one. */
    private constructor(active: number, keys: ReadonlyMap<number, RootKey>) {
        this.active = active;
        this.#keys = keys;
    }

    /** Write a new keyset as a process's secret holds it, its first root key at version 1. */
    static generate(): string {
        return JSON.stringify(LocalKeyring.#generate());
    }

    /** Open the keyset a process's secret holds. */
    static read(text: string): Promise<LocalKeyring> {
        return LocalKeyring.#from(LocalKeyring.#parse(text));
    }

    /** Open the root keys a keychain keeps under a name, generating the first one when none is kept. */
    static open(keychain: Keychain, name: string): Promise<LocalKeyring> {
        return LocalKeyring.#change(keychain, name, (keys) => keys ?? LocalKeyring.#generate());
    }

    /** Keep a new active root key beside the others. */
    static rotate(keychain: Keychain, name: string): Promise<LocalKeyring> {
        return LocalKeyring.#change(keychain, name, (keys) =>
            LocalKeyring.#generate(LocalKeyring.#required(keys, name)),
        );
    }

    /** Forget a root key version, refusing the active one and one the keychain does not keep. */
    static retire(keychain: Keychain, name: string, version: number): Promise<LocalKeyring> {
        return LocalKeyring.#change(keychain, name, (kept) => {
            // refuse forgetting the active or an unknown key
            const keys = LocalKeyring.#required(kept, name);
            if (keys.active === version) {
                throw new IdentityError("INVALID_ROOT_KEY", "cannot retire the active root key");
            } else if (!Object.hasOwn(keys.keys, String(version))) {
                throw new IdentityError(
                    "INVALID_ROOT_KEY",
                    `no root key ${version} is kept as ${name}`,
                );
            }

            // keep the other keys
            const { [String(version)]: _retired, ...others } = keys.keys;

            return { active: keys.active, keys: others };
        });
    }

    /** Import independently generated 256-bit root keys by version. */
    static async import(
        active: number,
        values: ReadonlyMap<number, Uint8Array<ArrayBuffer>>,
    ): Promise<LocalKeyring> {
        if (!values.has(active)) {
            throw new IdentityError("INVALID_ROOT_KEY", "active root key is missing");
        }

        // validate all versions before accepting the deployment
        const keys = new Map<number, RootKey>();
        for (const [version, value] of values) {
            if (!Number.isSafeInteger(version) || version < 1 || value.length !== ROOT_KEY_BYTES) {
                throw new IdentityError(
                    "INVALID_ROOT_KEY",
                    "root keys require a positive whole version and 32 bytes",
                );
            }

            // derive the version's encryption key apart from the secrets its root derives
            const root = await Derivation.root(value);
            const encryption = await Derivation.encryptionKey(root, ENCRYPTION_LABEL);
            keys.set(version, { encryption, root });
        }

        return new LocalKeyring(active, keys);
    }

    /** Encrypt bytes under the active root key. */
    async encrypt(plaintext: Uint8Array<ArrayBuffer>, context: Uint8Array<ArrayBuffer>) {
        return Ciphertext.encrypt(
            plaintext,
            this.#get(this.active).encryption,
            { alg: "A256GCMKW", kid: String(this.active) },
            context,
        );
    }

    /** Decrypt a ciphertext under the root key version it names. */
    async decrypt(ciphertext: Ciphertext, context: Uint8Array<ArrayBuffer>) {
        return Ciphertext.decrypt(
            ciphertext,
            this.#get(LocalKeyring.version(ciphertext)).encryption,
            context,
        );
    }

    /** Derive a 256-bit secret under a label from the active root key, the same until the root key rotates. */
    async derive(label: string): Promise<Uint8Array<ArrayBuffer>> {
        return this.#derive(this.active, label);
    }

    /** Derive a secret under a label from every root key version, newest first. */
    async deriveEach(label: string) {
        const versions = [...this.#keys.keys()].toSorted((left, right) => right - left);

        return Promise.all(
            versions.map(async (version) => ({
                version,
                secret: await this.#derive(version, label),
            })),
        );
    }

    /** Derive a P-256 private key under a label from the active root key, the same until the root key rotates. */
    async derivePrivateKey(label: string): Promise<JWK> {
        return Derivation.privateKey(this.#get(this.active).root, label);
    }

    /** Read the root key version a ciphertext is encrypted under, refusing one naming none. */
    static version(ciphertext: Ciphertext): number {
        const version = Number(Ciphertext.keyId(ciphertext));
        if (!Number.isSafeInteger(version)) {
            throw new IdentityError("KEY_UNAVAILABLE", "the ciphertext names no root key");
        }

        return version;
    }

    /** Change the root keys a keychain keeps under a name, and open the keys finally kept. */
    static async #change(
        keychain: Keychain,
        name: string,
        change: (keys: Keyset | undefined) => Keyset,
    ): Promise<LocalKeyring> {
        // keep the kept text when the parsed keys are unchanged
        const kept = await Keychain.update(keychain, name, (text) => {
            const keys = text === undefined ? undefined : LocalKeyring.#parse(text);
            const changed = change(keys);

            return changed === keys && text !== undefined ? text : JSON.stringify(changed);
        });

        return LocalKeyring.#from(LocalKeyring.#parse(kept));
    }

    /** Parse a kept keyset. */
    static #parse(text: string): Keyset {
        return Keyset.parse(JSON.parse(text));
    }

    /** Refuse a name the keychain keeps no root keys under. */
    static #required(keys: Keyset | undefined, name: string): Keyset {
        if (keys === undefined) {
            throw new IdentityError("KEY_UNAVAILABLE", `no root keys are kept as ${name}`);
        }

        return keys;
    }

    /** Import a kept keyset. */
    static #from(keys: Keyset): Promise<LocalKeyring> {
        return LocalKeyring.import(
            keys.active,
            new Map(
                Object.entries(keys.keys).map(([version, value]) => [
                    Number(version),
                    Uint8Array.fromBase64(value),
                ]),
            ),
        );
    }

    /** Add a fresh active root key at the next version to a keyset, or start one with it at version 1. */
    static #generate(keys?: Keyset): Keyset {
        const version = Math.max(0, ...Object.keys(keys?.keys ?? {}).map(Number)) + 1;
        const value = crypto.getRandomValues(new Uint8Array(ROOT_KEY_BYTES));

        return { active: version, keys: { ...keys?.keys, [String(version)]: value.toBase64() } };
    }

    /** Derive a 256-bit secret under a label from a root key version. */
    #derive(version: number, label: string): Promise<Uint8Array<ArrayBuffer>> {
        return Derivation.secret(this.#get(version).root, label);
    }

    /** Reject unavailable root key versions explicitly. */
    #get(version: number): RootKey {
        const key = this.#keys.get(version);
        if (key === undefined) {
            throw new IdentityError("KEY_UNAVAILABLE", "required root key is unavailable");
        }

        return key;
    }
}
