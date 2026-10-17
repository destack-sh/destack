import { v7 } from "uuid";
import { nth } from "@destack/schema";
import { Table } from "@destack/db";
import { anyone } from "../declare/principal.ts";
import { LinkSecret, type Link } from "./link.ts";
import { OWNER_ROLE } from "../role/table.ts";
import type { Authorization } from "../authorizer/authorization.ts";
import {
    ObjectReference,
    type ObjectTypeReference,
    AccessName,
    type ReferenceColumns,
    Subject,
} from "@destack/sync";
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
import { Instant, schema } from "@destack/schema";
import { AccessError } from "../error/index.ts";
import { Manager } from "../manager/manager.ts";
import {
    accessRelationship,
    type RelationshipColumnMap,
    type RelationshipRow,
    type RoleBinding,
} from "./table.ts";

/** A relationship as its row encodes it, before a manager and revision apply. */
type EncodedRelationship = Omit<
    Select<typeof accessRelationship>,
    "revision" | "managerInstallationId" | "managerPackageId" | "managerName" | "detachedAt"
>;

/** What a request must satisfy for a relationship to apply, beyond its lifetime. */
export const RelationshipCondition = schema.object({
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
});
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
const relationshipSchema = schema.union([
    relationshipBase.extend({
        /** The declared relation. */
        relation: AccessName,
    }),
    relationshipBase.extend({
        /** The bound role. */
        role: schema.string().min(1),
    }),
]);
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

    /** Write a relationship as a row in the scope it decides access for. */
    encode(relationship: Relationship, scope: string): EncodedRelationship {
        return {
            id: schema.identifier("relationship").parse(relationship.id),
            createdAt: relationship.createdAt,
            updatedAt: relationship.createdAt,
            scope,
            ...Relationship.objectColumns(relationship.object),
            ...Relationship.viaColumns(relationship),
            ...Relationship.subjectColumns(relationship.subject),
            expiresAt: relationship.expiresAt,
            ...Relationship.conditionColumns(relationship.conditions),
        };
    },

    /** Write what a request must satisfy as a row's condition columns. */
    conditionColumns(
        conditions: RelationshipCondition | undefined,
    ): Pick<
        Select<typeof accessRelationship>,
        "requestId" | "credential" | "linkSecret" | "assurance" | "maxAge"
    > {
        return {
            requestId: conditions?.request ?? null,
            credential: conditions?.credential ?? null,
            linkSecret: conditions?.linkSecret ?? null,
            assurance: conditions?.assurance ?? null,
            maxAge: conditions?.maxAge ?? null,
        };
    },

    /** Read what a request must satisfy from a row's condition columns, absent for nothing. */
    conditionsOf(
        row: Pick<
            Select<typeof accessRelationship>,
            "requestId" | "credential" | "linkSecret" | "assurance" | "maxAge"
        >,
    ): { readonly conditions?: RelationshipCondition } {
        const conditions = {
            ...(row.requestId === null ? {} : { request: row.requestId }),
            ...(row.credential === null ? {} : { credential: row.credential }),
            ...(row.linkSecret === null ? {} : { linkSecret: row.linkSecret }),
            ...(row.assurance === null ? {} : { assurance: row.assurance }),
            ...(row.maxAge === null ? {} : { maxAge: row.maxAge }),
        };

        return Object.keys(conditions).length === 0 ? {} : { conditions };
    },

    /** Read a row's subject from its subject columns. */
    subjectOf(
        row: Pick<
            Select<typeof accessRelationship>,
            "subjectPackageId" | "subjectType" | "subjectScope" | "subjectId" | "subjectRelation"
        >,
    ): Subject {
        return {
            packageId: row.subjectPackageId,
            type: row.subjectType,
            scope: row.subjectScope,
            id: row.subjectId,
            ...(row.subjectRelation === null ? {} : { relation: row.subjectRelation }),
        };
    },

    /** Insert a relationship as a row in the scope it decides access for. */
    async insert(
        database: DatabaseConnection,
        relationship: Relationship,
        scope: string,
    ): Promise<void> {
        await database.insert(accessRelationship).values(Relationship.encode(relationship, scope));
    },

    /** Write a relationship's object as its columns, which also match the rows on the object. */
    objectColumns(object: ObjectReference): ReferenceColumns<"object"> {
        return ObjectReference.columns("object", object);
    },

    /** Write a relationship's subject as its columns. */
    subjectColumns(
        subject: Subject,
    ): Pick<
        Select<typeof accessRelationship>,
        "subjectPackageId" | "subjectType" | "subjectScope" | "subjectId" | "subjectRelation"
    > {
        return {
            subjectPackageId: subject.packageId,
            subjectType: subject.type,
            subjectScope: subject.scope,
            subjectId: subject.id,
            subjectRelation: subject.relation ?? null,
        };
    },

    /** Read a relationship from its row. */
    decode(row: Select<typeof accessRelationship>): Relationship {
        return {
            id: row.id,
            object: ObjectReference.at("object", row),
            ...Relationship.viaRelation(row),
            subject: Relationship.subjectOf(row),
            createdAt: row.createdAt,
            expiresAt: row.expiresAt,
            ...Relationship.conditionsOf(row),
        };
    },

    /** Read the relationships on some objects as a snapshot shows them. */
    async readByObject(
        snapshot: Snapshot,
        objects: readonly ObjectReference[],
    ): Promise<RelationshipRow[]> {
        return snapshot.select(
            accessRelationship,
            ["objectPackageId", "objectType", "objectScope", "objectId"],
            objects.map((object) => [object.packageId, object.type, object.scope, object.id]),
        );
    },

    /** Read an object's relationships through a relation or role, held by one subject, or both. */
    async readOn(
        reader: Pick<Snapshot, "read">,
        object: ObjectReference,
        match: {
            /** The declared relation, any by default. */
            readonly relation?: string;
            /** The bound role, any by default. */
            readonly role?: string;
            /** The one subject, subject set or wildcard with them, any by default. */
            readonly subject?: Subject;
        },
    ): Promise<RelationshipRow[]> {
        // match the object, then each given field exactly
        const { relation, role, subject } = match;

        return reader.read(accessRelationship).findMany({
            where: {
                ...Relationship.objectColumns(object),
                ...(relation === undefined ? {} : { relation }),
                ...(role === undefined ? {} : { roleId: schema.identifier("role").parse(role) }),
                ...(subject === undefined
                    ? {}
                    : {
                          ...Relationship.subjectColumns(subject),
                          subjectRelation: Relationship.nullable(subject.relation ?? null),
                      }),
            },
        });
    },

    /** Read some subjects' relationships: plain ones also by wildcard, sets exactly. */
    async readBySubject(
        snapshot: Snapshot,
        subjects: readonly Subject[],
    ): Promise<RelationshipRow[]> {
        // list each possible relationship subject, with the wildcards that match plain subjects
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
            [...wanted.values()].map((tuple) => [
                JSON.stringify(tuple.slice(0, 4)),
                tuple.slice(0, 4),
            ]),
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
    },

    /** Match the rows of exactly one subject, as a row condition. */
    subjectCondition(subject: Subject): OpenCondition {
        return {
            subjectPackageId: subject.packageId,
            subjectType: subject.type,
            subjectScope: subject.scope,
            subjectId: subject.id,
            subjectRelation: subject.relation ?? { isNull: true },
        };
    },

    /** Match some subjects' relationships, with plain subjects' wildcards, as a row condition. */
    heldBy(subjects: readonly Subject[]): OpenCondition {
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
    },

    /** Match the relationships on one object, in the relationship table or one of its aliases. */
    on(object: ObjectReference, relationship: RelationshipColumnMap = accessRelationship): SQL {
        return sql`(
            ${relationship.objectPackageId} = ${object.packageId}
            AND ${relationship.objectType} = ${object.type}
            AND ${relationship.objectScope} = ${object.scope}
            AND ${relationship.objectId} = ${object.id}
        )`;
    },

    /** Match the rows with an encoded relationship's tuple, as the unique tuple compares them. */
    equal(row: EncodedRelationship): Condition<RelationshipRow> {
        return {
            ...Relationship.objectColumns(ObjectReference.at("object", row)),
            relation: Relationship.nullable(row.relation),
            roleId: Relationship.nullable(row.roleId),
            subjectPackageId: row.subjectPackageId,
            subjectType: row.subjectType,
            subjectScope: row.subjectScope,
            subjectId: row.subjectId,
            subjectRelation: Relationship.nullable(row.subjectRelation),
            requestId: Relationship.nullable(row.requestId),
            credential: Relationship.nullable(row.credential),
            linkSecret: Relationship.nullable(row.linkSecret),
            assurance: Relationship.nullable(row.assurance),
            maxAge: Relationship.nullable(row.maxAge),
        };
    },

    /** Read a selection's relationships as a transaction sees them. */
    async readSelected(
        transaction: Transaction,
        selection: RelationshipSelection,
        manager: Manager | null,
    ): Promise<RelationshipRow[]> {
        let covered: Condition<RelationshipRow>;
        // select a subject's relationships through a relation
        if ("objects" in selection) {
            covered = Relationship.heldThrough(selection);
        }
        // select the relationships the declaration being applied manages
        else if (manager !== null) {
            covered = Relationship.managedBy(manager);
        }
        // refuse a caller applying no declaration
        else {
            throw new AccessError("INVALID_CONTEXT", "only a declaration can manage relationships");
        }

        return transaction
            .read(accessRelationship)
            .findMany({ where: { scope: selection.scope, ...covered } });
    },

    /** Match the relationships one declaration manages. */
    managedBy(manager: Manager): Condition<RelationshipRow> {
        return {
            managerInstallationId: manager.installationId,
            managerPackageId: manager.packageId,
            managerName: manager.name,
        };
    },

    /** Match the relationships a subject has through a relation on objects of some types. */
    heldThrough(selection: {
        readonly objects: readonly ObjectTypeReference[];
        readonly relation: string;
        readonly subject: Subject;
    }): Condition<RelationshipRow> {
        const columns = Relationship.subjectColumns(selection.subject);

        return {
            OR: selection.objects.map((object) => ({
                objectPackageId: object.packageId,
                objectType: object.type,
            })),
            relation: selection.relation,
            subjectPackageId: columns.subjectPackageId,
            subjectType: columns.subjectType,
            subjectScope: columns.subjectScope,
            subjectId: columns.subjectId,
            subjectRelation: Relationship.nullable(columns.subjectRelation),
        };
    },

    /** Match a value, or a missing one for null. */
    nullable<Value extends Comparable>(value: Value | null): Value | { readonly isNull: true } {
        return value === null ? { isNull: true } : value;
    },

    /** Read whether a relationship row uses a relation or role, refusing neither or both. */
    viaRelation(
        row: Pick<Select<typeof accessRelationship>, "relation" | "roleId"> & {
            readonly id: string;
        },
    ): { readonly relation: string } | { readonly role: string } {
        if (row.relation !== null && row.roleId === null) {
            return { relation: row.relation };
        } else if (row.roleId !== null && row.relation === null) {
            return { role: row.roleId };
        } else {
            throw new TypeError(`relationship ${row.id} has no single relation or role`);
        }
    },

    /** Read what a relationship relates through: its declared relation, or its bound role. */
    via(
        relationship: { readonly relation: string } | { readonly role: string },
    ): { readonly relation: string } | { readonly role: string } {
        return "relation" in relationship
            ? { relation: relationship.relation }
            : { role: relationship.role };
    },

    /** Write what a relationship relates through as its row's relation and role columns. */
    viaColumns(
        relationship: { readonly relation: string } | { readonly role: string },
    ): Pick<Select<typeof accessRelationship>, "relation" | "roleId"> {
        return "relation" in relationship
            ? { relation: relationship.relation, roleId: null }
            : { relation: null, roleId: schema.identifier("role").parse(relationship.role) };
    },

    /** Decide whether a relationship binds a role. */
    isBinding(row: RelationshipRow): row is RoleBinding {
        return row.roleId !== null;
    },

    /** Relate a subject to an object by a relation or bound role, as the caller may grant. */
    async grant(request: RelationshipRequest, authorization: Authorization): Promise<Relationship> {
        const relationship = await authorization.transaction.savepoint(async () => {
            await authorization.authorizeGrant(request);

            return Relationship.create(request, authorization);
        });
        authorization.renew();

        return relationship;
    },

    /** Revoke one relationship of an object as the caller may, its history left in the log. */
    async revoke(
        object: ObjectReference,
        id: string,
        authorization: Authorization,
    ): Promise<Relationship> {
        return authorization.transaction.savepoint(async () => {
            // read the object's relationship
            const key = schema.identifier("relationship").parse(id);
            const row = await authorization.transaction.read(accessRelationship).findFirst({
                where: {
                    ...Relationship.objectColumns(object),
                    id: key,
                },
            });
            if (!row) {
                throw new AccessError("NOT_FOUND", "relationship not found");
            }

            // require permission to revoke what it grants, and leave an owner
            const relationship = Relationship.decode(row);
            await authorization.authorizeRevoke(relationship, row);
            await requireRemainingOwner(authorization.transaction, row);
            await authorization.transaction.write({
                table: accessRelationship,
                before: row,
                after: null,
            });
            authorization.renew();

            return relationship;
        });
    },

    /** Relate the holder of a new secret to an object and return the secret once. */
    async link(
        request: {
            readonly object: ObjectReference;
            readonly relation: string;
            readonly expiresAt?: number;
        },
        authorization: Authorization,
    ): Promise<Link> {
        const linkSecret = await LinkSecret.create();
        const relationship = await Relationship.grant(
            {
                ...request,
                subject: anyone.reference("*", "*"),
                conditions: { linkSecret: linkSecret.digest },
            },
            authorization,
        );

        return { id: relationship.id, secret: linkSecret.secret };
    },

    /** Replace a selection's relationships with exactly the wanted ones, as the caller may. */
    async replace(
        selection: RelationshipSelection,
        wanted: readonly RelationshipRequest[],
        authorization: Authorization,
    ): Promise<void> {
        await authorization.transaction.savepoint(async () => {
            // read the relationships the selection covers
            const related = (
                await Relationship.readSelected(
                    authorization.transaction,
                    selection,
                    authorization.manager,
                )
            ).map((row) => ({
                row,
                relationship: Relationship.decode(row),
            }));

            // revoke the relationships no longer wanted
            const missing = new Map(wanted.map((request) => [identity(request), request]));
            const stale = related.filter(
                ({ relationship }) => !missing.delete(identity(relationship)),
            );
            for (const { row, relationship } of stale) {
                await authorization.authorizeRevoke(relationship, row);
                await authorization.transaction.write({
                    table: accessRelationship,
                    before: row,
                    after: null,
                });
            }

            // grant the wanted relationships missing
            for (const request of missing.values()) {
                await authorization.authorizeGrant(request);
                await Relationship.create(request, authorization);
            }
        });
        authorization.renew();
    },

    /** Insert a valid relationship under the caller's declaration, refusing duplicates. */
    async create(
        request: RelationshipRequest,
        authorization: Authorization,
    ): Promise<Relationship> {
        // build the relationship
        const scope = authorization.authorizer.governingScope(request.object);
        const context = authorization.context();
        authorization.authorizer.validate(request, context.now);
        const relationship: Relationship = {
            id: schema.identifier("relationship").parse(`relationship-${v7()}`),
            object: request.object,
            ...Relationship.via(request),
            subject: request.subject,
            createdAt: context.now,
            expiresAt: request.expiresAt ?? null,
            ...(request.conditions === undefined ? {} : { conditions: request.conditions }),
        };

        // insert it unless the same relationship exists
        const row = {
            ...Relationship.encode(relationship, scope),
            ...Manager.values(authorization.manager),
        };
        const same = await authorization.transaction
            .read(accessRelationship)
            .findMany({ where: Relationship.equal(row), limit: 1 });
        if (same.length > 0) {
            throw new AccessError("CONFLICT", "relationship already exists");
        }
        await authorization.transaction.write({
            table: accessRelationship,
            before: null,
            after: Table.complete(accessRelationship, row),
        });

        return relationship;
    },
};

