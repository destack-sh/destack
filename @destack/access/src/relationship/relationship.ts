import {
    and,
    eq,
    inArray,
    isNull,
    or,
    sql,
    type DatabaseConnection,
    type Select,
    type SQL,
} from "@destack/db";
import type { Snapshot } from "@destack/db/log";
import { defineSchema, identifier, schema } from "@destack/schema";
import { v7 } from "uuid";
import { AccessName } from "../policy/expression.ts";
import { ObjectReference, objectKey, type TypeReference } from "../policy/policy.ts";
import { keySubject, Subject, subjectKey } from "../policy/subject.ts";
import { accessRelationship, type RelationshipColumnMap, type RelationshipRow } from "./table.ts";

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
    readByObject,
    readBySubject,
    subjectColumns,
    replace,
};

/** The relationships one subject holds through one relation on objects of some types in a scope. */
export interface RelationshipSelection {
    /** The scope the relationships live in. */
    readonly scope: string;
    /** The object types the selection covers. */
    readonly objects: readonly TypeReference[];
    /** The declared relation. */
    readonly relation: string;
    /** The subject holding the relationships. */
    readonly subject: Subject;
}

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

/** Read the relationships on some objects as a snapshot shows them. */
async function readByObject(
    snapshot: Snapshot,
    objects: readonly ObjectReference[],
): Promise<RelationshipRow[]> {
    return (await snapshot.select(
        accessRelationship,
        ["objectScope", "packageId", "type", "objectId"],
        objects.map((object) => [object.scope, object.packageId, object.type, object.id]),
    )) as RelationshipRow[];
}

/** Read the relationships some subjects hold as a snapshot shows them: plain subjects exactly and through wildcards, sets exactly. */
async function readBySubject(
    snapshot: Snapshot,
    subjects: readonly Subject[],
): Promise<RelationshipRow[]> {
    // name each subject a relationship may hold for them, with the wildcards plain subjects match
    const wanted = new Map<string, readonly (string | null)[]>();
    for (const subject of subjects) {
        const isPlain = subject.relation === undefined;
        for (const scope of isPlain ? [subject.scope, "*"] : [subject.scope]) {
            for (const id of isPlain ? [subject.id, "*"] : [subject.id]) {
                const held = [subject.packageId, subject.type, scope, id, subject.relation ?? null];
                wanted.set(JSON.stringify(held), held);
            }
        }
    }

    // read the relationships by subject, keeping those holding a wanted subject with its relation
    const tuples = new Map(
        [...wanted.values()].map((held) => [JSON.stringify(held.slice(0, 4)), held.slice(0, 4)]),
    );
    const rows = (await snapshot.select(
        accessRelationship,
        ["subjectPackageId", "subjectType", "subjectScope", "subjectId"],
        [...tuples.values()],
    )) as RelationshipRow[];

    return rows.filter((row) =>
        wanted.has(
            JSON.stringify([
                row.subjectPackageId,
                row.subjectType,
                row.subjectScope,
                row.subjectId,
                row.subjectRelation,
            ]),
        ),
    );
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

/** Relate a subject through a relation to exactly the wanted objects of the selected types, as the system. */
async function replace(
    database: DatabaseConnection,
    selection: RelationshipSelection,
    wanted: readonly ObjectReference[],
    now: number,
): Promise<void> {
    // read the relationships the subject holds through the relation on the selected types
    const columns = subjectColumns(selection.subject);
    const held = await database
        .select({
            id: accessRelationship.id,
            objectScope: accessRelationship.objectScope,
            packageId: accessRelationship.packageId,
            type: accessRelationship.type,
            objectId: accessRelationship.objectId,
        })
        .from(accessRelationship)
        .where(
            and(
                eq(accessRelationship.scope, selection.scope),
                selection.objects.length === 0
                    ? sql`false`
                    : or(
                          ...selection.objects.map((object) =>
                              and(
                                  eq(accessRelationship.packageId, object.packageId),
                                  eq(accessRelationship.type, object.type),
                              ),
                          ),
                      ),
                eq(accessRelationship.relation, selection.relation),
                eq(accessRelationship.subjectPackageId, columns.subjectPackageId),
                eq(accessRelationship.subjectType, columns.subjectType),
                eq(accessRelationship.subjectScope, columns.subjectScope),
                eq(accessRelationship.subjectId, columns.subjectId),
                columns.subjectRelation === null
                    ? isNull(accessRelationship.subjectRelation)
                    : eq(accessRelationship.subjectRelation, columns.subjectRelation),
            ),
        );

    // remove the relationships to objects no longer wanted
    const missing = new Map(wanted.map((object) => [objectKey(object), object]));
    const stale = held.filter(
        (row) =>
            !missing.delete(
                objectKey({
                    scope: row.objectScope,
                    packageId: row.packageId,
                    type: row.type,
                    id: row.objectId,
                }),
            ),
    );
    if (stale.length > 0) {
        await database.delete(accessRelationship).where(
            inArray(
                accessRelationship.id,
                stale.map((row) => row.id),
            ),
        );
    }

    // relate the subject to the wanted objects it lacks
    if (missing.size > 0) {
        await database.insert(accessRelationship).values(
            [...missing.values()].map((object) =>
                encode(
                    {
                        id: `relationship-${v7()}`,
                        object,
                        relation: selection.relation,
                        subject: selection.subject,
                        createdAt: now,
                        expiresAt: null,
                    },
                    selection.scope,
                ),
            ),
        );
    }
}
