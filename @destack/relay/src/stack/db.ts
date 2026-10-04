import { defineDatabase } from "@destack/db";
import { account, host, hostKey, zone } from "@destack/account/object";
import { serverTables } from "@destack/object";

/** The relay's database: copies of the accounts, hosts, host keys and zones names resolve with. */
export const relayDatabase = defineDatabase({
    name: "main",
    tables: serverTables,
    copies: [account.table, host.table, hostKey.table, zone.table],
});
