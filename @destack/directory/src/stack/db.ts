import type { Table } from "@destack/db";
import { claimTable } from "../claim/claim.ts";
import { assignmentTable, cellTable, zoneTable } from "../zone/zone.ts";
import { identityOperationTable } from "../identity/identity.ts";

/** The directory's tables in the account service's database: zones, their assignments, cells, claims and identities. */
export const directoryTables: readonly Table[] = [
    zoneTable,
    assignmentTable,
    cellTable,
    claimTable,
    identityOperationTable,
];
