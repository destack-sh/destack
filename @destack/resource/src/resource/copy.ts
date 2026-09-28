import { defineSchema, schema } from "@destack/schema";
import type { ResourceState } from "@destack/package/declare";
import type { ResourceRecord } from "./provider.ts";

/** The curve of recipient keys and the ephemeral keys sealing to them. */
const CURVE = { name: "ECDH", namedCurve: "P-256" } as const;
/** The label binding keys a seal derives to their use, as HPKE labels its key schedule. */
const SEAL_LABEL = new TextEncoder().encode("destack seal v1");
/** The bytes of an AES-GCM nonce. */
const NONCE_BYTES = 12;

/** One piece of a resource's content on its way from a transfer's source to its target, in its JSON form. */
export const Chunk = defineSchema(
    schema.object({
        /** Where the export continues after this chunk, which only the exporting provider reads. */
        cursor: schema.string().min(1),
        /** The content, in the provider's own JSON form. */
        body: schema.json(),
    }),
);
/** One piece of a resource's content on its way from a transfer's source to its target. */
export type Chunk = schema.Infer<typeof Chunk>;

/** How far a copy goes: live content while the source still serves writes, and all of it once the source is fenced. */
export const CopyStage = defineSchema(schema.enum(["live", "fenced"]));
/** How far a copy goes. */
export type CopyStage = schema.Infer<typeof CopyStage>;

/** Bytes sealed to a recipient: an ephemeral key's agreement with the recipient's key encrypts them. */
export const Sealed = defineSchema(
    schema.object({
        /** The ephemeral P-256 public key, as base64 of its uncompressed point. */
        key: schema.base64(),
        /** The base64 AES-GCM nonce. */
        nonce: schema.base64(),
        /** The base64 ciphertext with its authentication tag. */
        ciphertext: schema.base64(),
    }),
);
/** Bytes sealed to a recipient. */
export type Sealed = schema.Infer<typeof Sealed>;

/**
 * The target of a copy as the providers seal secrets to it: an ECDH P-256 key, fresh for each copy.
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
    ): Promise<Sealed> {
        // agree a key with a fresh ephemeral key
        const ephemeral = (await crypto.subtle.generateKey(CURVE, true, [
            "deriveBits",
        ])) as CryptoKeyPair;
        const sender = new Uint8Array(await crypto.subtle.exportKey("raw", ephemeral.publicKey));
        const key = await agree(ephemeral.privateKey, await publicKey(this.key), sender, this.key);

        // encrypt under it
        const nonce = crypto.getRandomValues(new Uint8Array(NONCE_BYTES));
        const ciphertext = await crypto.subtle.encrypt(
            { name: "AES-GCM", iv: nonce, additionalData: context },
            key,
            plaintext,
        );

        return {
            key: sender.toBase64(),
            nonce: nonce.toBase64(),
            ciphertext: new Uint8Array(ciphertext).toBase64(),
        };
    }

    /** Open bytes sealed to the recipient, refusing another context and a recipient named only by its public key. */
    async open(sealed: Sealed, context: Uint8Array<ArrayBuffer>): Promise<Uint8Array<ArrayBuffer>> {
        // require the private half
        if (this.#privateKey === undefined) {
            throw new TypeError("open sealed bytes on the recipient holding its private key");
        }

        // agree the sender's key, then decrypt under it
        const sender = Uint8Array.fromBase64(sealed.key);
        const key = await agree(this.#privateKey, await publicKey(sealed.key), sender, this.key);
        const plaintext = await crypto.subtle.decrypt(
            { name: "AES-GCM", iv: Uint8Array.fromBase64(sealed.nonce), additionalData: context },
            key,
            Uint8Array.fromBase64(sealed.ciphertext),
        );

        return new Uint8Array(plaintext);
    }
}

/** A resource a transfer copies on one side: its record there, its bindings' desired states, the target sealing secrets, and how far the copy goes. */
export interface Copy {
    /** The resource as this side records it: the source's or the target's provisioning. */
    readonly record: ResourceRecord;
    /** The desired states of the resource's bindings, whose shape the copy follows, such as a database's tables. */
    readonly desired: readonly ResourceState[];
    /** The target, which secrets are sealed to. */
    readonly recipient: Recipient;
    /** How far the copy goes. */
    readonly stage: CopyStage;
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
