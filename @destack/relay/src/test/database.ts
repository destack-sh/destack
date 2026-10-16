import {
    account,
    claim,
    endpoint,
    identity,
    key,
    machine,
    operation,
    placement,
} from "@destack/account/object";
import { defineDatabase } from "@destack/db";
import { storeTables } from "@destack/object";
import { callTables } from "@destack/object/test";
import { connection, tunnel } from "../object/index.ts";

/** A residency home's main database as the relay uses it: its tunnels, calls and replicas. */
export const homeDatabase = defineDatabase({
    name: "main",
    tables: [...storeTables, ...tunnel.tables, ...connection.tables, ...callTables],
    replicated: [
        account.table,
        machine.table,
        key.table,
        endpoint.table,
        placement.table,
        claim.table,
        identity.table,
        operation.table,
    ],
});

/** A machine's tunnel partition's database, replicating its tunnel row and the identities. */
export const tunnelDatabase = defineDatabase({
    name: "main",
    tables: [...storeTables, ...connection.tables, ...callTables],
    replicated: [tunnel.table, identity.table, operation.table],
});
