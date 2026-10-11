import {
    Change,
    and,
    eq,
    gt,
    inArray,
    lte,
    min,
    ne,
    or,
    type DatabaseConnection,
    ReadCache,
    CHAIN_TERMS,
    type SQL,
} from "@destack/db";
import { aligned, type Clock, Identifier, present } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { ObjectType } from "../object/object.ts";
import {
    Claim,
    claimTable,
    ClaimState,
    type Expiry,
    type Claimant,
    RESERVATION_MILLISECONDS,
} from "./claim.ts";

/** The most claim owners a store keeps: about 3 MiB at 200 bytes an entry. */
const CAPACITY = 16_384;

/** The names indexed objects claim across databases, reserved while a write runs and settled once it commits. */
export interface ClaimStore {
    /** Reserve the names an open transaction's objects claim until the write commits. */
    reserve(
        transaction: DatabaseConnection,
        claimants: readonly Claimant[],
        requestId: string,
    ): Promise<Reservation>;

    /** Release the reservations of a request with a failed write. */
    release(requestId: string): Promise<void>;

    /** Replace an object's names after a write without reservations, returning the names other objects own. */
    replace(claimant: Claimant): Promise<readonly Claim[]>;

    /** Find the object an active name resolves to, none for a reserved name. */
    owner(index: string, key: string): Promise<Pick<Claim, "objectId" | "scope"> | undefined>;

    /** List the expired reservations of some indexes, and the next deadline. */
    expired(indexes: readonly string[]): Promise<Expiry>;
}

/** The names a write reserved: those other objects own, and the confirmation once the write committed. */
export interface Reservation {
    /** The names other objects own. */
    readonly refused: readonly Claim[];

    /** Settle the reserved names once the write committed. */
    confirm(): Promise<void>;
}

/** The reservation with nothing to confirm. */
const NONE: Reservation = { refused: [], confirm: async () => {} };

/** The reservations of writes claiming no names, or claiming them in their own transaction. */
export const Reservation = {
    /** The reservation with nothing to confirm. */
    none: NONE,

    /** Reserve the names written objects claim in the committing transaction, refusing names other objects own. */
    async reserve(
        claims: ClaimStore,
        committing: DatabaseConnection,
        claimants: readonly Claimant[],
        types: readonly ObjectType[],
        requestId: string,
    ): Promise<Reservation> {
        // reserve none for a write of no indexed object
        const reservation =
            claimants.length === 0 ? NONE : await claims.reserve(committing, claimants, requestId);

        // refuse names other objects own
        Reservation.refuse(types, reservation.refused);

        return reservation;
    },

    /** Refuse a write whose names other objects own, reporting the index of the first. */
    refuse(types: readonly ObjectType[], taken: readonly Claim[]): void {
        const [first] = taken;
        if (first !== undefined) {
            const [name] = types.flatMap((type) =>
                Object.keys(type.indexes).filter(
                    (index) => Claim.index(type, index) === first.index,
                ),
            );
            throw new ServiceError("CONFLICT", {
                message: `${present(name, `the index of claim ${first.index}`)} is taken`,
            });
        }
    },
};

/** The claims kept in a database. */
export class DatabaseClaimStore implements ClaimStore {
    /** The database with the claims. */
    readonly database: DatabaseConnection;
    /** The clock reservations expire by. */
    readonly clock: Clock;
    /** Claim owners by index and key. */
    readonly #ownerCache = new ReadCache<Pick<Claim, "objectId" | "scope"> | undefined>(CAPACITY);
    /** Whether the log is followed, which keeps the owner reads current. */
    #isFollowing = false;

    /** Keep the claims of a database on a clock. */
    constructor(database: DatabaseConnection, clock: Clock) {
        this.database = database;
        this.clock = clock;
    }

    /** Cache owner reads while following the log until the signal aborts. */
    async follow(signal: AbortSignal): Promise<void> {
        this.#isFollowing = true;
        try {
            await this.database.log.invalidate(
                [claimTable],
                {
                    clear: () => this.#ownerCache.clear(),
                    forget: (change) => {
                        for (const image of [Change.before(change), Change.after(change)]) {
                            if (image !== null) {
                                this.#ownerCache.forget(Claims.key(image));
                            }
                        }
                    },
                },
                signal,
            );
        } finally {
            this.#isFollowing = false;
            this.#ownerCache.clear();
        }
    }

