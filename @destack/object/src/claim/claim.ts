import { ObjectError } from "../error/error.ts";
import {
    and,
    check,
    CHAIN_TERMS,
    type DatabaseConnection,
    defineTable,
    eq,
    index,
    inArray,
    integer,
    ne,
    or,
    type Condition,
    type RowImage,
    TABLE,
    text,
    uniqueIndex,
    type Row,
    type Snapshot,
    type SQL,
    type Transaction,
    type RelationalQueryBuilder,
    combine,
    textEnum,
} from "@destack/db";
import { canonicalize, Identifier, Instant, present, schema } from "@destack/schema";
import { Scope } from "@destack/sync";

import { ObjectKey, ObjectType, type ObjectIndex } from "../object/object.ts";

import type {} from "@destack/package/import-meta";

/** How long a reservation waits for its write to commit: a minute, above any write. */
export const RESERVATION_MILLISECONDS = 60_000;

/** The state of an object's committed names: active, or trashed while the object is trashed. */
export const ClaimState = schema.enum(["active", "trashed"]);
/** The state an object's committed names take. */
export type ClaimState = schema.Infer<typeof ClaimState>;

/** The states of a claim: pending while its write may fail, else its object's state. */
const CLAIM_STATES = ["pending", ...ClaimState.options] as const;

/** A name an object claims in a unique index across databases. */
const claim = schema.object({
    /** The index: the type's package, type and index name, or a shared namespace. */
    index: schema.string().min(1),
    /** The indexed values, with the scope they are unique within, as canonical JSON. */
    key: schema.string(),
    /** The package of the claiming object's type. */
    packageId: schema.string().min(1),
    /** The object claiming the name. */
    objectId: schema.string().min(1),
    /** The owner scope of the name: the object's scope, or its owner's for an object with one. */
    owner: schema.string().min(1),
});

/** The object an active name resolves to, with the owner scope of the name. */
export const ClaimOwner = claim.pick({ objectId: true, owner: true });
/** The object an active name resolves to, with the owner scope of the name. */
export type ClaimOwner = schema.Infer<typeof ClaimOwner>;

/** The answer of a claim call writing names: the names other objects claim. */
export const ClaimRefusal = schema.object({
    /** The names other objects claim. */
    refused: schema.array(claim),
});
/** The answer of a claim call writing names: the names other objects claim. */
export type ClaimRefusal = schema.Infer<typeof ClaimRefusal>;

/** The lookup of the objects active names resolve to. */
export interface ClaimLookup {
    /** Find the object an active name resolves to, none for a free or reserved name. */
    readonly owner: (index: string, key: string) => Promise<ClaimOwner | undefined>;
}

/** The lookups of the objects active names resolve to. */
export const ClaimLookup = {
    /** Look names up in a reader of the claims. */
    of(claims: RelationalQueryBuilder<typeof claimTable>): ClaimLookup {
        return { owner: (named, key) => Claim.owner(claims, { index: named, key }) };
    },
};

