import type { Table } from "@destack/db";
import { REPLICA_TABLES } from "@destack/sync";
import type { TableMapping } from "../authorizer/mapping.ts";
import { proposal, relationship, role } from "../policy/principal.ts";
import { accessRelationship } from "../relationship/table.ts";
import { accessRole, accessRolePermission } from "../role/table.ts";
import { accessScope } from "../scope/table.ts";
import { accessProposal } from "../proposal/table.ts";

/** The name of every copy of a scope's access, which each database holds one of per scope it copies. */
export const COPY_NAME = "access";

/** The access tables a decision reads, which databases copy for the scopes they decide in, parents before the rows referencing them. */
export const DECISION_TABLES: readonly Table[] = [
    accessScope,
    accessRole,
    accessRolePermission,
    accessRelationship,
];

/** The tables of every database holding protected objects: the decision tables, proposals, and the copies' records. */
export const ACCESS_TABLES: readonly Table[] = [
    ...DECISION_TABLES,
    accessProposal,
    ...REPLICA_TABLES,
];

/** Map access's own tables as objects, so their rows are listed, followed and explained like any other. */
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
