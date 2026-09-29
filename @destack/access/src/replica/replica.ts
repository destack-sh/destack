import type { Table } from "@destack/db";
import { replicaTables, Scope } from "@destack/sync";
import type { TableMapping } from "../authorizer/mapping.ts";
import { proposal, relationship, role } from "../policy/principal.ts";
import { accessRelationship } from "../relationship/table.ts";
import { accessRole, accessRolePermission } from "../role/table.ts";
import { accessProposal } from "../proposal/table.ts";

/** The name of each database's copy of its scope chain. */
export const COPY_NAME = "chain";

/** The access tables decisions read, parents first. */
export const decisionTables: readonly Table[] = [
    Scope.table,
    accessRole,
    accessRolePermission,
    accessRelationship,
];

/** The tables of every database holding protected objects. */
export const accessTables: readonly Table[] = [...decisionTables, accessProposal, ...replicaTables];

/** The mappings of access's own tables. */
export const ACCESS_MAPPINGS: readonly TableMapping[] = [
    { policy: role, table: accessRole, id: "id", scope: "scope", attributes: {}, relations: {} },
    {
        policy: relationship,
        table: accessRelationship,
        id: "id",
        scope: "scope",
        attributes: {},
        relations: {
            subject: {
                column: "subjectId",
                subject: {
                    packageId: "subjectPackageId",
                    type: "subjectType",
                    scope: "subjectScope",
                    relation: "subjectRelation",
                },
            },
        },
        references: {
            object: { packageId: "packageId", type: "type", scope: "objectScope", id: "objectId" },
        },
    },
    {
        policy: proposal,
        table: accessProposal,
        id: "id",
        scope: "scope",
        attributes: {},
        relations: {
            proposer: { column: "proposerKey", isKey: true },
            addressee: { column: "addressee", isKey: true },
            lender: { column: "lender", isKey: true },
        },
        references: {
            object: { packageId: "packageId", type: "type", scope: "objectScope", id: "objectId" },
        },
    },
];
