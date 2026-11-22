import { ObjectError } from "../error/error.ts";
import { AccessContext, Caller } from "@destack/access";
import {
    and,
    type Controller,
    defineTable,
    eq,
    index,
    integer,
    now,
    type Row,
    Snapshot,
    TABLE,
    text,
    type Participant,
    type Transaction,
} from "@destack/db";
import { type UuidEnvironment, present, schema, Uuid } from "@destack/schema";

import { RequestId } from "@destack/sync";
import type * as sync from "@destack/sync";
import type { ObjectStore } from "../server/store.ts";
import { ObjectKey, type ObjectType } from "../object/object.ts";
import { Condition } from "../object/condition.ts";
import { Claim, Claimant, ClaimRefusal, RESERVATION_MILLISECONDS } from "./claim.ts";
import type { ClaimType } from "./method.ts";
import { ObjectRemote } from "../replica/remote.ts";

import type {} from "@destack/package/import-meta";
import { TelemetryScope } from "@destack/telemetry";

/** The ready reason of a controlled object one of whose names another object holds. */
export const NAME_TAKEN = "NameTaken";

/** The ready reason of a controlled object holding its names again. */
const APPLIED = "Applied";

/** The store a reservation takes names for: its claims, types, database and principal. */
type ReservingStore = Pick<
    ObjectStore,
    "claims" | "objects" | "database" | "environment" | "principal"
>;

/** A store serving object types elsewhere, called as the claiming store. */
export interface RemoteStore {
    /** Run a mutation's calls there in one transaction, answering their results in order. */
    readonly mutate: (mutation: sync.Mutation) => Promise<readonly unknown[]>;
    /** Run a reading call there as the calling store. */
    readonly query: (call: sync.Call) => Promise<unknown>;
}

/** The stores elsewhere a store calls. */
export const RemoteStore = {
    /** Call a store elsewhere in one scope through a remote. */
    of(remote: ObjectRemote, scope: string): RemoteStore {
        return {
            mutate: (mutation) => ObjectRemote.mutate(remote, scope, mutation),
            query: (call) => remote.call(scope, call),
        };
    },

    /** Call a store served in this process as a principal. */
    served(
        store: Pick<ObjectStore, "mutate" | "query" | "clock">,
        principal: sync.Subject,
    ): RemoteStore {
        // run calls as the principal
        const caller = () => AccessContext.of(Caller.of(principal), store.clock.now());

        return {
            mutate: (mutation) => store.mutate(caller(), mutation),
            query: (call) => store.query(caller(), call),
        };
    },
};

