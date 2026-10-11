import {
    check,
    defineTable,
    index,
    integer,
    TABLE,
    text,
    uniqueIndex,
    type Row,
    type Snapshot,
    type Transaction,
} from "@destack/db";
import {
    canonicalize,
    defineSchema,
    type Identifier,
    Instant,
    present,
    schema,
} from "@destack/schema";
import { type ObjectReference, Scope } from "@destack/sync";
import { ServiceError } from "@destack/service/error";
import { ObjectKey, ObjectType, type IdentityOf, type ObjectIndex } from "../object/object.ts";
import type { ClaimStore } from "./store.ts";

/** How long a reservation waits for its write to commit, in milliseconds: a minute, above any write. */
export const RESERVATION_MILLISECONDS = 60_000;

/** The state an object's committed names take: active, or trashed while the object sits in the trash. */
export const ClaimState = defineSchema(schema.enum(["active", "trashed"]));
/** The state an object's committed names take. */
export type ClaimState = schema.Infer<typeof ClaimState>;

/** The states of a claim: pending while its write may still fail, else its object's committed state. */
const CLAIM_STATES = ["pending", ...ClaimState.options] as const;

/** A name an object claims in a unique index across databases. */
const claim = defineSchema(
    schema.object({
        /** The index: the object type's package, type and index name, or a namespace types share. */
        index: schema.string().min(1),
        /** The indexed values, with the scope they are unique within, as canonical JSON. */
        key: schema.string(),
        /** The package of the claiming object's type. */
        packageId: schema.string().min(1),
        /** The object claiming the name. */
        objectId: schema.string().min(1),
        /** The scope the object lives in. */
        scope: schema.string().min(1),
    }),
);
/** A name an object claims in a unique index across databases, with the verbs keying an object type's names. */
export const Claim = Object.assign(claim, {
    /** Read the index one of a type's unique indexes claims in: its shared namespace, or the type's own. */
    index(type: ObjectType, name: string): string {
        return (
            type.indexes[name]?.namespace ??
            `${type.policy.definition.packageId}/${type.name}/${name}`
        );
    },

    /** Key values in one of a type's indexes, within the scope the index is unique across. */
    key(
        type: ObjectType,
        name: string,
        values: readonly unknown[],
        scope?: string,
    ): Pick<Claim, "index" | "key"> {
        // require the index and a needed scope
        const declared = type.indexes[name];
        if (declared === undefined) {
            throw new TypeError(`object ${type.name} has no index ${name}`);
        }
        const within = declared.across() instanceof ObjectType ? scope : Scope.universe.id;
        if (within === undefined) {
            throw new TypeError(`index ${name} of ${type.name} keys values within a scope`);
        }

        return { index: Claim.index(type, name), key: canonicalize([within, ...values]) };
    },

    /** List the names a row claims in its type's indexes, with enclosing scopes from a snapshot. */
    async list(type: ObjectType, row: Row, snapshot: Snapshot): Promise<Claim[]> {
        // key each index the row has every value of
        const { scope, id } = ObjectKey.parse(row);
        const claims: Claim[] = [];
        for (const [name, declared] of Object.entries(type.indexes)) {
            // skip rows missing a value
            const values = declared.on.map((field) => row[field]);
            if (values.some((value) => value === null || value === undefined)) {
                continue;
            }

            // key the values within the scope the index is unique across
            const within = await enclosingScope(type, declared.across, scope, snapshot);
            claims.push({
                index: Claim.index(type, name),
                key: canonicalize([within, ...values]),
                packageId: type.policy.definition.packageId,
                objectId: id,
                scope,
            });
        }

        return claims;
    },

    /** Look up the object of a type owning values in one of its indexes. */
    async lookup<Type extends ObjectType>(
        type: Type,
        claims: Pick<ClaimStore, "owner">,
        name: string,
        values: readonly unknown[],
        scope?: string,
    ): Promise<(ObjectReference & { readonly id: Identifier<IdentityOf<Type>> }) | undefined> {
        // find the owner of the keyed values
        const keyed = Claim.key(type, name, values, scope);
        const owner = await claims.owner(keyed.index, keyed.key);

        return owner === undefined
            ? undefined
            : {
                  ...type.reference(owner.scope, owner.objectId),
                  id: type.identifier(owner.objectId),
              };
    },
});
/** A name an object claims in a unique index across databases. */
export type Claim = schema.Infer<typeof claim>;

