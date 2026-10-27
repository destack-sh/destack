import type { Table } from "@destack/db";
import { replicaTables, Scope } from "@destack/sync";
import type { TableMapping } from "../authorizer/mapping.ts";
import { grant, invitation, relationship, role } from "../declare/principal.ts";
import { accessRelationship } from "../relationship/table.ts";
import { accessRole, accessRolePermission } from "../role/table.ts";
import { accessInvitation } from "../invitation/table.ts";
import { accessGrant } from "../grant/table.ts";
import { nonce } from "../token/nonce.ts";

/** The name of the publication replicating a scope's chain. */
export const CHAIN_PUBLICATION = "chain";

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
    accessGrant,
    nonce,
    ...replicaTables,
];

/** The mappings of access's tables. */
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
            object: {
                packageId: "objectPackageId",
                type: "objectType",
                scope: "objectScope",
                id: "objectId",
            },
        },
    },
    {
        policy: invitation,
        table: accessInvitation,
        id: "id",
        scope: "scope",
        attributes: {},
        relations: {
            inviter: {
                column: "inviterId",
                subject: {
                    packageId: "inviterPackageId",
                    type: "inviterType",
                    scope: "inviterScope",
                },
            },
            addressee: {
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
            object: {
                packageId: "objectPackageId",
                type: "objectType",
                scope: "objectScope",
                id: "objectId",
            },
        },
    },
    {
        policy: grant,
        table: accessGrant,
        id: "id",
        scope: "scope",
        attributes: {},
        relations: {
            delegator: {
                column: "delegatorId",
                subject: {
                    packageId: "delegatorPackageId",
                    type: "delegatorType",
                    scope: "delegatorScope",
                },
            },
            delegate: {
                column: "delegateId",
                subject: {
                    packageId: "delegatePackageId",
                    type: "delegateType",
                    scope: "delegateScope",
                },
            },
            "delegate-space": { column: "delegateScope", scope: Scope.universe.id },
        },
    },
];
