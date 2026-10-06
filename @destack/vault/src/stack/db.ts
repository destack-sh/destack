import { journal } from "@destack/audit/stack";
import { defineDatabase, defineTable, identifier, type Table, text } from "@destack/db";
import { binding, capture, deployment, installation, space } from "@destack/space/object";
import { secret, secretVersion, vault } from "../object/index.ts";

/** Each vault's key, encrypted under the holding host's keyring. */
export const vaultKey = defineTable(
    "key",
    {
        /** The vault. */
        vaultId: identifier("vault_id", "vault").primaryKey(),
        /** The space of the vault. */
        scope: identifier("scope", "space").notNull(),
        /** The key's identifier, which each value it encrypts names. */
        id: text("id").notNull(),
        /** The key's bytes, encrypted under the host's keyring. */
        ciphertext: text("ciphertext").notNull().sensitive(),
    },
    { log: {} },
);

/** The vault service's tables: its vaults and their keys, their secrets and versions, and its journal. */
export const vaultTables: readonly Table[] = [
    ...vault.tables,
    vaultKey,
    ...secret.tables,
    ...secretVersion.tables,
    journal,
];

/** The vault service's database, with copies of the spaces its vaults live in and of the installations, bindings and captures its secrets' checks read. */
export const vaultDatabase = defineDatabase({
    name: "vault",
    tables: vaultTables,
    copies: [space.table, installation.table, binding.table, capture.table, deployment.table],
});
