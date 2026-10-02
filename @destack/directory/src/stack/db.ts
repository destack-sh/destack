import type { Table } from "@destack/db";
import { claimTable } from "../claim/claim.ts";
import { assignmentTable, cellTable, zoneTable } from "../zone/zone.ts";

/** The directory's tables in the global database: zones, their assignments, cells and claims. */
export const directoryTables: readonly Table[] = [
    zoneTable,
    assignmentTable,
    cellTable,
    claimTable,
];
