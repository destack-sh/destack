import { ObjectReference, type ObjectTypeReference, AccessName, Subject } from "@destack/sync";
import {
    and,
    eq,
    inArray,
    isNull,
    or,
    sql,
    type DatabaseConnection,
    type OpenCondition,
    type Select,
    type SQL,
    type Snapshot,
} from "@destack/db";
import { defineSchema, Instant, schema } from "@destack/schema";
import { v7 } from "uuid";
import {
    accessRelationship,
    type RelationshipColumnMap,
    type EncodedRelationship,
} from "./table.ts";

/** What a request must satisfy for a relationship to apply, beyond its lifetime. */
export const RelationshipCondition = defineSchema(
    schema.object({
        /** Apply only to this request, and only until it commits. */
        request: schema.string().min(1).exactOptional(),
        /** Apply only within this session or agent instance. */
        session: schema.string().min(1).exactOptional(),
        /** Apply only when the request presents the link secret with this digest. */
        linkSecret: schema
            .string()
            .regex(/^[0-9a-f]{64}$(?![\s\S])/u)
            .exactOptional(),
        /** Apply only when the caller authenticated at this assurance level or higher. */
        assurance: schema.number().int().min(1).max(3).exactOptional(),
        /** Apply only within this many milliseconds of the caller's authentication. */
        maxAge: schema.number().int().positive().exactOptional(),
        /** Apply only to a delegate acting for this principal, lending the principal's authority: a delegation. */
        onBehalfOf: Subject.exactOptional(),
    }),
);
/** What a request must satisfy for a relationship to apply, beyond its lifetime. */
export type RelationshipCondition = schema.Infer<typeof RelationshipCondition>;

/** A relationship apart from its relation or role, which each kind of relationship extends. */
export const relationshipBase = schema.object({
    /** The stable relationship identifier. */
    id: schema.string().min(1),
    /** The related object. */
    object: ObjectReference,
    /** The subject, subject set or wildcard related to the object. */
    subject: Subject,
    /** The creation time in Unix milliseconds. */
    createdAt: Instant,
    /** The exclusive expiry time in Unix milliseconds, or null for no expiry. */
    expiresAt: Instant.nullable(),
    /** What a request must satisfy for the relationship to apply. */
    conditions: RelationshipCondition.exactOptional(),
});

/** The schema of a relationship: an object related to a subject through a declared relation, or a bound role. */
const relationshipSchema = defineSchema(
    schema.union([
        relationshipBase.extend({
            /** The declared relation. */
            relation: AccessName,
        }),
        relationshipBase.extend({
            /** The bound role. */
            role: schema.string().min(1),
        }),
    ]),
);
/** An object related to a subject through a declared relation, or a bound role. */
export type Relationship = schema.Infer<typeof relationshipSchema>;

/** A relationship to write: through a declared relation, or binding a role. */
export type RelationshipRequest = {
    /** The related object. */
    readonly object: ObjectReference;
    /** The subject, subject set or wildcard. */
    readonly subject: Subject;
    /** Optional expiry in UTC epoch milliseconds. */
    readonly expiresAt?: number;
    /** What a request must satisfy for the relationship to apply. */
    readonly conditions?: RelationshipCondition;
} & ({ readonly relation: string } | { readonly role: string });

/** An object related to a subject through a declared relation or a bound role: its schema, and its rows. */
export const Relationship = {
    /** The schema of a relationship. */
    schema: relationshipSchema,
    encode,
    decode,
    on,
    readByObject,
    readBySubject,
    heldBy,
    subjectColumns,
    via,
    viaColumns,
    replace,
};

/** The relationships one subject has through one relation on objects of some types in a scope. */
export interface RelationshipSelection {
    /** The scope the relationships live in. */
    readonly scope: string;
    /** The object types the selection covers. */
    readonly objects: readonly ObjectTypeReference[];
    /** The declared relation. */
    readonly relation: string;
    /** The subject of the relationships. */
    readonly subject: Subject;
}