/** A name an object claims in a unique index across databases, with the keying verbs. */
export const Claim = Object.assign(claim, {
    /** Read the index of a type's unique index: its shared namespace, or the type's index. */
    index(type: ObjectType, name: string): string {
        return (
            type.indexes[name]?.namespace ??
            `${type.policy.definition.packageId}/${type.name}/${name}`
        );
    },

    /** Read a claim's indexed values from its key, without the scope it is unique within. */
    values(claimed: Pick<Claim, "key">): unknown[] {
        return schema.array(schema.unknown()).parse(JSON.parse(claimed.key)).slice(1);
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
            throw new TypeError(`index ${name} of ${type.name} needs a scope`);
        }

        return { index: Claim.index(type, name), key: canonicalize([within, ...values]) };
    },

    /** Read the owner scope of a row's names: its owner's when it has one, else its scope. */
    owning(type: ObjectType, row: Row): string {
        const column = type.ownerColumn;

        return column === undefined
            ? ObjectKey.parse(row).scope
            : schema.string().parse(row[column]);
    },

    /** List the names a row claims in its type's indexes, with enclosing scopes from a snapshot. */
    async list(type: ObjectType, row: Row, snapshot: Snapshot): Promise<Claim[]> {
        // record the names under the row's owner, or under its scope
        const { scope, id } = ObjectKey.parse(row);
        const owner = Claim.owning(type, row);

        // key each index the row has every value of
        const claims: Claim[] = [];
        for (const [name, declared] of Object.entries(type.indexes)) {
            // skip rows missing a value
            const values = declared.on.map((field) => row[field]);
            if (values.some((value) => value === null || value === undefined)) {
                continue;
            }

            // key the values within the scope the index is unique across
            const within = await Claim.enclosingScope(type, declared.across, scope, snapshot);
            claims.push({
                index: Claim.index(type, name),
                key: canonicalize([within, ...values]),
                packageId: type.policy.definition.packageId,
                objectId: id,
                owner,
            });
        }

        return claims;
    },

    /** Look up a type's object claiming values in an index, with the scope of the name. */
    async lookup(
        type: ObjectType,
        claims: ClaimLookup,
        name: string,
        values: readonly unknown[],
        scope?: string,
    ): Promise<{ readonly scope: string; readonly id: string } | undefined> {
        // find the owner of the keyed values
        const keyed = Claim.key(type, name, values, scope);
        const found = await claims.owner(keyed.index, keyed.key);

        return found === undefined ? undefined : { scope: found.owner, id: found.objectId };
    },

    /** Reserve an object's names until its write commits, returning the names others claim. */
    async reserve(
        database: DatabaseConnection,
        claimant: Claimant,
        requestId: string,
        now: number,
    ): Promise<Claim[]> {
        // insert the free names pending for a minute
        Claimant.require(claimant);
        const expiresAt = now + RESERVATION_MILLISECONDS;
        const inserted =
            claimant.claims.length === 0
                ? []
                : await database
                      .insert(claimTable)
                      .values(
                          claimant.claims.map((entry) => ({
                              ...entry,
                              id: Identifier.create("claim"),
                              scope: Scope.universe.id,
                              state: "pending" as const,
                              requestId,
                              expiresAt,
                          })),
                      )
                      .onConflictDoNothing()
                      .returning({ index: claimTable.index, key: claimTable.key });
        const existing = ClaimKey.missing(claimant.claims, inserted);

        // refuse the existing names other objects claim
        return existing.length === 0
            ? []
            : ClaimKey.refused(existing, await Claim.owners(database, existing));
    },

    /** Settle a committed write's names and release the names its object dropped. */
    async confirm(
        database: DatabaseConnection,
        claimant: Claimant,
        requestId: string,
    ): Promise<void> {
        // settle the request's pending names and the names the object claimed already
        Claimant.require(claimant);
        const settled = or(ne(claimTable.state, "pending"), eq(claimTable.requestId, requestId));
        await Claim.settle(database, claimant, settled);
        await Claim.drop(database, claimant);
    },

    /** Release an object's names a request with a failed write reserved. */
    async release(
        database: DatabaseConnection,
        object: Pick<Claimant, "objectId" | "owner">,
        requestId: string,
    ): Promise<void> {
        await database
            .delete(claimTable)
            .where(
                and(
                    eq(claimTable.requestId, requestId),
                    eq(claimTable.objectId, object.objectId),
                    eq(claimTable.owner, object.owner),
                    eq(claimTable.state, "pending"),
                ),
            );
    },

    /** Replace an object's names after an unreserved write, returning the names others claim. */
    async replace(database: DatabaseConnection, claimant: Claimant): Promise<Claim[]> {
        // refuse the names other objects claim before writing any
        Claimant.require(claimant);
        const found = await Claim.owners(database, claimant.claims);
        const refused = ClaimKey.refused(claimant.claims, found);
        if (refused.length > 0) {
            return refused;
        }

        // insert the new names, refusing those a concurrent write took meanwhile
        const fresh = claimant.claims.filter((entry) => !found.has(ClaimKey.of(entry)));
        const inserted =
            fresh.length === 0
                ? []
                : await database
                      .insert(claimTable)
                      .values(
                          fresh.map((entry) => ({
                              ...entry,
                              id: Identifier.create("claim"),
                              scope: Scope.universe.id,
                              state: claimant.state,
                              requestId: null,
                              expiresAt: null,
                          })),
                      )
                      .onConflictDoNothing()
                      .returning({ index: claimTable.index, key: claimTable.key });
        const lost = ClaimKey.missing(fresh, inserted);
        if (lost.length > 0) {
            return ClaimKey.refused(lost, await Claim.owners(database, lost));
        }

        // take over every reservation of the object and release the names it dropped
        await Claim.settle(database, claimant, undefined);
        await Claim.drop(database, claimant);

        return [];
    },

    /** Find the object an active name resolves to, none for a free or reserved name. */
    async owner(
        claims: RelationalQueryBuilder<typeof claimTable>,
        name: Pick<Claim, "index" | "key">,
    ): Promise<ClaimOwner | undefined> {
        const found = await claims.findFirst({
            where: { index: name.index, key: name.key, state: "active" },
        });

        return found === undefined ? undefined : { objectId: found.objectId, owner: found.owner };
    },

    /** List the keys of one index some object claims, active or reserved, in one read. */
    async claimed(
        claims: RelationalQueryBuilder<typeof claimTable>,
        names: { readonly index: string; readonly keys: readonly string[] },
    ): Promise<ReadonlySet<string>> {
        const rows = await claims.findMany({
            where: { index: names.index, key: { in: [...names.keys] } },
        });

        return new Set(rows.map((row) => row.key));
    },

    /** List the expired reservations of some indexes a reader has, and the next deadline. */
    async expired(
        claims: RelationalQueryBuilder<typeof claimTable>,
        indexes: readonly string[],
        now: number,
        filter: (rows: readonly ClaimRow[]) => Promise<readonly ClaimRow[]> = async (rows) => rows,
    ): Promise<Expiry> {
        // read the expired reservations of the indexes a chain's worth at a time, filtered for the reader
        const reserved: ClaimCondition = { state: "pending", index: { in: [...indexes] } };
        const expired = await filter(
            await claims.findMany({
                where: { AND: [reserved, { expiresAt: { lte: now } }] },
                orderBy: { expiresAt: "asc" },
                limit: CHAIN_TERMS,
            }),
        );

        // find the next deadline
        const next = await claims.findFirst({
            where: { AND: [reserved, { expiresAt: { gt: now } }] },
            orderBy: { expiresAt: "asc" },
        });
        const found = expired.map((row) => ({
            index: row.index,
            key: row.key,
            packageId: row.packageId,
            objectId: row.objectId,
            owner: row.owner,
        }));

        return next === undefined
            ? { claims: found }
            : { claims: found, next: present(next.expiresAt, "a pending claim's expiry") };
    },

    /** Read the objects that claim some names, by claim key. */
    async owners(
        database: DatabaseConnection,
        claims: readonly Claim[],
    ): Promise<Map<string, string>> {
        // read nothing for no names
        if (claims.length === 0) {
            return new Map();
        }

        // read the owners of the names in one read
        const rows = await database
            .select({ index: claimTable.index, key: claimTable.key, objectId: claimTable.objectId })
            .from(claimTable)
            .where(
                combine(
                    claims.map((entry) =>
                        and(eq(claimTable.index, entry.index), eq(claimTable.key, entry.key)),
                    ),
                    "OR",
                ),
            );

        return new Map(rows.map((row) => [ClaimKey.of(row), row.objectId]));
    },

    /** Settle an object's names in its state, only those a condition selects when given. */
    async settle(
        database: DatabaseConnection,
        object: Claimant,
        settling: SQL | undefined,
    ): Promise<void> {
        await database
            .update(claimTable)
            .set({ state: object.state, requestId: null, expiresAt: null })
            .where(
                and(
                    inArray(claimTable.index, [...object.indexes]),
                    eq(claimTable.objectId, object.objectId),
                    settling,
                    eq(claimTable.owner, object.owner),
                ),
            );
    },

    /** Release the names an object no longer claims. */
    async drop(database: DatabaseConnection, object: Claimant): Promise<void> {
        // skip a type without indexes
        if (object.indexes.length === 0) {
            return;
        }

        // delete the object's names other than those it claims
        await database
            .delete(claimTable)
            .where(
                and(
                    inArray(claimTable.index, [...object.indexes]),
                    eq(claimTable.objectId, object.objectId),
                    eq(claimTable.owner, object.owner),
                    ...object.claims.map((entry) =>
                        or(ne(claimTable.index, entry.index), ne(claimTable.key, entry.key)),
                    ),
                ),
            );
    },

    /** Find the scope an index is unique across, from the scope an object lives in. */
    async enclosingScope(
        type: ObjectType,
        across: ObjectIndex["across"],
        scope: string,
        snapshot: Snapshot,
    ): Promise<string> {
        // key universe-wide indexes by the universe, and indexes across the object's scope by it
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
            throw new ObjectError("NO_ENCLOSING_SCOPE", {
                message: `scope ${scope} has no enclosing ${name}`,
            });
        }

        return enclosing.object.id;
    },

    /** Decide whether a row has a value in a column its type may lack, such as a deletion. */
    isSet(row: Row, column: string): boolean {
        return row[column] !== undefined && row[column] !== null;
    },
});

