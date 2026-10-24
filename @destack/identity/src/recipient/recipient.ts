import type { Environment } from "@destack/environment";
import { Ciphertext } from "../ciphertext/ciphertext.ts";
import type { Deriver } from "../derivation/derivation.ts";
import { PublicKey } from "../key/key.ts";
import type { Label } from "../label/label.ts";

/** The curve of recipient keys and the ephemeral keys sealing to them. */
const CURVE = { name: "ECDH", namedCurve: "P-256" } as const;

/** A party with an ECDH P-256 key that others seal bytes to. */
export class Recipient {
    /** The public key sources seal to, as base64 of its uncompressed P-256 point (SEC 1). */
    readonly key: string;
    /** The private key, present only on the recipient itself. */
    readonly #privateKey: CryptoKey | undefined;
    /** The WebCrypto and randomness the recipient seals and opens with. */
    readonly #environment: Pick<Environment, "crypto" | "random">;

    /** Create a recipient from its public key, and its private key on the recipient itself. */
    private constructor(
        key: string,
        privateKey: CryptoKey | undefined,
        environment: Pick<Environment, "crypto" | "random">,
    ) {
        this.key = key;
        this.#privateKey = privateKey;
        this.#environment = environment;
    }

    /** Generate a recipient with both halves of a fresh key. */
    static async generate(environment: Pick<Environment, "crypto" | "random">): Promise<Recipient> {
        // generate a pair, and publish its public point
        const { crypto } = environment;
        const pair = await crypto.generateKey(CURVE, false, ["deriveBits"]);
        const key = new Uint8Array(await crypto.exportKey("raw", pair.publicKey));

        return new Recipient(key.toBase64(), pair.privateKey, environment);
    }

    /** Derive the recipient of an identity's root secret under a label. */
    static async derive(
        deriver: Pick<Deriver, "derivePrivateKey">,
        label: Label,
        environment: Pick<Environment, "crypto" | "random">,
    ): Promise<Recipient> {
        // import the derived scalar as an ECDH key, and publish its point
        const { kty, crv, x, y, ["d"]: scalar } = await deriver.derivePrivateKey(label);
        const privateKey = await environment.crypto.importKey(
            "jwk",
            { kty, crv, x, y, ["d"]: scalar },
            CURVE,
            false,
            ["deriveBits"],
        );
        const key = PublicKey.raw({ kty, crv, x, y });

        return new Recipient(key.toBase64(), privateKey, environment);
    }

    /** Read a recipient from its public key, as a source sealing to it does. */
    static of(key: string, environment: Pick<Environment, "crypto" | "random">): Recipient {
        return new Recipient(key, undefined, environment);
    }

    /** Seal bytes to the recipient, binding a context both sides give. */
    async seal(plaintext: Uint8Array<ArrayBuffer>, context: Label): Promise<Ciphertext> {
        const key = await this.#environment.crypto.importKey(
            "raw",
            Uint8Array.fromBase64(this.key),
            CURVE,
            true,
            [],
        );

        return Ciphertext.encrypt(
            plaintext,
            key,
            { alg: "ECDH-ES+A256KW" },
            context,
            this.#environment,
        );
    }

    /** Open bytes sealed to the recipient. */
    async open(ciphertext: Ciphertext, context: Label): Promise<Uint8Array<ArrayBuffer>> {
        if (this.#privateKey === undefined) {
            throw new TypeError("the recipient has no private key to decrypt with");
        }

        return Ciphertext.decrypt(ciphertext, this.#privateKey, context, this.#environment.crypto);
    }
}
