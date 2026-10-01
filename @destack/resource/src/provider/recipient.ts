/** The curve of recipient keys and the ephemeral keys sealing to them. */
const CURVE = { name: "ECDH", namedCurve: "P-256" } as const;
/** The label binding keys a seal derives to their use, as HPKE labels its key schedule. */
const SEAL_LABEL = new TextEncoder().encode("destack seal v1");
/** The bytes of an AES-GCM nonce. */
const NONCE_BYTES = 12;

/** Bytes encrypted to a recipient under its key's agreement with a fresh ephemeral key, as HPKE seals them. */
export interface Ciphertext {
    /** The ephemeral P-256 public key, as base64 of its uncompressed point: HPKE's encapsulated key. */
    readonly encapsulated: string;
    /** The base64 AES-GCM nonce. */
    readonly nonce: string;
    /** The base64 encrypted bytes with their authentication tag. */
    readonly bytes: string;
}

/**
 * The target of a move as the providers seal secrets to it: an ECDH P-256 key, fresh for each move.
 *
 * The source holds only the public half and seals, and the target holds both halves and opens.
 * Secrets such as data keys cross between hosts sealed, so their plaintext never leaves a host.
 */
export class Recipient {
    /** The public key, as base64 of its uncompressed point. */
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
        const pair = (await crypto.subtle.generateKey(CURVE, false, [
            "deriveBits",
        ])) as CryptoKeyPair;
        const key = new Uint8Array(await crypto.subtle.exportKey("raw", pair.publicKey));

        return new Recipient(key.toBase64(), pair.privateKey);
    }

    /** Name a recipient by its public key, as a source sealing to it does. */
    static of(key: string): Recipient {
        return new Recipient(key, undefined);
    }

    /** Seal bytes to the recipient, authenticating a context both sides name. */
    async seal(
        plaintext: Uint8Array<ArrayBuffer>,
        context: Uint8Array<ArrayBuffer>,
    ): Promise<Ciphertext> {
        // agree a key with a fresh ephemeral key
        const ephemeral = (await crypto.subtle.generateKey(CURVE, true, [
            "deriveBits",
        ])) as CryptoKeyPair;
        const sender = new Uint8Array(await crypto.subtle.exportKey("raw", ephemeral.publicKey));
        const key = await agree(ephemeral.privateKey, await publicKey(this.key), sender, this.key);

        // encrypt under it
        const nonce = crypto.getRandomValues(new Uint8Array(NONCE_BYTES));
        const encrypted = await crypto.subtle.encrypt(
            { name: "AES-GCM", iv: nonce, additionalData: context },
            key,
            plaintext,
        );

        return {
            encapsulated: sender.toBase64(),
            nonce: nonce.toBase64(),
            bytes: new Uint8Array(encrypted).toBase64(),
        };
    }

    /** Open bytes sealed to the recipient, refusing another context and a recipient named only by its public key. */
    async open(
        ciphertext: Ciphertext,
        context: Uint8Array<ArrayBuffer>,
    ): Promise<Uint8Array<ArrayBuffer>> {
        // require the private half
        if (this.#privateKey === undefined) {
            throw new TypeError("open sealed bytes on the recipient holding its private key");
        }

        // agree the sender's key, then decrypt under it
        const sender = Uint8Array.fromBase64(ciphertext.encapsulated);
        const key = await agree(
            this.#privateKey,
            await publicKey(ciphertext.encapsulated),
            sender,
            this.key,
        );
        const plaintext = await crypto.subtle.decrypt(
            {
                name: "AES-GCM",
                iv: Uint8Array.fromBase64(ciphertext.nonce),
                additionalData: context,
            },
            key,
            Uint8Array.fromBase64(ciphertext.bytes),
        );

        return new Uint8Array(plaintext);
    }
}

/** Import a raw P-256 public key for key agreement. */
function publicKey(key: string): Promise<CryptoKey> {
    return crypto.subtle.importKey("raw", Uint8Array.fromBase64(key), CURVE, false, []);
}

/** Agree an AES-GCM key between a private key and a public key, bound to both public keys as HPKE binds its key schedule. */
async function agree(
    privateKey: CryptoKey,
    otherKey: CryptoKey,
    sender: Uint8Array<ArrayBuffer>,
    recipient: string,
): Promise<CryptoKey> {
    // derive the shared secret
    const secret = await crypto.subtle.deriveBits(
        { name: "ECDH", public: otherKey },
        privateKey,
        256,
    );
    const material = await crypto.subtle.importKey("raw", secret, "HKDF", false, ["deriveKey"]);

    // expand it under the label and both public keys
    const info = new Uint8Array([...SEAL_LABEL, ...sender, ...Uint8Array.fromBase64(recipient)]);

    return crypto.subtle.deriveKey(
        { name: "HKDF", hash: "SHA-256", salt: new Uint8Array(), info },
        material,
        { name: "AES-GCM", length: 256 },
        false,
        ["encrypt", "decrypt"],
    );
}