/** A condition over the claims a database has. */
export type ClaimCondition = Condition<RowImage<typeof claimTable>>;

/** A claim row as a database stores it. */
export type ClaimRow = RowImage<typeof claimTable>;

/** A name an object claims in a unique index across databases. */
export type Claim = schema.Infer<typeof claim>;

/** An object and the names it claims in its type's indexes after a write. */
const claimant = schema.object({
    /** The indexes the object's type declares. */
    indexes: schema.array(schema.string().min(1)),
    /** The object. */
    objectId: schema.string().min(1),
    /** The owner scope of the object's names. */
    owner: schema.string().min(1),
    /** The names it claims. */
    claims: schema.array(claim),
    /** The state its names take. */
    state: ClaimState,
});
/** An object and its claimed names after a write, with the verbs reading them. */
export const Claimant = Object.assign(claimant, {
    /** Read an object's claimed names after a write: active, trashed, or none once purged. */
    async of(
        type: ObjectType,
        object: Pick<Claimant, "objectId" | "owner">,
        row: Row | undefined,
        snapshot: Snapshot,
    ): Promise<Claimant> {
        const isGone = row === undefined || Claim.isSet(row, "purgedAt");

        return {
            indexes: Object.keys(type.indexes).map((name) => Claim.index(type, name)),
            ...object,
            claims: isGone ? [] : await Claim.list(type, row, snapshot),
            state: row !== undefined && Claim.isSet(row, "deletedAt") ? "trashed" : "active",
        };
    },

    /** Read the names each object a transaction wrote claims, by its last image. */
    async written(transaction: Transaction, types: readonly ObjectType[]): Promise<Claimant[]> {
        // take each written object's last image
        const indexed = types.filter((type) => Object.keys(type.indexes).length > 0);
        const byTable = new Map(indexed.map((type) => [type.table[TABLE].sqlName, type]));
        const written = transaction.changes.changes.flatMap((change) => {
            const type = byTable.get(change.table[TABLE].sqlName);
            const image = present(change.after ?? change.before, "the row a change writes");

            return type === undefined ? [] : [{ type, image, row: change.after ?? undefined }];
        });

        // read each object's names once
        const snapshot = transaction.view();
        const claimants: Claimant[] = [];
        for (const { type, image, row } of written) {
            const object = {
                objectId: ObjectKey.parse(image).id,
                owner: Claim.owning(type, image),
            };
            claimants.push(await Claimant.of(type, object, row, snapshot));
        }

        return claimants;
    },

    /** Refuse a claimant with a name of another object or owner. */
    require(claimed: Claimant): void {
        const stranger = claimed.claims.find(
            (entry) => entry.objectId !== claimed.objectId || entry.owner !== claimed.owner,
        );
        if (stranger !== undefined) {
            throw new ObjectError("FOREIGN_CLAIM", {
                message: `claim ${stranger.index} is not a name of ${claimed.objectId} in ${claimed.owner}`,
            });
        }
    },
});
/** An object and the names it claims in its type's indexes after a write. */
export type Claimant = schema.Infer<typeof claimant>;

