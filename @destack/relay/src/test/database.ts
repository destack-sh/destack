import { account, claim, key, machine, placement } from "@destack/account/object";
import { journal } from "@destack/audit/stack";
import { defineDatabase } from "@destack/db";
import { storeTables } from "@destack/object";
import { connection, tunnel } from "../object/index.ts";

/** A residency home's main database as the relay uses it: its tunnels, journal and copies. */
export const homeDatabase = defineDatabase({
    name: "main",
    tables: [...storeTables, ...tunnel.tables, ...connection.tables, journal],
    copies: [account.table, machine.table, key.table, placement.table, claim.table],
});
