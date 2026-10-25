import { Failure } from "@destack/error";
import { ObjectError } from "../error/error.ts";
import { AccessContext, Caller } from "@destack/access";
import { type DatabaseConnection, Snapshot } from "@destack/db";
import { type UuidEnvironment, present, schema } from "@destack/schema";

import { RequestId } from "@destack/sync";
import type * as sync from "@destack/sync";
import type { ObjectStore } from "../server/store.ts";
import type { ObjectType } from "../object/object.ts";
import { Claim, type Claimant, ClaimRefusal, claimTable, Expiry } from "./claim.ts";
import type { ClaimType } from "./method.ts";
import type { PushResult } from "../replica/replica.ts";

import type {} from "@destack/package/import-meta";

/** A store serving object types elsewhere, called as the claiming store. */
export interface RemoteStore {
    /** Run a mutation's calls there in one transaction, answering their results in order. */
    readonly mutate: (mutation: sync.Mutation) => Promise<readonly unknown[]>;
    /** Run a reading call there as the calling store. */
    readonly query: (call: sync.Call) => Promise<unknown>;
}

/** The replica procedures of a store elsewhere, as a client calls them. */
export interface ReplicaClient {
    /** Push mutations to a scope, answering each one's outcome. */
    readonly push: (input: {
        readonly scope: string;
        readonly mutations: sync.Mutation[];
    }) => Promise<Pick<PushResult, "outcomes">>;
    /** Run a reading call in a scope. */
    readonly call: (input: {
        readonly scope: string;
        readonly call: sync.Call;
    }) => Promise<unknown>;
}