/** The relationships of a scope that `replaceRelationships` covers. */
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

/** Identify a relationship by its object, relation or role, subject, expiry and conditions. */
function identity(relationship: RelationshipRequest | Relationship): string {
    return JSON.stringify([
        ObjectReference.key(relationship.object),
        Relationship.viaColumns(relationship),
        Subject.key(relationship.subject),
        relationship.expiresAt ?? null,
        Object.entries(relationship.conditions ?? {}).toSorted(([left], [right]) =>
            left.localeCompare(right),
        ),
    ]);
}

/** Refuse removing the last owner binding on an object. */
async function requireRemainingOwner(
    transaction: Transaction,
    row: Select<typeof accessRelationship>,
): Promise<void> {
    // skip relations and other roles' bindings
    if (row.roleId !== OWNER_ROLE.id) {
        return;
    }

    // read the object's bindings of the owner role
    const owners = await transaction.read(accessRelationship).findMany({
        where: {
            objectPackageId: row.objectPackageId,
            objectType: row.objectType,
            objectScope: row.objectScope,
            objectId: row.objectId,
            roleId: OWNER_ROLE.id,
        },
    });

    // refuse removing the only such binding
    if (owners.length === 1 && nth(owners, 0).id === row.id) {
        throw new AccessError("CONFLICT", "the last owner cannot be removed");
    }
}
