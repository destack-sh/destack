import type { Environment } from "@destack/environment";
import { schema } from "@destack/schema";
import { Ciphertext } from "../ciphertext/ciphertext.ts";
import { Derivation, type Deriver } from "../derivation/derivation.ts";
import { IdentityError } from "../error/error.ts";
import type { PrivateKey } from "../key/key.ts";
import { Keychain } from "../keychain/keychain.ts";
import type { Label } from "../label/label.ts";

/** The bytes of a root key, an AES-256 key. */
const ROOT_KEY_BYTES = 32;

/** A root key version as text: a positive integer without leading zeros. */
const VERSION = /^[1-9]\d*$/u;

/** The label of the key a root key encrypts under, apart from every secret it derives. */
const ENCRYPTION_LABEL = { label: "@destack/identity/keyring-encryption", context: "" };

/** The root keys a keychain stores under one name: each version's bytes, and the active version. */
const Keyset = schema.object({
    /** The version new ciphertexts are encrypted under. */
    active: schema.number().int().positive(),
    /** The base64 bytes of each version, by its decimal version. */
    keys: schema.record(schema.string().regex(VERSION), schema.base64()),
});
/** The root keys a keychain stores under one name. */
type Keyset = schema.Infer<typeof Keyset>;

/** The WebCrypto and randomness a keyring encrypts and derives with. */
export type KeyringEnvironment = Pick<Environment, "crypto" | "random">;

/** One root key version: the key it encrypts under and the HKDF key its secrets derive from. */
interface RootKey {
    /** The key encrypting under the version. */
    readonly encryption: CryptoKey;
    /** The key the version's secrets derive from. */
    readonly root: CryptoKey;
}

/** A process's one root of custody. */
export interface Keyring extends Deriver {
    /** Encrypt bytes under the active root key, binding an encryption context. */
    readonly encrypt: (plaintext: Uint8Array<ArrayBuffer>, context: Label) => Promise<Ciphertext>;
    /** Decrypt a ciphertext under the root key version it names. */
    readonly decrypt: (ciphertext: Ciphertext, context: Label) => Promise<Uint8Array<ArrayBuffer>>;
    /** Derive a secret under a label from every root key version, newest first. */
    readonly deriveEach: (
        label: Label,
    ) => Promise<readonly { readonly version: number; readonly secret: Uint8Array<ArrayBuffer> }[]>;
}

/** A keyring over root keys in the process itself, stored in a keychain or a secret. */
export class LocalKeyring implements Keyring {
    /** The version new ciphertexts are encrypted under. */
    readonly active: number;
    /** Each version's keys. */
    readonly #keys: ReadonlyMap<number, RootKey>;
    /** The WebCrypto and randomness the keyring encrypts and derives with. */
    readonly #environment: KeyringEnvironment;

    /** Store imported keys under their versions with the active one. */
    private constructor(
        active: number,
        keys: ReadonlyMap<number, RootKey>,
        environment: KeyringEnvironment,
    ) {
        this.active = active;
        this.#keys = keys;
        this.#environment = environment;
    }

    /** Write a new keyset as a process's secret stores it, its first root key at version 1. */
    static generate(environment: KeyringEnvironment): string {
        return JSON.stringify(LocalKeyring.#generate(environment));
    }

    /** Open the keyset a process's secret stores. */
    static read(text: string, environment: KeyringEnvironment): Promise<LocalKeyring> {
        return LocalKeyring.#from(LocalKeyring.#parse(text), environment);
    }

    /** Open the root keys a keychain stores under a name. */
    static open(
        keychain: Keychain,
        name: string,
        environment: KeyringEnvironment,
    ): Promise<LocalKeyring> {
        return LocalKeyring.#change(
            keychain,
            name,
            (keys) => keys ?? LocalKeyring.#generate(environment),
            environment,
        );
    }

