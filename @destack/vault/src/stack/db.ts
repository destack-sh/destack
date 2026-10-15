import type { Table } from "@destack/db";
import { secret, secretVersion, vault } from "../object/index.ts";

/** The vault kind's tables: the vaults, their secrets and the secrets' versions. */
export const vaultTables: readonly Table[] = [
    ...vault.tables,
    ...secret.tables,
    ...secretVersion.tables,
];
