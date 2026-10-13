import { ObjectReference, type ObjectTypeReference, AccessName, Subject } from "@destack/sync";
import {
    sql,
    type Comparable,
    type Condition,
    type DatabaseConnection,
    type Transaction,
    type OpenCondition,
    type Select,
    type SQL,
    type Snapshot,
} from "@destack/db";
import { defineSchema, Instant, schema } from "@destack/schema";
import { AccessError } from "../error/index.ts";
import type { Manager } from "../manager/manager.ts";
import {
    accessRelationship,
    type RelationshipColumnMap,
    type RelationshipRow,
    type RoleBinding,
} from "./table.ts";

/** What a request must satisfy for a relationship to apply, beyond its lifetime. */
export const RelationshipCondition = defineSchema(
    schema.object({
        /** Apply only to this request, and only until it commits. */
        request: schema.string().min(1).exactOptional(),
        /** Apply only to calls presenting this credential, as `<kind>:<id>`. */
        credential: schema.string().min(1).exactOptional(),
        /** Apply only when the request presents the link secret with this digest. */
        linkSecret: schema
            .string()
            .regex(/^[0-9a-f]{64}$/u)
            .exactOptional(),
        /** Apply only when the caller authenticated at this assurance level or higher. */
        assurance: schema.number().int().min(1).max(3).exactOptional(),
        /** Apply only within this many milliseconds of the caller's authentication. */
        maxAge: schema.number().int().positive().exactOptional(),
        /** Apply only to a delegate acting for this principal, with its lent authority. */
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

/** The schema of a relationship: an object related to a subject by relation or role. */
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

/** An object related to a subject by a declared relation or bound role: schema and rows. */
export const Relationship = {
    /** The schema of a relationship. */
    schema: relationshipSchema,
    encode,
    decode,
    objectOf,
    insert,
    on,
    readByObject,
    readOn,
    readBySubject,
    isBinding,
    heldBy,
    equal,
    objectColumns,
    subjectColumns,
    via,
    viaColumns,
};

/** The scope's relationships a keep covers. */
export type RelationshipSelection = {
    /** The scope the relationships live in. */
    readonly scope: string;
} & (
    | {
          /** The object types the selection covers. */
          readonly objects: readonly ObjectTypeReference[];
          /** The declared relation. */
          readonly relation: string;
          /** The subject of the relationships. */
          readonly subject: Subject;
      }
    | Record<never, never>
);

/** Write a relationship as a row in the scope it decides access for. */
function encode(relationship: Relationship, scope: string) {
    return {
        id: schema.identifier("relationship").parse(relationship.id),
        createdAt: relationship.createdAt,
        updatedAt: relationship.createdAt,
        scope,
        ...objectColumns(relationship.object),
        ...viaColumns(relationship),
        ...subjectColumns(relationship.subject),
        expiresAt: relationship.expiresAt,
        requestId: relationship.conditions?.request ?? null,
        credential: relationship.conditions?.credential ?? null,
        linkSecret: relationship.conditions?.linkSecret ?? null,
        assurance: relationship.conditions?.assurance ?? null,
        maxAge: relationship.conditions?.maxAge ?? null,
        onBehalfOf:
            relationship.conditions?.onBehalfOf === undefined
                ? null
                : Subject.key(relationship.conditions.onBehalfOf),
    };
}

/** Insert a relationship as a row in the scope it decides access for. */
async function insert(
    database: DatabaseConnection,
    relationship: Relationship,
    scope: string,
): Promise<void> {
    await database.insert(accessRelationship).values(encode(relationship, scope));
}

/** Write a relationship's object as its columns, which also match the rows on the object. */
function objectColumns(object: ObjectReference) {
    return {
        objectScope: object.scope,
        packageId: object.packageId,
        type: object.type,
        objectId: object.id,
    };
}

/** Read the object a relationship or role binding row is on, as a logged change shows it. */
function objectOf(row: Readonly<Record<string, unknown>>): ObjectReference {
    return ObjectReference.parse({
        packageId: row["packageId"],
        type: row["type"],
        scope: row["objectScope"],
        id: row["objectId"],
    });
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
        ...(row.credential === null ? {} : { credential: row.credential }),
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
): Promise<RelationshipRow[]> {
    return snapshot.select(
        accessRelationship,
        ["objectScope", "packageId", "type", "objectId"],
        objects.map((object) => [object.scope, object.packageId, object.type, object.id]),
    );
}

/** Read an object's relationships through a relation or role, held by one subject, or both. */
async function readOn(
    snapshot: Snapshot,
    object: ObjectReference,
    match: {
        /** The declared relation, any by default. */
        readonly relation?: string;
        /** The bound role, any by default. */
        readonly role?: string;
        /** The one subject, subject set or wildcard holding them, any by default. */
        readonly subject?: Subject;
    },
): Promise<RelationshipRow[]> {
    // match the object, then each given part exactly
    const { relation, role, subject } = match;

    return snapshot.rows(accessRelationship, {
        ...objectColumns(object),
        ...(relation === undefined ? {} : { relation }),
        ...(role === undefined ? {} : { roleId: schema.identifier("role").parse(role) }),
        ...(subject === undefined
            ? {}
            : {
                  ...subjectColumns(subject),
                  subjectRelation: nullable(subject.relation ?? null),
              }),
    });
}

/** Read some subjects' relationships in a snapshot: plain ones also by wildcard, sets exactly. */
async function readBySubject(
    snapshot: Snapshot,
    subjects: readonly Subject[],
): Promise<RelationshipRow[]> {
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

/** Match some subjects' relationships, with plain subjects' wildcards, as a row condition. */
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

/** Match the rows with an encoded relationship's tuple, as the unique tuple compares them. */
function equal(row: ReturnType<typeof encode>): Condition<RelationshipRow> {
    return {
        ...objectColumns({
            scope: row.objectScope,
            packageId: row.packageId,
            type: row.type,
            id: row.objectId,
        }),
        relation: nullable(row.relation),
        roleId: nullable(row.roleId),
        subjectPackageId: row.subjectPackageId,
        subjectType: row.subjectType,
        subjectScope: row.subjectScope,
        subjectId: row.subjectId,
        subjectRelation: nullable(row.subjectRelation),
        onBehalfOf: nullable(row.onBehalfOf),
        requestId: nullable(row.requestId),
        credential: nullable(row.credential),
        linkSecret: nullable(row.linkSecret),
        assurance: nullable(row.assurance),
        maxAge: nullable(row.maxAge),
    };
}

/** Read a selection's relationships as a transaction sees them. */
export async function readSelected(
    transaction: Transaction,
    selection: RelationshipSelection,
    manager: Manager | null,
): Promise<RelationshipRow[]> {
    let covered: Condition<RelationshipRow>;
    // select a subject's relationships through a relation
    if ("objects" in selection) {
        covered = heldThrough(selection);
    }
    // select the relationships the declaration being applied manages
    else if (manager !== null) {
        covered = managedBy(manager);
    }
    // refuse a caller applying no declaration
    else {
        throw new AccessError(
            "INVALID_CONTEXT",
            "only a declaration keeps the relationships it manages",
        );
    }

    return transaction.rows(accessRelationship, { scope: selection.scope, ...covered });
}

/** Match the relationships one declaration manages. */
function managedBy(manager: Manager): Condition<RelationshipRow> {
    return {
        managerInstallationId: manager.installationId,
        managerPackageId: manager.packageId,
        managerName: manager.name,
    };
}

/** Match the relationships a subject has through a relation on objects of some types. */
function heldThrough(selection: {
    readonly objects: readonly ObjectTypeReference[];
    readonly relation: string;
    readonly subject: Subject;
}): Condition<RelationshipRow> {
    const columns = subjectColumns(selection.subject);

    return {
        OR: selection.objects.map((object) => ({
            packageId: object.packageId,
            type: object.type,
        })),
        relation: selection.relation,
        subjectPackageId: columns.subjectPackageId,
        subjectType: columns.subjectType,
        subjectScope: columns.subjectScope,
        subjectId: columns.subjectId,
        subjectRelation: nullable(columns.subjectRelation),
    };
}

/** Match a value, or a missing one for null. */
function nullable<Value extends Comparable>(
    value: Value | null,
): Value | { readonly isNull: true } {
    return value === null ? { isNull: true } : value;
}

/** Read whether a relationship row uses a relation or role, refusing neither or both. */
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

/** Decide whether a relationship binds a role. */
function isBinding(row: RelationshipRow): row is RoleBinding {
    return row.roleId !== null;
}
