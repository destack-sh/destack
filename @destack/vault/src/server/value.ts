import type { secret } from "../object/index.ts";
import type { InstanceOf } from "@destack/object";
import { and, eq, type DatabaseConnection, type Select } from "@destack/db";
import type { Recipient, Sealed } from "@destack/resource";
import { ServiceError } from "@destack/service/error";
import { type EncryptionContext, Envelope, type VaultKey } from "../encryption/index.ts";
import { MAX_VALUE_BYTES, SecretValue } from "../secret/index.ts";
import { vaultValue } from "../stack/index.ts";

/** A stored value row. */
type ValueRow = Select<typeof vaultValue>;

/** The identity a value's ciphertext authenticates besides its location: its space, vault, secret and version. */
type ValueIdentity = Omit<EncryptionContext, "location">;

/** A value sent to a transfer's target, its data key sealed to the target. */
export interface ValueTransfer {
    /** The envelope format. */
    readonly format: number;
    /** The location the ciphertext authenticates. */
    readonly location: string;
    /** The base64 value ciphertext. */
    readonly ciphertext: string;
    /** The base64 value nonce. */
    readonly nonce: string;
    /** The data key, sealed to the target. */
    readonly key: Sealed;
}

/** The sealed values of secret versions. */
export const VaultValue = {
    /** Seal one version's value under its vault's key and store it. */
    async write(
        database: DatabaseConnection,
        key: VaultKey,
        owner: InstanceOf<typeof secret>,
        version: number,
        value: SecretValue,
    ): Promise<void> {
        // bound the decoded bytes before sealing the transport representation
        const bytes =
            value.encoding === "text"
                ? new TextEncoder().encode(value.value)
                : Uint8Array.fromBase64(value.value);
        const size = bytes.byteLength;
        bytes.fill(0);
        if (size > MAX_VALUE_BYTES) {
            throw new ServiceError("PAYLOAD_TOO_LARGE", {
                message: `secret value exceeds ${MAX_VALUE_BYTES} bytes`,
            });
        }

        // seal the value under the version's storage identity, and keep it beside the version
        const plaintext = new TextEncoder().encode(JSON.stringify(value));
        let envelope: Envelope;
        try {
            envelope = await Envelope.seal(plaintext, key, VaultValue.context(key, owner, version));
        } finally {
            plaintext.fill(0);
        }
        await database.insert(vaultValue).values({ secretId: owner.id, version, ...envelope });
    },

    /** Open one version's value, authenticating its storage identity. */
    async read(
        database: DatabaseConnection,
        key: VaultKey,
        owner: InstanceOf<typeof secret>,
        version: number,
    ): Promise<SecretValue> {
        // require the stored envelope
        const [stored] = await database
            .select()
            .from(vaultValue)
            .where(and(eq(vaultValue.secretId, owner.id), eq(vaultValue.version, version)));
        if (stored === undefined) {
            throw new ServiceError("INTERNAL_SERVER_ERROR", {
                message: "secret ciphertext is missing",
            });
        }

        // open it, clearing the plaintext either way
        const plaintext = await Envelope.open(
            Envelope.schema.strip().parse(stored),
            key,
            VaultValue.context(key, owner, version),
        );
        try {
            return SecretValue.parse(JSON.parse(new TextDecoder().decode(plaintext)));
        } finally {
            plaintext.fill(0);
        }
    },

    /** Destroy one version's value. */
    async destroy(
        database: DatabaseConnection,
        secretId: InstanceOf<typeof secret>["id"],
        version: number,
    ): Promise<void> {
        await database
            .delete(vaultValue)
            .where(and(eq(vaultValue.secretId, secretId), eq(vaultValue.version, version)));
    },

    /** Send a stored value to a transfer's target. */
    async transfer(
        key: VaultKey,
        value: ValueRow,
        identity: ValueIdentity,
        recipient: Recipient,
    ): Promise<ValueTransfer> {
        const context = { ...identity, location: key.location };

        return {
            format: value.format,
            location: key.location,
            ciphertext: value.ciphertext,
            nonce: value.nonce,
            key: await Envelope.transfer(
                Envelope.schema.strip().parse(value),
                key,
                context,
                recipient,
            ),
        };
    },

    /** Receive a value another host sent, sealed again under this vault's key. */
    receive(
        key: VaultKey,
        value: ValueTransfer,
        identity: ValueIdentity,
        recipient: Recipient,
    ): Promise<Envelope> {
        return Envelope.receive(
            value,
            value.key,
            { ...identity, location: value.location },
            { ...identity, location: key.location },
            key,
            recipient,
        );
    },

    /** Build the storage identity a version's ciphertext authenticates. */
    context(key: VaultKey, owner: InstanceOf<typeof secret>, version: number): EncryptionContext {
        return {
            location: key.location,
            spaceId: owner.scope,
            vaultId: owner.parentId,
            secretId: owner.id,
            version,
        };
    },
};