    /** Store a new active root key beside the others. */
    static rotate(
        keychain: Keychain,
        name: string,
        environment: KeyringEnvironment,
    ): Promise<LocalKeyring> {
        return LocalKeyring.#change(
            keychain,
            name,
            (keys) => LocalKeyring.#generate(environment, LocalKeyring.#required(keys, name)),
            environment,
        );
    }

    /** Forget a root key version, refusing the active one and one the keychain does not store. */
    static retire(
        keychain: Keychain,
        name: string,
        version: number,
        environment: KeyringEnvironment,
    ): Promise<LocalKeyring> {
        return LocalKeyring.#change(
            keychain,
            name,
            (stored) => {
                // refuse forgetting the active or an unknown key
                const keys = LocalKeyring.#required(stored, name);
                if (keys.active === version) {
                    throw IdentityError.of("ROOT_KEY_INVALID", "cannot retire the active root key");
                } else if (!Object.hasOwn(keys.keys, String(version))) {
                    throw IdentityError.of(
                        "ROOT_KEY_INVALID",
                        `root key ${version} not found in ${name}`,
                    );
                }

                // drop the retired key
                const { [String(version)]: _retired, ...others } = keys.keys;

                return { active: keys.active, keys: others };
            },
            environment,
        );
    }

    /** Import independently generated 256-bit root keys by version. */
    static async import(
        active: number,
        values: ReadonlyMap<number, Uint8Array<ArrayBuffer>>,
        environment: KeyringEnvironment,
    ): Promise<LocalKeyring> {
        if (!values.has(active)) {
            throw IdentityError.of("ROOT_KEY_INVALID", "active root key is missing");
        }

        // validate all versions before accepting the deployment
        const keys = new Map<number, RootKey>();
        for (const [version, value] of values) {
            if (!Number.isSafeInteger(version) || version < 1 || value.length !== ROOT_KEY_BYTES) {
                throw IdentityError.of(
                    "ROOT_KEY_INVALID",
                    "root keys require a positive whole version and 32 bytes",
                );
            }

            // derive the version's encryption key apart from the secrets its root derives
            const root = await Derivation.root(value, environment.crypto);
            const encryption = await Derivation.encryptionKey(
                root,
                ENCRYPTION_LABEL,
                environment.crypto,
            );
            keys.set(version, { encryption, root });
        }

        return new LocalKeyring(active, keys, environment);
    }

    /** Encrypt bytes under the active root key. */
    async encrypt(plaintext: Uint8Array<ArrayBuffer>, context: Label) {
        return Ciphertext.encrypt(
            plaintext,
            this.#get(this.active).encryption,
            { alg: "A256GCMKW", kid: String(this.active) },
            context,
            this.#environment,
        );
    }

    /** Decrypt a ciphertext under the root key version it names. */
    async decrypt(ciphertext: Ciphertext, context: Label) {
        return Ciphertext.decrypt(
            ciphertext,
            this.#get(LocalKeyring.version(ciphertext)).encryption,
            context,
            this.#environment.crypto,
        );
    }

    /** Derive a 256-bit secret under a label from the active root key. */
    async derive(label: Label): Promise<Uint8Array<ArrayBuffer>> {
        return this.#derive(this.active, label);
    }

    /** Write the keyset a label derives from every root key version, as secrets store it. */
    async deriveKeyset(label: Label): Promise<string> {
        const derived = await this.deriveEach(label);
        const keys = Object.fromEntries(
            derived.map(({ version, secret }) => [String(version), secret.toBase64()]),
        );

        return JSON.stringify(Keyset.parse({ active: this.active, keys }));
    }

    /** Derive a secret under a label from every root key version, newest first. */
    async deriveEach(label: Label) {
        const versions = [...this.#keys.keys()].toSorted((left, right) => right - left);

        return Promise.all(
            versions.map(async (version) => ({
                version,
                secret: await this.#derive(version, label),
            })),
        );
    }

    /** Derive a P-256 private key under a label from the active root key. */
    async derivePrivateKey(label: Label): Promise<PrivateKey> {
        return Derivation.privateKey(this.#get(this.active).root, label, this.#environment.crypto);
    }

    /** Read the root key version a ciphertext is encrypted under, refusing one naming none. */
    static version(ciphertext: Ciphertext): number {
        const keyId = Ciphertext.keyId(ciphertext);
        if (keyId === undefined || !VERSION.test(keyId)) {
            throw IdentityError.of(
                "CIPHERTEXT_MALFORMED",
                "the ciphertext does not reference a root key version",
            );
        }

        return Number(keyId);
    }

    /** Change the root keys a keychain stores under a name, and open the keys finally stored. */
    static async #change(
        keychain: Keychain,
        name: string,
        change: (keys: Keyset | undefined) => Keyset,
        environment: KeyringEnvironment,
    ): Promise<LocalKeyring> {
        // leave the text unchanged when the parsed keys are unchanged
        const secret = await Keychain.update(keychain, name, (text) => {
            const keys = text === undefined ? undefined : LocalKeyring.#parse(text);
            const changed = change(keys);

            return changed === keys && text !== undefined ? text : JSON.stringify(changed);
        });

        return LocalKeyring.#from(LocalKeyring.#parse(secret), environment);
    }

    /** Parse a stored keyset. */
    static #parse(text: string): Keyset {
        return Keyset.parse(JSON.parse(text));
    }

    /** Refuse a name the keychain stores no root keys under. */
    static #required(keys: Keyset | undefined, name: string): Keyset {
        if (keys === undefined) {
            throw IdentityError.of("KEY_UNAVAILABLE", `no root keys found in ${name}`);
        }

        return keys;
    }

    /** Import a stored keyset. */
    static #from(keys: Keyset, environment: KeyringEnvironment): Promise<LocalKeyring> {
        return LocalKeyring.import(
            keys.active,
            new Map(
                Object.entries(keys.keys).map(([version, value]) => [
                    Number(version),
                    Uint8Array.fromBase64(value),
                ]),
            ),
            environment,
        );
    }

    /** Add a fresh active root key at the next version to a keyset, or start a keyset with one. */
    static #generate(environment: KeyringEnvironment, keys?: Keyset): Keyset {
        const version = Math.max(0, ...Object.keys(keys?.keys ?? {}).map(Number)) + 1;
        const value = environment.random.bytes(ROOT_KEY_BYTES);

        return { active: version, keys: { ...keys?.keys, [String(version)]: value.toBase64() } };
    }

    /** Derive a 256-bit secret under a label from a root key version. */
    #derive(version: number, label: Label): Promise<Uint8Array<ArrayBuffer>> {
        return Derivation.secret(this.#get(version).root, label, this.#environment.crypto);
    }

    /** Reject unavailable root key versions explicitly. */
    #get(version: number): RootKey {
        const key = this.#keys.get(version);
        if (key === undefined) {
            throw IdentityError.of("KEY_UNAVAILABLE", "required root key is unavailable");
        }

        return key;
    }
}
