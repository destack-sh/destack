import { defineTable, text, identifier, type Table } from "@destack/db";
import { journal } from "@destack/audit";
import { secret, secretVersion, vault } from "../object/secret.ts";

/** Each vault's key, wrapped under the holding host's root key. */
export const vaultKey = defineTable("key", {
    /** The vault. */
    vaultId: identifier("vault_id", "vault").primaryKey(),
    /** The space of the vault. */
    scope: identifier("scope", "space").notNull(),
    /** The key's identifier, which each data key it wraps records. */
    id: text("id").notNull(),
    /** The root key it is wrapped under. */
    rootKeyId: text("root_key_id").notNull(),
    /** The wrapped key, encoded as base64. */
    wrappedKey: text("wrapped_key").notNull(),
    /** The wrapping nonce, encoded as base64. */
    keyNonce: text("key_nonce").notNull(),
});

/** Vaults, secrets, their sealed versions, the vaults' keys and the journal. */
export const vaultTables: readonly Table[] = [
    ...[vault, secret, secretVersion].flatMap((object) => object.tables),
    vaultKey,
    journal,
];
