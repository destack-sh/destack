import {
    defineTable,
    text,
    identifier,
    integer,
    primaryKey,
    foreignKey,
    type Table,
} from "@destack/db";
import { defineJournal } from "@destack/service/database";
import { secret, secretVersion, vault } from "../object/secret.ts";

/** The encrypted value of each secret version, apart from its record. */
export const vaultValue = defineTable(
    "value",
    {
        /** The secret. */
        secretId: identifier("secret_id", "secret").notNull(),
        /** Immutable value version. */
        version: integer("version").notNull(),
        /** Envelope format version. */
        format: integer("format").notNull(),
        /** The vault key wrapping the data key. */
        keyId: text("key_id").notNull(),
        /** Authenticated value ciphertext, encoded as base64. */
        ciphertext: text("ciphertext").notNull(),
        /** Value encryption nonce, encoded as base64. */
        nonce: text("nonce").notNull(),
        /** Protected data key, encoded as base64. */
        wrappedKey: text("wrapped_key").notNull(),
        /** Key protection nonce, encoded as base64. */
        keyNonce: text("key_nonce").notNull(),
    },
    {
        constraints: (entry) => [
            primaryKey({ columns: [entry.secretId, entry.version] }),
            foreignKey({
                columns: [entry.secretId, entry.version],
                foreignColumns: [secretVersion.table.parentId, secretVersion.table.number],
            }).onDelete("restrict"),
        ],
    },
);

/** Each vault's key, wrapped under a root key, kept apart from zone transfers since only this host unwraps it. */
export const vaultKey = defineTable("key", {
    /** The vault. */
    vaultId: identifier("vault_id", "resource").primaryKey(),
    /** The key's identifier, which each data key it wraps records. */
    id: text("id").notNull(),
    /** The root key it is wrapped under. */
    rootKeyId: text("root_key_id").notNull(),
    /** The wrapped key, encoded as base64. */
    wrappedKey: text("wrapped_key").notNull(),
    /** The wrapping nonce, encoded as base64. */
    keyNonce: text("key_nonce").notNull(),
});

/** The vault's executed requests and their public results. */
export const vaultJournal = defineJournal("journal");

/** Vaults, secrets, their versions, their encrypted values, the vaults' keys and the journal. */
export const vaultTables: readonly Table[] = [
    ...[vault, secret, secretVersion].flatMap((object) => object.tables),
    vaultValue,
    vaultKey,
    vaultJournal,
];
