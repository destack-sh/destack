import { defineDatabase } from "@destack/db";
import { account, key, machine, zone } from "@destack/account/object";
import { journal } from "@destack/audit/stack";
import { serverTables } from "@destack/object";

/** The relay's database: copies of the accounts, machines, machine keys and zones names resolve with, and the journal of tunnels opening and closing. */
export const relayDatabase = defineDatabase({
    name: "main",
    tables: [...serverTables, journal],
    copies: [account.table, machine.table, key.table, zone.table],
});
