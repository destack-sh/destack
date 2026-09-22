import { table, text, identifier, integer, primaryKey, foreignKey } from "@destack/db";
import { secretVersion } from "@destack/model/regional";
import { defineRequestTable } from "@destack/service/database";
import { defineDatabaseSchema } from "@destack/db";
import { regionalSchema } from "@destack/model/regional";
import { auditOutboxSchema } from "@destack/audit/outbox";

/** Encrypted values retained only by the managed vault implementation. */
export const vaultValue = table(
    "vault_value",
    {
        /** Secret identity shared with regional metadata. */
        secretId: identifier("secret_id", "secret").notNull(),
        /** Immutable value version. */
        version: integer("version").notNull(),
        /** Envelope format version. */
        format: integer("format").notNull(),
        /** Root-key version. */
        keyId: text("key_id").notNull(),
        /** Authenticated value ciphertext, encoded as base64. */
        ciphertext: text("ciphertext").notNull(),
        /** Value encryption nonce, encoded as base64. */
        nonce: text("nonce").notNull(),
        /** Protected data key, encoded as base64. */
        wrappedKey: text("wrapped_key").notNull(),
        /** Key protection nonce, encoded as base64. */
        keyNonce: text("key_nonce"),
    },
    (entry) => [
        primaryKey({ columns: [entry.secretId, entry.version] }),
        foreignKey({
            columns: [entry.secretId, entry.version],
            foreignColumns: [secretVersion.secretId, secretVersion.version],
        }).onDelete("restrict"),
    ],
);

/** Protected mutation fingerprints and public results in the vault database. */
export const vaultRequest = defineRequestTable("vault_request");

/** Managed secret storage sharing regional metadata and a transactional audit outbox. */
export const vaultSchema = defineDatabaseSchema({
    name: "destack-vault",
    dependencies: [regionalSchema, auditOutboxSchema],
    tables: { vaultValue, vaultRequest },
    migrations: new URL("./migration/", import.meta.url),
});
