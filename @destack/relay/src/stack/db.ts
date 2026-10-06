import { defineDatabase } from "@destack/db";
import { account, key, machine, zone } from "@destack/account/object";
import { serverTables } from "@destack/object";

/** The relay's database: copies of the accounts, machines, machine keys and zones names resolve with. */
export const relayDatabase = defineDatabase({
    name: "main",
    tables: serverTables,
    copies: [account.table, machine.table, key.table, zone.table],
});
