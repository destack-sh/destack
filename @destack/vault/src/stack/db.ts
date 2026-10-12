import type { Table } from "@destack/db";
import { secret, secretVersion, vault, vaultKey } from "../object/index.ts";

/** The vault kind's tables: the vaults and their key versions, their secrets and versions. */
export const vaultTables: readonly Table[] = [
    ...vault.tables,
    ...vaultKey.tables,
    ...secret.tables,
    ...secretVersion.tables,
];