/** An object and the names it claims in its type's indexes after a write. */
const claimant = defineSchema(
    schema.object({
        /** The indexes the object's type declares. */
        indexes: schema.array(schema.string().min(1)),
        /** The object. */
        objectId: schema.string().min(1),
        /** The names it claims. */
        claims: schema.array(Claim),
        /** The state its names take. */
        state: ClaimState,
    }),
);
/** An object and the names it claims in its type's indexes after a write, with the verbs reading them. */
export const Claimant = Object.assign(claimant, {
    /** Read the names an object claims after a write: active, trashed while it sits in the trash, none once purged. */
    async of(
        type: ObjectType,
        objectId: string,
        row: Row | undefined,
        snapshot: Snapshot,
    ): Promise<Claimant> {
        const isGone = row === undefined || isSet(row, "purgedAt");

        return {
            indexes: Object.keys(type.indexes).map((name) => Claim.index(type, name)),
            objectId,
            claims: isGone ? [] : await Claim.list(type, row, snapshot),
            state: row !== undefined && isSet(row, "deletionRequestedAt") ? "trashed" : "active",
        };
    },

    /** Read the names each object a transaction wrote claims, by its last image. */
    async written(transaction: Transaction, types: readonly ObjectType[]): Promise<Claimant[]> {
        // keep each written object's last image
        const indexed = types.filter((type) => Object.keys(type.indexes).length > 0);
        const byTable = new Map(indexed.map((type) => [type.table[TABLE].sqlName, type]));
        const written = transaction.changes.changes.flatMap((change) => {
            const type = byTable.get(change.table[TABLE].sqlName);
            const image = present(change.after ?? change.before, "the row a change writes");

            return type === undefined
                ? []
                : [
                      {
                          type,
                          objectId: schema.string().parse(image["id"]),
                          row: change.after ?? undefined,
                      },
                  ];
        });

        // read each object's names once
        const snapshot = transaction.view();
        const claimants: Claimant[] = [];
        for (const { type, objectId, row } of written) {
            claimants.push(await Claimant.of(type, objectId, row, snapshot));
        }

        return claimants;
    },
});
/** An object and the names it claims in its type's indexes after a write. */
export type Claimant = schema.Infer<typeof claimant>;

/** The expired reservations of some indexes and the next deadline. */
export const Expiry = defineSchema(
    schema.object({
        /** The expired reservations. */
        claims: schema.array(Claim),
        /** When the next reservation expires, in UTC epoch milliseconds. */
        next: Instant.exactOptional(),
    }),
);
/** The expired reservations of some indexes and the next deadline. */
export type Expiry = schema.Infer<typeof Expiry>;

/** Every claimed name: pending during its write, trashed while its object sits in the trash, active otherwise. */
export const claimTable = defineTable(
    "claim",
    {
        /** The claim's identifier. */
        id: text("id").notNull(),
        /** The index: the object type's package, type and index name, or a namespace types share. */
        index: text("index").primaryKey(),
        /** The indexed values, with the scope they are unique within, as canonical JSON. */
        key: text("key").primaryKey(),
        /** The package of the claiming object's type. */
        packageId: text("package_id").notNull(),
        /** The object claiming the name. */
        objectId: text("object_id").notNull(),
        /** The scope the object lives in. */
        scope: text("scope").notNull(),
        /** Whether the claim is pending, active or trashed. */
        state: text("state", { enum: CLAIM_STATES }).notNull(),
        /** The request whose write reserved the name, null once the write committed. */
        requestId: text("request_id"),
        /** The deadline of a pending claim's write, in UTC epoch milliseconds, null once the write committed. */
        expiresAt: integer("expires_at"),
    },
    {
        constraints: (columns) => [
            uniqueIndex("claim_id").on(columns.id),
            index("claim_request").on(columns.requestId),
            index("claim_object").on(columns.index, columns.objectId),
            index("claim_expiry").on(columns.expiresAt),
            check("claim_pending", {
                AND: [
                    {
                        OR: [
                            { state: "pending", expiresAt: { isNotNull: true } },
                            { state: { ne: "pending" }, expiresAt: { isNull: true } },
                        ],
                    },
                    {
                        OR: [
                            { state: "pending", requestId: { isNotNull: true } },
                            { state: { ne: "pending" }, requestId: { isNull: true } },
                        ],
                    },
                ],
            }),
        ],
        log: {},
    },
);

/** Find the scope an index is unique across, from the scope an object lives in. */
async function enclosingScope(
    type: ObjectType,
    across: ObjectIndex["across"],
    scope: string,
    snapshot: Snapshot,
): Promise<string> {
    // key universe-wide indexes by the universe, and indexes across the object's own scope by it
    const enclosingType = across();
    if (!(enclosingType instanceof ObjectType)) {
        return Scope.universe.id;
    } else if (enclosingType.same(type.ancestors[0])) {
        return scope;
    }

    // find the enclosing scope of the declared type
    const { packageId, name } = enclosingType.policy.definition;
    const chain = await Scope.chain(snapshot, scope);
    const enclosing = chain.find(
        (link) => link.object.packageId === packageId && link.object.type === name,
    );
    if (enclosing === undefined) {
        throw new ServiceError("PRECONDITION_FAILED", {
            message: `scope ${scope} has no enclosing ${name}`,
        });
    }

    return enclosing.object.id;
}

/** Decide whether a row has a value in a column its type may lack, such as a deletion's time. */
function isSet(row: Row, column: string): boolean {
    return row[column] !== undefined && row[column] !== null;
}
