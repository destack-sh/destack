import { eq, type DatabaseConnection } from "@destack/db";
import type { Keychain } from "@destack/host/keychain";
import { ServiceError } from "@destack/service/error";
import { VaultError } from "../error/index.ts";
import { vaultKey } from "../stack/db.ts";
import type { Envelope } from "./envelope.ts";
import { type Keyring, LocalKeyring } from "./keyring.ts";

/** The protocol the vault key's wrapping authenticates, beside its location and vault. */
const VAULT_KEY_PROTOCOL = "@destack/vault/key";

/** The bytes of a vault key, an AES-256 key. */
const VAULT_KEY_BYTES = 32;

/** The bytes of an AES-GCM nonce. */
const NONCE_BYTES = 12;

/** A vault key as its table stores it: its identifier, and its bytes wrapped under a root key. */
export interface WrappedVaultKey {
    /** The key's identifier, which each data key it wraps records. */
    readonly id: string;
    /** The root key it is wrapped under. */
    readonly rootKeyId: string;
    /** The wrapped key, encoded as base64. */
    readonly wrappedKey: string;
    /** The wrapping nonce, encoded as base64. */
    readonly keyNonce: string;
}

/** The key of one vault, wrapped under the host's root keys. */
export class VaultKey implements Keyring {
    /** The key's identifier. */
    readonly id: string;
    /** The host the key's values are stored at, which every ciphertext authenticates. */
    readonly location: string;
    /** The vault. */
    readonly vaultId: string;
    /** The imported, non-extractable key. */
    readonly #key: CryptoKey;

    /** Keep an imported vault key. */
    private constructor(id: string, location: string, vaultId: string, key: CryptoKey) {
        // keep the key with the vault and location it belongs to
        this.id = id;
        this.location = location;
        this.vaultId = vaultId;
        this.#key = key;
    }

    /** Keep a new vault's key, doing nothing for a vault that has one. */
    static async provision(
        database: DatabaseConnection,
        root: Keyring,
        location: string,
        vaultId: string,
    ): Promise<void> {
        const [kept] = await database
            .select({ id: vaultKey.id })
            .from(vaultKey)
            .where(eq(vaultKey.vaultId, vaultId as never));
        if (kept === undefined) {
            const generated = await VaultKey.generate(root, location, vaultId);
            await database.insert(vaultKey).values({ vaultId: vaultId as never, ...generated });
        }
    }

    /** Delete a vault's key, leaving every value it wrapped unreadable. */
    static async discard(database: DatabaseConnection, vaultId: string): Promise<void> {
        await database.delete(vaultKey).where(eq(vaultKey.vaultId, vaultId as never));
    }

    /** Load a vault's key with the root keys, refusing a vault without one. */
    static async load(
        database: DatabaseConnection,
        root: Keyring,
        location: string,
        vaultId: string,
    ): Promise<VaultKey> {
        const [kept] = await database
            .select()
            .from(vaultKey)
            .where(eq(vaultKey.vaultId, vaultId as never));
        if (kept === undefined) {
            throw new ServiceError("INTERNAL_SERVER_ERROR", {
                message: `vault ${vaultId} has no key: provision it first`,
            });
        }

        return VaultKey.open(root, location, vaultId, kept);
    }

    /** Wrap every vault key wrapped under a root key under the active one instead, answering how many. */
    static async rewrapAll(
        database: DatabaseConnection,
        root: Keyring,
        location: string,
        rootKeyId: string,
    ): Promise<number> {
        const wrapped = await database
            .select()
            .from(vaultKey)
            .where(eq(vaultKey.rootKeyId, rootKeyId));
        for (const row of wrapped) {
            const rewrapped = await VaultKey.rewrap(root, location, row.vaultId, row);
            await database.update(vaultKey).set(rewrapped).where(eq(vaultKey.vaultId, row.vaultId));
        }

        return wrapped.length;
    }

    /** Forget a root key a keychain keeps, refusing one that vault keys are still wrapped under. */
    static async retire(
        database: DatabaseConnection,
        keychain: Keychain,
        name: string,
        keyId: string,
    ): Promise<LocalKeyring> {
        // refuse a key vault keys still need
        const [wrapped] = await database
            .select({ vaultId: vaultKey.vaultId })
            .from(vaultKey)
            .where(eq(vaultKey.rootKeyId, keyId))
            .limit(1);
        if (wrapped !== undefined) {
            throw new ServiceError("CONFLICT", {
                message: `vault keys are still wrapped under root key ${keyId}: rewrap them first`,
            });
        }

        return LocalKeyring.retire(keychain, name, keyId);
    }

