import { Ciphertext } from "../ciphertext/ciphertext.ts";

/** The curve of recipient keys and the ephemeral keys sealing to them. */
const CURVE = { name: "ECDH", namedCurve: "P-256" } as const;

/**
 * The target of a move as the providers seal secrets to it: an ECDH P-256 key, fresh for each move.
 *
 * The source holds only the public half and seals, and the target holds both halves and opens.
 * Secrets such as data keys cross between hosts sealed, so their plaintext never leaves a host.
 */
export class Recipient {
    /** The public key sources seal to, as base64 of its uncompressed P-256 point (SEC 1). */
    readonly key: string;
    /** The private key, held only by the recipient itself. */
    readonly #privateKey: CryptoKey | undefined;

    /** Hold a recipient's public key, and its private key on the recipient itself. */
    private constructor(key: string, privateKey: CryptoKey | undefined) {
        this.key = key;
        this.#privateKey = privateKey;
    }

    /** Generate a recipient holding both halves of a fresh key. */
    static async generate(): Promise<Recipient> {
        const pair = await crypto.subtle.generateKey(CURVE, false, ["deriveBits"]);
        const key = new Uint8Array(await crypto.subtle.exportKey("raw", pair.publicKey));

        return new Recipient(key.toBase64(), pair.privateKey);
    }

    /** Name a recipient by its public key, as a source sealing to it does. */
    static of(key: string): Recipient {
        return new Recipient(key, undefined);
    }

    /** Seal bytes to the recipient with a key agreed with a fresh ephemeral key, binding a context both sides name. */
    async seal(
        plaintext: Uint8Array<ArrayBuffer>,
        context: Uint8Array<ArrayBuffer>,
    ): Promise<Ciphertext> {
        const key = await crypto.subtle.importKey(
            "raw",
            Uint8Array.fromBase64(this.key),
            CURVE,
            true,
            [],
        );

        return Ciphertext.encrypt(plaintext, key, { alg: "ECDH-ES" }, context);
    }

    /** Open bytes sealed to the recipient, refusing another context and a recipient named only by its public key. */
    async open(
        ciphertext: Ciphertext,
        context: Uint8Array<ArrayBuffer>,
    ): Promise<Uint8Array<ArrayBuffer>> {
        if (this.#privateKey === undefined) {
            throw new TypeError("open sealed bytes on the recipient holding its private key");
        }

        return Ciphertext.decrypt(ciphertext, this.#privateKey, context);
    }
}