/** The names a write reserved at the claims' home, kept here until it confirms or releases them. */
export const claimReservation = defineTable(
    "claim_reservation",
    {
        /** The write's request. */
        requestId: text("request_id").primaryKey(),
        /** The object whose names the write reserved. */
        objectId: text("object_id").primaryKey(),
        /** The owner scope of the names. */
        owner: text("owner").notNull(),
        /** One of the object type's indexes, which finds the type. */
        index: text("index").notNull(),
        /** When the home's reservation expires, in UTC epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
    },
    { constraints: (columns) => [index("claim_reservation_expiry").on(columns.expiresAt)] },
);

/** The claim type recording the names of indexed objects, and the store serving it, its primary. */
export interface ClaimPrimary {
    /** The claim type, whose methods reserve, confirm, release and replace names. */
    readonly object: ClaimType;
    /** The store serving the claim type, absent when the claiming store serves it itself. */
    readonly home?: RemoteStore;
}

/** A write's hold on the names its rows change: taken before its commit, settled after it. */
export class Reservation implements Participant {
    /** The transaction whose written rows the reservation reads. */
    readonly #transaction: Transaction;
    /** The store whose types and claims the reservation takes names for. */
    readonly #store: ReservingStore;
    /** The request the names are reserved under at the claims' home. */
    readonly #id: string;
    /** The objects whose names the reservation holds at the claims' home. */
    #reserved: readonly Claimant[] = [];

    /** Hold the names of a store's rows a transaction writes. */
    constructor(transaction: Transaction, store: ReservingStore) {
        this.#transaction = transaction;
        this.#store = store;
        this.#id = Uuid.v4(store.environment.random);
    }

    /** Enlist a store's reservation in a transaction's commit, once for each store. */
    static enlist(transaction: Transaction, store: ReservingStore): void {
        transaction.enlist(
            `destack.object.reservation:${store.principal.id}`,
            new Reservation(transaction, store),
        );
    }

    /** Take the names the written rows change: queued, written here, or reserved at their home. */
    async prepare(): Promise<void> {
        // take the rows whose names changed, none for most writes
        const { claims, objects, environment } = this.#store;
        const claimants = await Claimant.written(this.#transaction, objects);
        if (claims === undefined || claimants.length === 0) {
            return;
        }
        const { home } = claims;

        // write the names with the commit where the store has the claims itself
        if (home === undefined) {
            this.#transaction.record(async (committing) => {
                const refused = await Promise.all(
                    claimants.map((claimant) => Claim.replace(committing, claimant, environment)),
                );
                Reservation.refuse(objects, refused.flat());
            });

            return;
        }

        // queue the names of controlled types in this transaction, which their controller claims at the home
        const types = Reservation.#types(objects);
        const isQueued = (claimant: Claimant) =>
            claimant.indexes.some((name) => types.get(name)?.isControlled === true);
        const queued = this.#rows(claimants.filter(isQueued), environment.clock.now());
        if (queued.length > 0) {
            this.#transaction.record(async (committing) => {
                await committing.insert(claimReservation).values(queued).onConflictDoNothing();
            });
        }

        // reserve every other name at the home before the commit
        const scope = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
        await scope.span("destack.claim.reserve", {}, () =>
            this.#reserve(
                claims.object,
                home,
                claimants.filter((each) => !isQueued(each)),
            ),
        );
    }

    /** Confirm the reserved names at their home once the commit stands. */
    async confirm(): Promise<void> {
        // confirm nothing for a write that reserved no name at a home
        const { claims, environment } = this.#store;
        if (claims?.home === undefined || this.#reserved.length === 0) {
            return;
        }

        // confirm every reserved object's names in one request, then forget the rows here
        await claims.home.mutate({
            id: RequestId.create(environment),
            calls: this.#reserved.map((claimant) =>
                claims.object.call("confirm", { ...claimant, request: this.#id }),
            ),
        });
        await this.#forget();
    }

    /** Release the reserved names at their home once the commit failed. */
    async cancel(): Promise<void> {
        // release nothing for a write that reserved no name at a home
        const { claims, environment } = this.#store;
        if (claims?.home === undefined || this.#reserved.length === 0) {
            return;
        }

        // release the claiming objects' names in one request, then forget the rows here
        const claiming = this.#reserved.filter((claimant) => claimant.claims.length > 0);
        if (claiming.length > 0) {
            await claims.home.mutate({
                id: RequestId.create(environment),
                calls: claiming.map(({ objectId, owner }) =>
                    claims.object.call("release", { objectId, owner, request: this.#id }),
                ),
            });
        }
        await this.#forget();
    }

    /** Reserve objects' names at the claims' home, refusing the write once others claim some. */
    async #reserve(
        claim: ClaimType,
        home: RemoteStore,
        claimants: readonly Claimant[],
    ): Promise<void> {
        // record the reservations here first, so a crash before their confirmation leaves them due
        if (claimants.length === 0) {
            return;
        }
        const { database, environment, objects } = this.#store;
        const recorded = this.#rows(claimants, environment.clock.now() + RESERVATION_MILLISECONDS);
        await database.transaction(
            async (transaction) =>
                await transaction.insert(claimReservation).values(recorded).onConflictDoNothing(),
        );
        this.#reserved = claimants;

        // reserve the claiming objects' names in one request
        const claiming = claimants.filter((claimant) => claimant.claims.length > 0);
        const results =
            claiming.length === 0
                ? []
                : await home.mutate({
                      id: RequestId.create(environment),
                      calls: claiming.map((claimant) =>
                          claim.call("reserve", { ...claimant, request: this.#id }),
                      ),
                  });

        // release the reserved names and refuse the write once other objects claim some
        const refused = results.flatMap((result) => ClaimRefusal.parse(result).refused);
        if (refused.length > 0) {
            await this.cancel();
            Reservation.refuse(objects, refused);
        }
    }

    /** Describe the reservation rows of some objects, due at a time. */
    #rows(claimants: readonly Claimant[], expiresAt: number) {
        return claimants.flatMap((claimant) => {
            const [first] = claimant.indexes;

            return first === undefined
                ? []
                : [
                      {
                          requestId: this.#id,
                          objectId: claimant.objectId,
                          owner: claimant.owner,
                          index: first,
                          expiresAt,
                      },
                  ];
        });
    }

    /** Forget the reservation's rows here. */
    async #forget(): Promise<void> {
        await this.#store.database.transaction(
            async (transaction) =>
                await transaction
                    .delete(claimReservation)
                    .where(eq(claimReservation.requestId, this.#id)),
        );
    }

    /** Find each indexed type by its indexes. */
    static #types(objects: readonly ObjectType[]): Map<string, ObjectType> {
        return new Map(
            objects.flatMap((object) =>
                Object.keys(object.indexes).map(
                    (name) => [Claim.index(object, name), object] as const,
                ),
            ),
        );
    }

    /** Replace objects' names at their home after unreserved writes, answering the taken names. */
    static async #replace(
        claim: ClaimType,
        home: RemoteStore,
        claimants: readonly Claimant[],
        environment: UuidEnvironment,
    ): Promise<Claim[]> {
        // replace every object's names in one mutation at the home
        if (claimants.length === 0) {
            return [];
        }
        const calls = claimants.map((claimant) => claim.call("replace", claimant));
        const results = await home.mutate({ id: RequestId.create(environment), calls });

        return results.flatMap((result) => ClaimRefusal.parse(result).refused);
    }

    /** Observe on a controlled object whether another object holds one of its names. */
    static async #observe(
        store: ObjectStore,
        types: ReadonlyMap<string, ObjectType>,
        reservation: { readonly objectId: string; readonly index: string },
        taken: readonly Claim[],
    ): Promise<void> {
        // read the object, leaving one that is gone
        const object = present(types.get(reservation.index), "the reserved type");
        const [row]: readonly Row[] = await store.database
            .select()
            .from(object.table)
            .where(eq(object.table[TABLE].column("id"), reservation.objectId));
        if (row === undefined) {
            return;
        }

        // observe a taken name, and a name held again after one was taken
        const refused = taken.find((entry) => entry.objectId === reservation.objectId);
        const conditions = schema.record(schema.string(), Condition).parse(row["conditions"]);
        const isTaken = conditions["Ready"]?.reason === NAME_TAKEN;
        if (refused === undefined && !isTaken) {
            return;
        }
        const ready: Condition =
            refused === undefined
                ? { status: "true", reason: APPLIED, message: "" }
                : {
                      status: "false",
                      reason: NAME_TAKEN,
                      message: `'${Claim.values(refused).join(" ")}' is already taken`,
                  };
        await store.mutate(store.as(object), {
            id: RequestId.create(store.environment),
            calls: [
                object.call("observe", {
                    ...object.target(ObjectKey.parse(row)),
                    observedGeneration: row["generation"],
                    conditions: { Ready: ready },
                }),
            ],
        });
    }

    /** Refuse a write whose names other objects claim, reporting the index of the first. */
    static refuse(types: readonly ObjectType[], taken: readonly Claim[]): void {
        const [first] = taken;
        if (first !== undefined) {
            const [name] = types.flatMap((type) =>
                Object.keys(type.indexes).filter(
                    (indexName) => Claim.index(type, indexName) === first.index,
                ),
            );
            throw ObjectError.of(
                "NAME_EXISTS",
                `'${present(name, `the index of claim ${first.index}`)}' already exists`,
            );
        }
    }

    /** Declare the controller replacing the names a crash left reserved, or a migration queued. */
    static controller(
        store: ObjectStore,
        claim: ClaimType,
        home: RemoteStore | undefined,
    ): Controller<typeof claimReservation> {
        const types = Reservation.#types(store.objects);

        return {
            name: "destack.claim_reservation.expire",
            table: claimReservation,
            when: { expiresAt: { lte: now } },
            by: [],
            reconcile: async (rows) => {
                // read each object's names as its row has them now, under the reservation's owner once gone
                const { database, environment } = store;
                const snapshot = Snapshot.live(database);
                const claimants = await Promise.all(
                    rows.map(async (reservation) => {
                        // release the names of an index no type declares anymore
                        const object = types.get(reservation.index);
                        if (object === undefined) {
                            return {
                                indexes: [reservation.index],
                                objectId: reservation.objectId,
                                owner: reservation.owner,
                                claims: [],
                                state: "active" as const,
                                formerIndexes: [],
                                isRemoved: true,
                            };
                        }

                        // read the reserving object's row by its type
                        const [row]: readonly Row[] = await database
                            .select()
                            .from(object.table)
                            .where(eq(object.table[TABLE].column("id"), reservation.objectId));
                        const owner =
                            row === undefined ? reservation.owner : Claim.owning(object, row);

                        return Claimant.of(
                            object,
                            { objectId: reservation.objectId, owner },
                            row,
                            snapshot,
                        );
                    }),
                );

                // replace their names here where the store keeps the claims, else at the claim home
                const taken =
                    home === undefined
                        ? await database.transaction(async (transaction) => {
                              const refused: Claim[] = [];
                              for (const claimant of claimants) {
                                  refused.push(
                                      ...(await Claim.replace(transaction, claimant, environment)),
                                  );
                              }

                              return refused;
                          })
                        : await Reservation.#replace(claim, home, claimants, environment);

                // observe a controlled object's names on it, and refuse any other type's taken name
                const declared = rows.filter(
                    (reservation) => types.get(reservation.index)?.isControlled === true,
                );
                for (const reservation of declared) {
                    await Reservation.#observe(store, types, reservation, taken);
                }
                Reservation.refuse(
                    store.objects,
                    taken.filter((entry) => types.get(entry.index)?.isControlled !== true),
                );

                // forget the reservations
                await database.transaction(async (transaction) => {
                    for (const reservation of rows) {
                        await transaction
                            .delete(claimReservation)
                            .where(
                                and(
                                    eq(claimReservation.requestId, reservation.requestId),
                                    eq(claimReservation.objectId, reservation.objectId),
                                ),
                            );
                    }
                });
            },
        };
    }
}