/** The expired reservations of some indexes and the next deadline. */
export const Expiry = schema.object({
    /** The expired reservations. */
    claims: schema.array(claim),
    /** When the next reservation expires, in UTC epoch milliseconds. */
    next: Instant.exactOptional(),
});
/** The expired reservations of some indexes and the next deadline. */
export type Expiry = schema.Infer<typeof Expiry>;

/** Every claimed name: pending during its write, trashed with its object, else active. */
export const claimTable = defineTable(
    "claim",
    {
        /** The claim's identifier. */
        id: text("id").notNull(),
        /** The universe, where every claim lives. */
        scope: text("scope").notNull(),
        /** The index: the type's package, type and index name, or a shared namespace. */
        index: text("index").primaryKey(),
        /** The indexed values, with the scope they are unique within, as canonical JSON. */
        key: text("key").primaryKey(),
        /** The package of the claiming object's type. */
        packageId: text("package_id").notNull(),
        /** The object claiming the name. */
        objectId: text("object_id").notNull(),
        /** The owner scope of the name. */
        owner: text("owner").notNull(),
        /** Whether the claim is pending, active or trashed. */
        state: textEnum("state", CLAIM_STATES).notNull(),
        /** The request whose write reserved the name, null once the write committed. */
        requestId: text("request_id"),
        /** The pending write's deadline in UTC epoch milliseconds, null once committed. */
        expiresAt: integer("expires_at"),
    },
    {
        constraints: (columns) => [
            uniqueIndex("claim_id").on(columns.id),
            index("claim_request").on(columns.requestId),
            index("claim_object").on(columns.objectId, columns.index),
            index("claim_owner").on(columns.owner),
            index("claim_expiry").on(columns.expiresAt),
            check("claim_universe", { scope: Scope.universe.id }),
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

/** The comparisons of claimed names by index and key. */
const ClaimKey = {
    /** Key a claimed name by its index and key. */
    of(entry: { readonly index: string; readonly key: string }): string {
        return JSON.stringify([entry.index, entry.key]);
    },

    /** List the claims a write left out of the names it inserted. */
    missing(
        claims: readonly Claim[],
        inserted: readonly { readonly index: string; readonly key: string }[],
    ): Claim[] {
        const written = new Set(inserted.map((entry) => ClaimKey.of(entry)));

        return claims.filter((entry) => !written.has(ClaimKey.of(entry)));
    },

    /** List the claims whose names another object claims. */
    refused(claims: readonly Claim[], found: ReadonlyMap<string, string>): Claim[] {
        return claims.filter((entry) => {
            const objectId = found.get(ClaimKey.of(entry));

            return objectId !== undefined && objectId !== entry.objectId;
        });
    },
};
