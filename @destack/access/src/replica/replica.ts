import type { Table } from "@destack/db";
import { replicaTables, Scope } from "@destack/sync";
import type { TableMapping } from "../authorizer/mapping.ts";
import { invitation, relationship, role } from "../policy/principal.ts";
import { accessRelationship } from "../relationship/table.ts";
import { accessRole, accessRolePermission } from "../role/table.ts";
import { accessInvitation } from "../invitation/table.ts";

/** The name of the shape copying a scope's chain. */
export const CHAIN_SHAPE = "chain";

/** The access tables decisions read, parents first. */
export const decisionTables: readonly Table[] = [
    Scope.table,
    accessRole,
    accessRolePermission,
    accessRelationship,
];

/** The tables of every database with protected objects. */
export const accessTables: readonly Table[] = [
    ...decisionTables,
    accessInvitation,
    ...replicaTables,
];

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
        policy: invitation,
        table: accessInvitation,
        id: "id",
        scope: "scope",
        attributes: {},
        relations: {
            inviter: { column: "inviterKey", isKey: true },
            addressee: { column: "addressee", isKey: true },
            lender: { column: "lender", isKey: true },
        },
        references: {
            object: { packageId: "packageId", type: "type", scope: "objectScope", id: "objectId" },
        },
    },
];