    /** Claim the names in a transaction on this database, else reserve them until the write commits. */
    async reserve(
        transaction: DatabaseConnection,
        claimants: readonly Claimant[],
        requestId: string,
    ): Promise<Reservation> {
        // claim in the write's own transaction
        if (transaction.state === this.database.state) {
            const within = new DatabaseClaimStore(transaction, this.clock);
            const refused: Claim[] = [];
            for (const claimant of claimants) {
                refused.push(...(await within.replace(claimant)));
            }

            return { ...Reservation.none, refused };
        }
        // reserve in this database until the write elsewhere commits
        else {
            const refused = await this.claim(
                claimants.flatMap((entry) => entry.claims),
                requestId,
            );

            return { refused, confirm: () => this.confirm(requestId, claimants) };
        }
    }

    /** Reserve a request's claims in chunks until its write commits, returning the names other objects own. */
    async claim(claims: readonly Claim[], requestId: string): Promise<readonly Claim[]> {
        // reserve each chunk for a minute
        const expiresAt = this.clock.now() + RESERVATION_MILLISECONDS;
        const refused: Claim[] = [];
        for (let start = 0; start < claims.length; start += CHAIN_TERMS) {
            const chunk = claims.slice(start, start + CHAIN_TERMS);
            refused.push(...(await this.#reserve(chunk, requestId, expiresAt)));
        }

        return refused;
    }

    /** Settle a committed write's names and release the names its objects dropped. */
    async confirm(requestId: string, claimants: readonly Claimant[]): Promise<void> {
        // settle the request's pending claims and the names the objects claimed already
        const settled = or(ne(claimTable.state, "pending"), eq(claimTable.requestId, requestId));
        await this.#settle(claimants, settled);
        await this.#release(claimants);
    }

    /** Release the reservations of a request with a failed write. */
    async release(requestId: string): Promise<void> {
        await this.database
            .delete(claimTable)
            .where(and(eq(claimTable.requestId, requestId), eq(claimTable.state, "pending")));
    }

    /** Replace an object's names after a write that reserved none, returning the names other objects own. */
    async replace(claimant: Claimant): Promise<readonly Claim[]> {
        // refuse the names other objects own before writing any
        const owners = await this.#owners(claimant.claims);
        const refused = Claims.refused(claimant.claims, owners);
        if (refused.length > 0) {
            return refused;
        }

        // insert the new names, refusing those a concurrent write took meanwhile
        const fresh = claimant.claims.filter((entry) => !owners.has(Claims.key(entry)));
        for (let start = 0; start < fresh.length; start += CHAIN_TERMS) {
            const chunk = fresh.slice(start, start + CHAIN_TERMS);
            const inserted = await this.database
                .insert(claimTable)
                .values(
                    chunk.map((entry) => ({
                        ...entry,
                        id: Identifier.create("claim"),
                        state: claimant.state,
                        requestId: null,
                        expiresAt: null,
                    })),
                )
                .onConflictDoNothing()
                .returning({ index: claimTable.index, key: claimTable.key });
            const lost = Claims.missing(chunk, inserted);
            if (lost.length > 0) {
                return Claims.refused(lost, await this.#owners(lost));
            }
        }

        // take over every reservation of the object and release the names it dropped
        await this.#settle([claimant], undefined);
        await this.#release([claimant]);

        return [];
    }

    /** Find the object an active name resolves to. */
    owner(index: string, key: string): Promise<Pick<Claim, "objectId" | "scope"> | undefined> {
        const read = async () => {
            const [found] = await this.database
                .select({ objectId: claimTable.objectId, scope: claimTable.scope })
                .from(claimTable)
                .where(
                    and(
                        eq(claimTable.index, index),
                        eq(claimTable.key, key),
                        eq(claimTable.state, "active"),
                    ),
                );

            return found;
        };

        return this.#isFollowing ? this.#ownerCache.get(Claims.key({ index, key }), read) : read();
    }

    /** List the keys of one index some object claims, active or reserved, in one read. */
    static async claimed(
        database: DatabaseConnection,
        index: string,
        keys: readonly string[],
    ): Promise<ReadonlySet<string>> {
        const rows = await database
            .select({ key: claimTable.key })
            .from(claimTable)
            .where(and(eq(claimTable.index, index), inArray(claimTable.key, [...keys])));

        return new Set(rows.map((row) => row.key));
    }

    /** List the expired reservations of some indexes, and the next deadline. */
    async expired(indexes: readonly string[]): Promise<Expiry> {
        // read the expired reservations of the indexes
        const now = this.clock.now();
        const reserved = and(
            eq(claimTable.state, "pending"),
            inArray(claimTable.index, [...indexes]),
        );
        const claims = await this.database
            .select({
                index: claimTable.index,
                packageId: claimTable.packageId,
                key: claimTable.key,
                objectId: claimTable.objectId,
                scope: claimTable.scope,
            })
            .from(claimTable)
            .where(and(reserved, lte(claimTable.expiresAt, now)));

        // find the next deadline
        const next = aligned(
            await this.database
                .select({ expiresAt: min(claimTable.expiresAt) })
                .from(claimTable)
                .where(and(reserved, gt(claimTable.expiresAt, now))),
            0,
        );

        return next.expiresAt === null ? { claims } : { claims, next: next.expiresAt };
    }

    /** Reserve a chunk of claims in one insert, returning the names other objects own. */
    async #reserve(
        claims: readonly Claim[],
        requestId: string,
        expiresAt: number,
    ): Promise<readonly Claim[]> {
        // insert the free names
        const inserted = await this.database
            .insert(claimTable)
            .values(
                claims.map((entry) => ({
                    ...entry,
                    id: Identifier.create("claim"),
                    state: "pending" as const,
                    requestId,
                    expiresAt,
                })),
            )
            .onConflictDoNothing()
            .returning({ index: claimTable.index, key: claimTable.key });
        const existing = Claims.missing(claims, inserted);

        // refuse the existing names other objects own
        return existing.length === 0 ? [] : Claims.refused(existing, await this.#owners(existing));
    }

    /** Read the objects owning some names, by claim key. */
    async #owners(claims: readonly Claim[]): Promise<Map<string, string>> {
        const rows = await this.database
            .select({ index: claimTable.index, key: claimTable.key, objectId: claimTable.objectId })
            .from(claimTable)
            .where(
                or(
                    ...claims.map((entry) =>
                        and(eq(claimTable.index, entry.index), eq(claimTable.key, entry.key)),
                    ),
                ),
            );

        return new Map(rows.map((row) => [Claims.key(row), row.objectId]));
    }

    /** Settle each object's names in its state, only those a condition keeps when given. */
    async #settle(claimants: readonly Claimant[], settling: SQL | undefined): Promise<void> {
        for (const state of ClaimState.options) {
            const settled = claimants.filter((entry) => entry.state === state);
            for (let start = 0; start < settled.length; start += CHAIN_TERMS) {
                const chunk = settled.slice(start, start + CHAIN_TERMS);
                await this.database
                    .update(claimTable)
                    .set({ state, requestId: null, expiresAt: null })
                    .where(
                        and(
                            settling,
                            or(
                                ...chunk.map(({ indexes, objectId }) =>
                                    and(
                                        inArray(claimTable.index, [...indexes]),
                                        eq(claimTable.objectId, objectId),
                                    ),
                                ),
                            ),
                        ),
                    );
            }
        }
    }

    /** Release the names objects no longer claim. */
    async #release(claimants: readonly Claimant[]): Promise<void> {
        for (let start = 0; start < claimants.length; start += CHAIN_TERMS) {
            const chunk = claimants.slice(start, start + CHAIN_TERMS);
            await this.database
                .delete(claimTable)
                .where(
                    or(
                        ...chunk.map(({ indexes, objectId, claims }) =>
                            and(
                                inArray(claimTable.index, [...indexes]),
                                eq(claimTable.objectId, objectId),
                                ...claims.map((entry) =>
                                    or(
                                        ne(claimTable.index, entry.index),
                                        ne(claimTable.key, entry.key),
                                    ),
                                ),
                            ),
                        ),
                    ),
                );
        }
    }
}

/** The comparisons of claimed names. */
const Claims = {
    /** Key a claimed name by its index and key. */
    key(entry: { readonly index: string; readonly key: string }): string {
        return JSON.stringify([entry.index, entry.key]);
    },

    /** List the claims a write left out of the names it inserted. */
    missing(
        claims: readonly Claim[],
        inserted: readonly { readonly index: string; readonly key: string }[],
    ): Claim[] {
        const written = new Set(inserted.map((entry) => Claims.key(entry)));

        return claims.filter((entry) => !written.has(Claims.key(entry)));
    },

    /** List the claims whose names another object owns. */
    refused(claims: readonly Claim[], owners: ReadonlyMap<string, string>): Claim[] {
        return claims.filter((entry) => {
            const owner = owners.get(Claims.key(entry));

            return owner !== undefined && owner !== entry.objectId;
        });
    },
};
