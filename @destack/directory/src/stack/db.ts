import type { Table } from "@destack/db";
import { claimTable } from "../claim/claim.ts";
import { assignmentTable, endpointTable, placementTable } from "../placement/placement.ts";
import { identityOperation } from "../identity/operation.ts";

/** The directory's tables in the account service's database: placements, their assignments, endpoints, claims and identities. */
export const directoryTables: readonly Table[] = [
    placementTable,
    assignmentTable,
    endpointTable,
    claimTable,
    identityOperation,
];