/** Write a relationship as a row in the scope it decides access for. */
function encode(relationship: Relationship, scope: string) {
    return {
        id: schema.identifier("relationship").parse(relationship.id),
        createdAt: relationship.createdAt,
        updatedAt: relationship.createdAt,
        scope,
        objectScope: relationship.object.scope,
        packageId: relationship.object.packageId,
        type: relationship.object.type,
        objectId: relationship.object.id,
        ...viaColumns(relationship),
        ...subjectColumns(relationship.subject),
        expiresAt: relationship.expiresAt,
        requestId: relationship.conditions?.request ?? null,
        sessionId: relationship.conditions?.session ?? null,
        linkSecret: relationship.conditions?.linkSecret ?? null,
        assurance: relationship.conditions?.assurance ?? null,
        maxAge: relationship.conditions?.maxAge ?? null,
        onBehalfOf:
            relationship.conditions?.onBehalfOf === undefined
                ? null
                : Subject.key(relationship.conditions.onBehalfOf),
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
        ...(row.linkSecret === null ? {} : { linkSecret: row.linkSecret }),
        ...(row.assurance === null ? {} : { assurance: row.assurance }),
        ...(row.maxAge === null ? {} : { maxAge: row.maxAge }),
        ...(row.onBehalfOf === null ? {} : { onBehalfOf: Subject.read(row.onBehalfOf) }),
    };

    return {
        id: row.id,
        object: {
            packageId: row.packageId,
            type: row.type,
            scope: row.objectScope,
            id: row.objectId,
        },
        ...viaOf(row),
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
): Promise<EncodedRelationship[]> {
    return snapshot.select(
        accessRelationship,
        ["objectScope", "packageId", "type", "objectId"],
        objects.map((object) => [object.scope, object.packageId, object.type, object.id]),
    );
}

/** Read the relationships of some subjects as a snapshot shows them: plain subjects exactly and through wildcards, sets exactly. */
async function readBySubject(
    snapshot: Snapshot,
    subjects: readonly Subject[],
): Promise<EncodedRelationship[]> {
    // list each subject a relationship may have for them, with the wildcards plain subjects match
    const wanted = new Map<string, readonly (string | null)[]>();
    for (const subject of subjects) {
        const isPlain = subject.relation === undefined;
        for (const scope of isPlain ? [subject.scope, "*"] : [subject.scope]) {
            for (const id of isPlain ? [subject.id, "*"] : [subject.id]) {
                const tuple = [
                    subject.packageId,
                    subject.type,
                    scope,
                    id,
                    subject.relation ?? null,
                ];
                wanted.set(JSON.stringify(tuple), tuple);
            }
        }
    }

    // read the relationships of each wanted subject and relation
    const tuples = new Map(
        [...wanted.values()].map((tuple) => [JSON.stringify(tuple.slice(0, 4)), tuple.slice(0, 4)]),
    );
    const rows = await snapshot.select(
        accessRelationship,
        ["subjectPackageId", "subjectType", "subjectScope", "subjectId"],
        [...tuples.values()],
    );

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

/** Match the relationships some subjects hold, with the wildcards plain subjects match, as a condition on relationship rows. */
function heldBy(subjects: readonly Subject[]): OpenCondition {
    return {
        OR: subjects.map((subject) => {
            const isPlain = subject.relation === undefined;

            return {
                subjectPackageId: subject.packageId,
                subjectType: subject.type,
                subjectScope: isPlain ? { in: [subject.scope, "*"] } : subject.scope,
                subjectId: isPlain ? { in: [subject.id, "*"] } : subject.id,
                subjectRelation: subject.relation ?? { isNull: true },
            };
        }),
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

/** Relate a subject through a relation to exactly the wanted objects of the selected types, as the system. */
async function replace(
    database: DatabaseConnection,
    selection: RelationshipSelection,
    wanted: readonly ObjectReference[],
    now: number,
): Promise<void> {
    // read the relationships the subject has through the relation on the selected types
    const columns = subjectColumns(selection.subject);
    const related = await database
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
    const missing = new Map(wanted.map((object) => [ObjectReference.key(object), object]));
    const stale = related.filter(
        (row) =>
            !missing.delete(
                ObjectReference.key({
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

/** Read whether a relationship row relates through a relation or a role, refusing a row with neither or both. */
function viaOf(
    row: Pick<Select<typeof accessRelationship>, "id" | "relation" | "roleId">,
): { readonly relation: string } | { readonly role: string } {
    if (row.relation !== null && row.roleId === null) {
        return { relation: row.relation };
    } else if (row.roleId !== null && row.relation === null) {
        return { role: row.roleId };
    } else {
        throw new TypeError(`relationship ${row.id} has no single relation or role`);
    }
}

/** Read what a relationship relates through: its declared relation, or its bound role. */
function via(
    relationship: { readonly relation: string } | { readonly role: string },
): { readonly relation: string } | { readonly role: string } {
    return "relation" in relationship
        ? { relation: relationship.relation }
        : { role: relationship.role };
}

/** Write what a relationship relates through as its row's relation and role columns. */
function viaColumns(relationship: { readonly relation: string } | { readonly role: string }) {
    return "relation" in relationship
        ? { relation: relationship.relation, roleId: null }
        : { relation: null, roleId: schema.identifier("role").parse(relationship.role) };
}
