import { defineDatabase } from "@destack/db";
import { account, claim, key, machine, placement } from "@destack/account/object";
import { journal } from "@destack/audit/stack";
import { storeTables } from "@destack/object";
import { connection, tunnel } from "../object/index.ts";

/** The relay's database: its tunnels and connections, its journal and the copies it routes by. */
export const relayDatabase = defineDatabase({
    name: "main",
    tables: [...storeTables, ...tunnel.tables, ...connection.tables, journal],
    copies: [account.table, machine.table, key.table, placement.table, claim.table],
});
