import { schema } from "@destack/schema";
import { VaultError } from "../error/index.ts";
import type { Keyring } from "./keyring.ts";

/** The protocol every ciphertext authenticates, independent of package metadata. */
const ENCRYPTION_PROTOCOL = "@destack/vault";
/** The envelope format, which the authenticated context includes. */
const ENCRYPTION_FORMAT = 1;
/** The bytes of a data key, an AES-256 key. */
const DATA_KEY_BYTES = 32;
/** The bytes of an AES-GCM nonce. */
const NONCE_BYTES = 12;

/** The storage identity of one secret version, authenticated with both ciphertexts. */
export interface EncryptionContext {
    /** The administering space. */
    readonly spaceId: string;
    /** The containing vault resource. */
    readonly vaultId: string;
    /** The immutable secret identity. */
    readonly secretId: string;
    /** The immutable value version. */
    readonly version: number;
}

/** The storage identity ciphertexts authenticate. */
export const EncryptionContext = {
    /** Encode the identity in an unambiguous, versioned order. */
    encode(context: EncryptionContext): Uint8Array<ArrayBuffer> {
        return new TextEncoder().encode(
            JSON.stringify([
                ENCRYPTION_PROTOCOL,
                ENCRYPTION_FORMAT,
                context.spaceId,
                context.vaultId,
                context.secretId,
                context.version,
            ]),
        );
    },
};

/** A sealed value: its ciphertext, and its data key wrapped under a keyring. */
export interface Envelope {
    /** The envelope format. */
    readonly format: typeof ENCRYPTION_FORMAT;
    /** The keyring key the data key is wrapped under. */
    readonly keyId: string;
    /** The base64 ciphertext with its authentication tag. */
    readonly ciphertext: string;
    /** The base64 value nonce. */
    readonly nonce: string;
    /** The base64 wrapped data key with its authentication tag. */
    readonly wrappedKey: string;
    /** The base64 key nonce. */
    readonly keyNonce: string;
}

/** Sealed values. */
export const Envelope = {
    /** The stored shape of an envelope. */
    schema: schema.object({
        /** The envelope format. */
        format: schema.literal(ENCRYPTION_FORMAT),
        /** The keyring key the data key is wrapped under. */
        keyId: schema.string().min(1),
        /** The base64 ciphertext with its authentication tag. */
        ciphertext: schema.base64(),
        /** The base64 value nonce. */
        nonce: schema.base64(),
        /** The base64 wrapped data key with its authentication tag. */
        wrappedKey: schema.base64(),
        /** The base64 key nonce. */
        keyNonce: schema.base64(),
    }),

    /** Seal a value under a fresh data key, which a keyring wraps. */
    async seal(
        value: Uint8Array<ArrayBuffer>,
        keyring: Keyring,
        context: EncryptionContext,
    ): Promise<Envelope> {
        // encrypt under a fresh data key, authenticating the storage identity
        const authenticated = EncryptionContext.encode(context);
        const raw = crypto.getRandomValues(new Uint8Array(DATA_KEY_BYTES));
        try {
            const key = await crypto.subtle.importKey("raw", raw, "AES-GCM", false, ["encrypt"]);
            const nonce = crypto.getRandomValues(new Uint8Array(NONCE_BYTES));
            const ciphertext = await crypto.subtle.encrypt(
                { name: "AES-GCM", iv: nonce, additionalData: authenticated },
                key,
                value,
            );

            // wrap the data key beside the ciphertext
            return {
                format: ENCRYPTION_FORMAT,
                ...(await keyring.wrap(raw, authenticated)),
                nonce: nonce.toBase64(),
                ciphertext: new Uint8Array(ciphertext).toBase64(),
            };
        } finally {
            raw.fill(0);
        }
    },

    /** Open an envelope, authenticating its storage identity. */
    async open(
        envelope: Envelope,
        keyring: Keyring,
        context: EncryptionContext,
    ): Promise<Uint8Array<ArrayBuffer>> {
        Envelope.requireFormat(envelope);
        const authenticated = EncryptionContext.encode(context);

        return decrypt(await keyring.unwrap(envelope, authenticated), envelope, authenticated);
    },

    /** Refuse an envelope of another format. */
    requireFormat(envelope: { readonly format: number }): void {
        if (envelope.format !== ENCRYPTION_FORMAT) {
            throw new VaultError("DECRYPTION_FAILED", "unsupported secret encryption format");
        }
    },
};

/** Decrypt a ciphertext with a raw data key, erasing the key either way. */
async function decrypt(
    raw: Uint8Array<ArrayBuffer>,
    envelope: Pick<Envelope, "ciphertext" | "nonce">,
    authenticated: Uint8Array<ArrayBuffer>,
): Promise<Uint8Array<ArrayBuffer>> {
    try {
        const key = await crypto.subtle.importKey("raw", raw, "AES-GCM", false, ["decrypt"]);
        const plaintext = await crypto.subtle.decrypt(
            {
                name: "AES-GCM",
                iv: Uint8Array.fromBase64(envelope.nonce),
                additionalData: authenticated,
            },
            key,
            Uint8Array.fromBase64(envelope.ciphertext),
        );

        return new Uint8Array(plaintext);
    } catch {
        throw new VaultError("DECRYPTION_FAILED", "secret authentication failed");
    } finally {
        raw.fill(0);
    }
}