/** The stores elsewhere a store calls. */
export const RemoteStore = {
    /** Call a store elsewhere in one scope through its replica procedures. */
    of(replica: ReplicaClient, scope: string): RemoteStore {
        return {
            mutate: async (mutation) => {
                // push the mutation alone, and throw its failure
                const { outcomes } = await replica.push({ scope, mutations: [mutation] });
                const { outcome } = present(outcomes[0], "the pushed mutation's outcome");
                if (!("value" in outcome)) {
                    throw Failure.error(outcome.error);
                }

                return schema.array(schema.json()).parse(outcome.value);
            },
            query: (call) => replica.call({ scope, call }),
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

/** The claim type recording the names of indexed objects, and the store serving it, its home. */
export interface ClaimHome {
    /** The claim type, whose methods reserve, confirm, release and replace names. */
    readonly object: ClaimType;
    /** The store serving the claim type, absent when the claiming store serves it itself. */
    readonly home?: RemoteStore;
}

/** The names a write reserved, and how its commit and outcome settle them. */
export interface Reservation {
    /** Write the names in the committing transaction where the store has the claims itself. */
    readonly commit: (committing: DatabaseConnection) => Promise<void>;

    /** Settle the reserved names once the write committed. */
    readonly confirm: () => Promise<void>;

    /** Release the reserved names once the write failed. */
    readonly release: () => Promise<void>;
}

/** A write's claim request: its identifier and the environment its claims draw from. */
interface ClaimRequest {
    /** The write's request, which has its reservations. */
    readonly id: string;
    /** The environment of the write's store. */
    readonly environment: UuidEnvironment;
}

/** The reservations of writes claiming names. */
export const Reservation = {
    /** Reserve written objects' names before the write commits, refusing taken names. */
    async reserve(
        claims: ClaimHome,
        claimants: readonly Claimant[],
        types: readonly ObjectType[],
        request: ClaimRequest,
    ): Promise<Reservation> {
        // claim in the committing transaction of the store serving the claim type, else at its home now
        return claims.home === undefined
            ? Reservation.local(claimants, types, request.environment)
            : Reservation.remote(claims.object, claims.home, claimants, types, request);
    },

    /** Replace objects' names after unreserved writes, refusing taken names. */
    async replace(
        claims: ClaimHome,
        database: DatabaseConnection,
        claimants: readonly Claimant[],
        types: readonly ObjectType[],
        environment: UuidEnvironment,
    ): Promise<void> {
        // replace in the store's database, else at the claim type's home
        const refused: Claim[] = [];
        if (claims.home === undefined) {
            for (const claimant of claimants) {
                refused.push(...(await Claim.replace(database, claimant, environment)));
            }
        } else if (claimants.length > 0) {
            const calls = claimants.map((claimant) => claims.object.call("replace", claimant));
            const mutation = { id: RequestId.create(environment), calls };
            const results = await claims.home.mutate(mutation);
            refused.push(...results.flatMap((result) => ClaimRefusal.parse(result).refused));
        }

        // refuse names other objects claim
        Reservation.refuse(types, refused);
    },

    /** List the expired reservations of some indexes. */
    async expired(
        claims: ClaimHome,
        database: DatabaseConnection,
        indexes: readonly string[],
        now: number,
    ): Promise<Expiry> {
        return claims.home === undefined
            ? Claim.expired(Snapshot.live(database).read(claimTable), indexes, now)
            : Expiry.parse(
                  await claims.home.query(claims.object.call("expired", { indexes: [...indexes] })),
              );
    },

    /** Refuse a write whose names other objects claim, reporting the index of the first. */
    refuse(types: readonly ObjectType[], taken: readonly Claim[]): void {
        const [first] = taken;
        if (first !== undefined) {
            const [name] = types.flatMap((type) =>
                Object.keys(type.indexes).filter(
                    (index) => Claim.index(type, index) === first.index,
                ),
            );
            throw ObjectError.of(
                "NAME_TAKEN",
                `${present(name, `the index of claim ${first.index}`)} is taken`,
            );
        }
    },

    /** Claim the names in the committing transaction of the store serving the claim type. */
    local(
        claimants: readonly Claimant[],
        types: readonly ObjectType[],
        environment: UuidEnvironment,
    ): Reservation {
        return {
            commit: async (committing) => {
                // replace each object's names at once, refusing names other objects claim
                const refused: Claim[] = [];
                for (const claimant of claimants) {
                    refused.push(...(await Claim.replace(committing, claimant, environment)));
                }
                Reservation.refuse(types, refused);
            },
            confirm: async () => {},
            release: async () => {},
        };
    },

    /** Reserve the names at the claim type's home until the write commits. */
    async remote(
        claim: ClaimType,
        home: RemoteStore,
        claimants: readonly Claimant[],
        types: readonly ObjectType[],
        request: ClaimRequest,
    ): Promise<Reservation> {
        // reserve the claiming objects' names in one request
        const { id, environment } = request;
        const claiming = claimants.filter((claimant) => claimant.claims.length > 0);
        const results =
            claiming.length === 0
                ? []
                : await home.mutate({
                      id: RequestId.create(environment),
                      calls: claiming.map((claimant) =>
                          claim.call("reserve", { ...claimant, request: id }),
                      ),
                  });
        const refused = results.flatMap((result) => ClaimRefusal.parse(result).refused);

        // confirm the reserved names on commit, and release them on failure
        const reservation: Reservation = {
            commit: async () => {},
            confirm: async () => {
                if (claimants.length > 0) {
                    await home.mutate({
                        id: RequestId.create(environment),
                        calls: claimants.map((claimant) =>
                            claim.call("confirm", { ...claimant, request: id }),
                        ),
                    });
                }
            },
            release: async () => {
                if (claiming.length > 0) {
                    await home.mutate({
                        id: RequestId.create(environment),
                        calls: claiming.map(({ objectId, owner }) =>
                            claim.call("release", { objectId, owner, request: id }),
                        ),
                    });
                }
            },
        };

        // release the reserved names and refuse the write once other objects claim some
        if (refused.length > 0) {
            await reservation.release();
            Reservation.refuse(types, refused);
        }

        return reservation;
    },
};