    /** Generate a vault's key and wrap it under the root keyring's active key. */
    static async generate(
        root: Keyring,
        location: string,
        vaultId: string,
    ): Promise<WrappedVaultKey> {
        const raw = crypto.getRandomValues(new Uint8Array(VAULT_KEY_BYTES));
        try {
            const wrapped = await root.wrap(raw, authenticated(location, vaultId));

            return {
                id: crypto.randomUUID(),
                rootKeyId: wrapped.keyId,
                wrappedKey: wrapped.wrappedKey,
                keyNonce: wrapped.keyNonce,
            };
        } finally {
            raw.fill(0);
        }
    }

    /** Open a vault's wrapped key with the root keyring. */
    static async open(
        root: Keyring,
        location: string,
        vaultId: string,
        wrapped: WrappedVaultKey,
    ): Promise<VaultKey> {
        const raw = await root.unwrap(
            {
                keyId: wrapped.rootKeyId,
                wrappedKey: wrapped.wrappedKey,
                keyNonce: wrapped.keyNonce,
            },
            authenticated(location, vaultId),
        );
        try {
            const key = await crypto.subtle.importKey("raw", raw, "AES-GCM", false, [
                "encrypt",
                "decrypt",
            ]);

            return new VaultKey(wrapped.id, location, vaultId, key);
        } finally {
            raw.fill(0);
        }
    }

    /** Wrap a vault's key under the root keyring's active key instead of the one it is wrapped under. */
    static async rewrap(
        root: Keyring,
        location: string,
        vaultId: string,
        wrapped: WrappedVaultKey,
    ): Promise<WrappedVaultKey> {
        const context = authenticated(location, vaultId);
        const raw = await root.unwrap(
            {
                keyId: wrapped.rootKeyId,
                wrappedKey: wrapped.wrappedKey,
                keyNonce: wrapped.keyNonce,
            },
            context,
        );
        try {
            const rewrapped = await root.wrap(raw, context);

            return {
                id: wrapped.id,
                rootKeyId: rewrapped.keyId,
                wrappedKey: rewrapped.wrappedKey,
                keyNonce: rewrapped.keyNonce,
            };
        } finally {
            raw.fill(0);
        }
    }

    /** Wrap a data key under the vault key. */
    async wrap(
        value: Uint8Array<ArrayBuffer>,
        context: Uint8Array<ArrayBuffer>,
    ): Promise<Pick<Envelope, "keyId" | "wrappedKey" | "keyNonce">> {
        const nonce = crypto.getRandomValues(new Uint8Array(NONCE_BYTES));
        const wrapped = await crypto.subtle.encrypt(
            { name: "AES-GCM", iv: nonce, additionalData: context },
            this.#key,
            value,
        );

        return {
            keyId: this.id,
            keyNonce: nonce.toBase64(),
            wrappedKey: new Uint8Array(wrapped).toBase64(),
        };
    }

    /** Unwrap a data key wrapped under the vault key, refusing one wrapped under another key. */
    async unwrap(
        envelope: Pick<Envelope, "keyId" | "wrappedKey" | "keyNonce">,
        context: Uint8Array<ArrayBuffer>,
    ): Promise<Uint8Array<ArrayBuffer>> {
        // require the vault's own key
        if (envelope.keyId !== this.id) {
            throw new VaultError("KEY_UNAVAILABLE", "the value is wrapped under another vault key");
        }

        // authenticate and unwrap the data key
        try {
            const value = await crypto.subtle.decrypt(
                {
                    name: "AES-GCM",
                    iv: Uint8Array.fromBase64(envelope.keyNonce),
                    additionalData: context,
                },
                this.#key,
                Uint8Array.fromBase64(envelope.wrappedKey),
            );

            return new Uint8Array(value);
        } catch {
            throw new VaultError("DECRYPTION_FAILED", "data key authentication failed");
        }
    }
}

/** Encode what a vault key's wrapping authenticates: the protocol, the location and the vault. */
function authenticated(location: string, vaultId: string): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(JSON.stringify([VAULT_KEY_PROTOCOL, location, vaultId]));
}
