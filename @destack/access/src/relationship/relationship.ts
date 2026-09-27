import { sql, type Select, type SQL } from "@destack/db";
import { defineSchema, identifier, schema } from "@destack/schema";
import { AccessName } from "../policy/expression.ts";
import { ObjectReference } from "../policy/policy.ts";
import { keySubject, Subject, subjectKey } from "../policy/subject.ts";
import { accessRelationship, type RelationshipColumnMap } from "./table.ts";

/** What a request must satisfy for a relationship to apply, beyond its lifetime. */
export const RelationshipCondition = defineSchema(
    schema.object({
        /** Apply only to this request, and only until it commits. */
        request: schema.string().min(1).optional(),
        /** Apply only within this session or agent instance. */
        session: schema.string().min(1).optional(),
        /** Apply only when the request presents the capability with this digest. */
        capability: schema
            .string()
            .regex(/^[0-9a-f]{64}$(?![\s\S])/)
            .optional(),
        /** Apply only when the caller authenticated at this assurance level or higher. */
        assurance: schema.number().int().min(1).max(3).optional(),
        /** Apply only within this many milliseconds of the caller's authentication. */
        maxAge: schema.number().int().positive().optional(),
        /** Apply only to a delegate acting for this principal, lending the principal's authority: a delegation. */
        onBehalfOf: Subject.optional(),
    }),
);
/** What a request must satisfy for a relationship to apply, beyond its lifetime. */
export type RelationshipCondition = schema.Infer<typeof RelationshipCondition>;

/** The schema of a relationship: an object related to a subject through a declared relation or a bound role. */
const relationshipSchema = defineSchema(
    schema.object({
        /** The stable relationship identifier. */
        id: schema.string().min(1),
        /** The related object. */
        object: ObjectReference,
        /** The declared relation, absent for a role binding. */
        relation: AccessName.optional(),
        /** The bound role, absent for a declared relation. */
        role: schema.string().min(1).optional(),
        /** The subject, subject set or wildcard related to the object. */
        subject: Subject,
        /** The creation time in Unix milliseconds. */
        createdAt: schema.number().int(),
        /** The exclusive expiry time in Unix milliseconds, or null for no expiry. */
        expiresAt: schema.number().int().nullable(),
        /** What a request must satisfy for the relationship to apply. */
        conditions: RelationshipCondition.optional(),
    }),
);
/** An object related to a subject through a declared relation or a bound role. */
export type Relationship = schema.Infer<typeof relationshipSchema>;

/** A relationship to write: exactly one of a relation or a role. */
export interface RelationshipRequest {
    /** The related object. */
    readonly object: ObjectReference;
    /** The declared relation. */
    readonly relation?: string;
    /** The role to bind. */
    readonly role?: string;
    /** The subject, subject set or wildcard. */
    readonly subject: Subject;
    /** Optional expiry in UTC epoch milliseconds. */
    readonly expiresAt?: number;
    /** What a request must satisfy for the relationship to apply. */
    readonly conditions?: RelationshipCondition;
}

/** An object related to a subject through a declared relation or a bound role: its schema, and its rows. */
export const Relationship = {
    /** The schema of a relationship. */
    schema: relationshipSchema,
    encode,
    decode,
    on,
    subjectColumns,
};

/** Write a relationship as its row, living in the scope whose access it decides. */
function encode(relationship: Relationship, scope: string) {
    return {
        id: identifier("relationship").parse(relationship.id),
        createdAt: relationship.createdAt,
        updatedAt: relationship.createdAt,
        scope,
        objectScope: relationship.object.scope,
        packageId: relationship.object.packageId,
        type: relationship.object.type,
        objectId: relationship.object.id,
        relation: relationship.relation ?? null,
        roleId:
            relationship.role === undefined ? null : identifier("role").parse(relationship.role),
        ...subjectColumns(relationship.subject),
        expiresAt: relationship.expiresAt,
        requestId: relationship.conditions?.request ?? null,
        sessionId: relationship.conditions?.session ?? null,
        capability: relationship.conditions?.capability ?? null,
        assurance: relationship.conditions?.assurance ?? null,
        maxAge: relationship.conditions?.maxAge ?? null,
        onBehalfOf:
            relationship.conditions?.onBehalfOf === undefined
                ? null
                : subjectKey(relationship.conditions.onBehalfOf),
    };
}

/** Write a relationship's subject as its columns. */
function subjectColumns(subject: Subject) {
    return {
        subjectPackageId: subject.packageId,
        subjectType: subject.type,
        subjectScope: subject.scope,
        subjectId: subject.id,
        subjectRelation: subject.relation ?? null,
    };
}

/** Read a relationship from its row. */
function decode(row: Select<typeof accessRelationship>): Relationship {
    const conditions = {
        ...(row.requestId === null ? {} : { request: row.requestId }),
        ...(row.sessionId === null ? {} : { session: row.sessionId }),
        ...(row.capability === null ? {} : { capability: row.capability }),
        ...(row.assurance === null ? {} : { assurance: row.assurance }),
        ...(row.maxAge === null ? {} : { maxAge: row.maxAge }),
        ...(row.onBehalfOf === null ? {} : { onBehalfOf: keySubject(row.onBehalfOf) }),
    };

    return {
        id: row.id,
        object: {
            packageId: row.packageId,
            type: row.type,
            scope: row.objectScope,
            id: row.objectId,
        },
        ...(row.relation === null ? {} : { relation: row.relation }),
        ...(row.roleId === null ? {} : { role: row.roleId }),
        subject: {
            packageId: row.subjectPackageId,
            type: row.subjectType,
            scope: row.subjectScope,
            id: row.subjectId,
            ...(row.subjectRelation === null ? {} : { relation: row.subjectRelation }),
        },
        createdAt: row.createdAt,
        expiresAt: row.expiresAt,
        ...(Object.keys(conditions).length === 0 ? {} : { conditions }),
    };
}

/** Match the relationships on one object, in the relationship table or one of its aliases. */
function on(
    object: ObjectReference,
    relationship: RelationshipColumnMap = accessRelationship,
): SQL {
    return sql`(
        ${relationship.objectScope} = ${object.scope}
        AND ${relationship.packageId} = ${object.packageId}
        AND ${relationship.type} = ${object.type}
        AND ${relationship.objectId} = ${object.id}
    )`;
}
