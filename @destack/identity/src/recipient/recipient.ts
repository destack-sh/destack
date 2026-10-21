import { Ciphertext } from "../ciphertext/ciphertext.ts";
import type { Deriver } from "../derivation/derivation.ts";
import type { Label } from "../label/label.ts";

/** The curve of recipient keys and the ephemeral keys sealing to them. */
const CURVE = { name: "ECDH", namedCurve: "P-256" } as const;

/** A party with an ECDH P-256 key that others seal bytes to. */
export class Recipient {
    /** The public key sources seal to, as base64 of its uncompressed P-256 point (SEC 1). */
    readonly key: string;
    /** The private key, present only on the recipient itself. */
    readonly #privateKey: CryptoKey | undefined;

    /** Create a recipient from its public key, and its private key on the recipient itself. */
    private constructor(key: string, privateKey: CryptoKey | undefined) {
        this.key = key;
        this.#privateKey = privateKey;
    }

    /** Generate a recipient with both halves of a fresh key. */
    static async generate(): Promise<Recipient> {
        const pair = await crypto.subtle.generateKey(CURVE, false, ["deriveBits"]);
        const key = new Uint8Array(await crypto.subtle.exportKey("raw", pair.publicKey));

        return new Recipient(key.toBase64(), pair.privateKey);
    }

    /** Derive the recipient of an identity's root secret under a label. */
    static async derive(
        deriver: Pick<Deriver, "derivePrivateKey">,
        label: Label,
    ): Promise<Recipient> {
        // import the derived scalar as an ECDH key and read its public point
        const jwk = await deriver.derivePrivateKey(label);
        const { key_ops: _operations, ext: _extractable, alg: _algorithm, ...ecdh } = jwk;
        const privateKey = await crypto.subtle.importKey("jwk", ecdh, CURVE, true, ["deriveBits"]);
        const pair = await crypto.subtle.exportKey("jwk", privateKey);
        const { d: _scalar, key_ops: _uses, ...point } = pair;
        const publicKey = await crypto.subtle.importKey("jwk", point, CURVE, true, []);
        const key = new Uint8Array(await crypto.subtle.exportKey("raw", publicKey));

        return new Recipient(key.toBase64(), privateKey);
    }

    /** Name a recipient by its public key, as a source sealing to it does. */
    static of(key: string): Recipient {
        return new Recipient(key, undefined);
    }

    /** Seal bytes to the recipient, binding a context both sides give. */
    async seal(plaintext: Uint8Array<ArrayBuffer>, context: Label): Promise<Ciphertext> {
        const key = await crypto.subtle.importKey(
            "raw",
            Uint8Array.fromBase64(this.key),
            CURVE,
            true,
            [],
        );

        return Ciphertext.encrypt(plaintext, key, { alg: "ECDH-ES+A256KW" }, context);
    }

    /** Open bytes sealed to the recipient. */
    async open(ciphertext: Ciphertext, context: Label): Promise<Uint8Array<ArrayBuffer>> {
        if (this.#privateKey === undefined) {
            throw new TypeError("open sealed bytes on the recipient with its private key");
        }

        return Ciphertext.decrypt(ciphertext, this.#privateKey, context);
    }
}
