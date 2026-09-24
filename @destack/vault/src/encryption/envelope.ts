import { VaultError } from "../error/index.ts";
import type { Keyring } from "./keyring.ts";
import { schema } from "@destack/schema";

/** Fixed encryption protocol identifier, independent of package metadata. */
const ENCRYPTION_PROTOCOL = "@destack/vault";
/** Authenticated context and ciphertext format version. */
const ENCRYPTION_FORMAT = 1;

/** Authenticated ciphertext and the protected key needed to decrypt it. */
export const SecretEnvelope = schema.object({
    /** Encoding and algorithm version. */
    format: schema.literal(ENCRYPTION_FORMAT),
    /** Root key version used to protect the data key. */
    keyId: schema.string().min(1),
    /** Base64 ciphertext including its authentication tag. */
    ciphertext: schema.base64(),
    /** Base64 value nonce. */
    nonce: schema.base64(),
    /** Base64 encrypted data key including its authentication tag. */
    wrappedKey: schema.base64(),
    /** Base64 key nonce. */
    keyNonce: schema.base64().optional(),
});
/** Authenticated ciphertext and its protected data key. */
export type SecretEnvelope = schema.Infer<typeof SecretEnvelope>;

/** Tenant-qualified identity authenticated with both ciphertexts. */
export type EncryptionContext = {
    /** Storage region or local host, assigned by the server. */
    location: string;
    /** Administering space. */
    spaceId: string;
} & (
    | {
          /** Secret value protected by this envelope. */
          kind: "secret";
          /** Containing vault resource. */
          vaultId: string;
          /** Immutable secret identity. */
          secretId: string;
          /** Immutable value version. */
          version: number;
      }
    | {
          /** Mutation fingerprint protected by this envelope. */
          kind: "request";
          /** Authenticated mutation initiator. */
          caller: string;
          /** Declared mutation procedure. */
          procedure: string;
          /** Immutable mutation identity. */
          requestId: string;
      }
);

/** Encrypt immutable secret values and rewrap their data keys. */
export class EnvelopeEncryption {
    /** Trusted root-key implementation. */
    readonly keys: Keyring;

    /** Bind root-key access without retaining plaintext secret values. */
    constructor(keys: Keyring) {
        this.keys = keys;
    }

    /** Encrypt a value with a fresh key and nonce. */
    async encrypt(
        value: Uint8Array<ArrayBuffer>,
        context: EncryptionContext,
    ): Promise<SecretEnvelope> {
        // authenticate the exact storage identity and format
        const authenticated = encodeEncryptionContext(context);
        const raw = crypto.getRandomValues(new Uint8Array(32));
        try {
            const key = await crypto.subtle.importKey("raw", raw, "AES-GCM", false, ["encrypt"]);
            const nonce = crypto.getRandomValues(new Uint8Array(12));
            const ciphertext = await crypto.subtle.encrypt(
                { name: "AES-GCM", iv: nonce, additionalData: authenticated },
                key,
                value,
            );
            const protectedKey = await this.keys.wrap(raw, authenticated);

            return {
                format: ENCRYPTION_FORMAT,
                ...protectedKey,
                nonce: nonce.toBase64(),
                ciphertext: new Uint8Array(ciphertext).toBase64(),
            };
        } finally {
            raw.fill(0);
        }
    }

    /** Verify the stored identity before releasing plaintext. */
    async decrypt(
        envelope: SecretEnvelope,
        context: EncryptionContext,
    ): Promise<Uint8Array<ArrayBuffer>> {
        if (envelope.format !== ENCRYPTION_FORMAT) {
            throw new VaultError("DECRYPTION_FAILED", "unsupported secret encryption format");
        }
        const authenticated = encodeEncryptionContext(context);
        const raw = await this.keys.unwrap(envelope, authenticated);
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

    /** Change root-key protection without changing the secret version or value. */
    async rewrap(envelope: SecretEnvelope, context: EncryptionContext): Promise<SecretEnvelope> {
        if (envelope.format !== ENCRYPTION_FORMAT) {
            throw new VaultError("DECRYPTION_FAILED", "unsupported secret encryption format");
        }
        const authenticated = encodeEncryptionContext(context);
        const raw = await this.keys.unwrap(envelope, authenticated);
        try {
            const protectedKey = await this.keys.wrap(raw, authenticated);

            return { ...envelope, ...protectedKey, keyNonce: protectedKey.keyNonce };
        } finally {
            raw.fill(0);
        }
    }
}

/** Encode the immutable identity in an unambiguous, versioned order. */
function encodeEncryptionContext(context: EncryptionContext): Uint8Array<ArrayBuffer> {
    const selection =
        context.kind === "secret"
            ? [context.vaultId, context.secretId, context.version]
            : [context.caller, context.procedure, context.requestId];

    return new TextEncoder().encode(
        JSON.stringify([
            ENCRYPTION_PROTOCOL,
            ENCRYPTION_FORMAT,
            context.location,
            context.spaceId,
            context.kind,
            ...selection,
        ]),
    );
}
