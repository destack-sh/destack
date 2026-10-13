import { Caller } from "@destack/access";
import { type DatabaseConnection, Snapshot } from "@destack/db";
import { present, schema } from "@destack/schema";
import { errorOf, CallError } from "@destack/runtime/error";
import { RequestId } from "@destack/runtime/request";
import type * as sync from "@destack/sync";
import type { ObjectStore } from "../server/store.ts";
import { Call } from "../method/call.ts";
import type { ObjectType } from "../object/object.ts";
import { Claim, type Claimant, ClaimRefusal, claimTable, Expiry } from "./claim.ts";
import { ObjectReader } from "../method/reader.ts";
import type { ClaimType } from "./method.ts";
import type { PushResult } from "../replica/replica.ts";

/** A principal signing calls to a store elsewhere, with the connection reading its keys. */
export interface Signer {
    /** The signing principal. */
    readonly subject: sync.Subject;
    /** The connection reading the principal's keys. */
    readonly database: DatabaseConnection;
}

/** The principal claiming a write's names: the store itself, or an installation it serves. */
export type Claimer =
    | {
          /** The store claims as its own principal. */
          readonly kind: "store";
      }
    | {
          /** An installation the store serves claims as itself. */
          readonly kind: "installation";
          /** The installation signing its claims. */
          readonly signer: Signer;
      };

/** A store serving object types elsewhere, called as a write's claimer. */
export interface RemoteStore {
    /** Run a mutation's calls there in one transaction, answering their results in order. */
    mutate(mutation: sync.Mutation, claimer: Claimer): Promise<readonly unknown[]>;
    /** Run a reading call there as the calling store. */
    query(call: sync.Call): Promise<unknown>;
}

/** The replica procedures of a store elsewhere, as a client calls them. */
export interface ReplicaClient {
    /** Push mutations to a scope, answering each one's outcome. */
    push(input: {
        readonly scope: string;
        readonly mutations: sync.Mutation[];
    }): Promise<Pick<PushResult, "outcomes">>;
    /** Run a reading call in a scope. */
    call(input: { readonly scope: string; readonly call: sync.Call }): Promise<unknown>;
}

/** The stores elsewhere a store calls. */
export const RemoteStore = {
    /** Call a store elsewhere in one scope through its replica procedures, as a claimer. */
    of(replica: (claimer: Claimer) => ReplicaClient, scope: string): RemoteStore {
        return {
            mutate: async (mutation, claimer) => {
                // push the mutation alone, and throw its failure
                const { outcomes } = await replica(claimer).push({ scope, mutations: [mutation] });
                const { outcome } = present(outcomes[0], "the pushed mutation's outcome");
                if (!("value" in outcome)) {
                    throw errorOf(outcome.error);
                }

                return schema.array(schema.json()).parse(outcome.value);
            },
            query: (call) => replica({ kind: "store" }).call({ scope, call }),
        };
    },

    /** Call a store served in this process as a principal. */
    served(
        store: Pick<ObjectStore, "mutate" | "query" | "clock">,
        principal: sync.Subject,
    ): RemoteStore {
        // run calls as the claiming installation, else as the principal
        const callerOf = (claimer: Claimer) =>
            Caller.of(
                claimer.kind === "store" ? principal : claimer.signer.subject,
                store.clock.now(),
            );

        return {
            mutate: (mutation, claimer) => store.mutate(callerOf(claimer), mutation),
            query: (call) => store.query(callerOf({ kind: "store" }), call),
        };
    },
};

/** The claim type keeping the names of indexed objects, and the store serving it, its home. */
export interface ClaimHome {
    /** The claim type, whose methods reserve, confirm, release and replace names. */
    readonly object: ClaimType;
    /** The store serving the claim type, absent when the claiming store serves it itself. */
    readonly home?: RemoteStore;
}

/** The names a write reserved, and how its commit and outcome settle them. */
export interface Reservation {
    /** Write the names in the committing transaction where the store keeps the claims itself. */
    commit(committing: DatabaseConnection): Promise<void>;

    /** Settle the reserved names once the write committed. */
    confirm(): Promise<void>;

    /** Release the reserved names once the write failed. */
    release(): Promise<void>;
}

/** A write's claim request: its identifier, its time and its claimer. */
interface ClaimRequest {
    /** The write's request, which keeps its reservations. */
    readonly id: string;
    /** The write's time, in UTC epoch milliseconds. */
    readonly now: number;
    /** The principal claiming the names. */
    readonly claimer: Claimer;
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
            ? local(claimants, types)
            : remote(claims.object, claims.home, claimants, types, request);
    },

    /** Replace objects' names after unreserved writes, as their claimer, refusing taken names. */
    async replace(
        claims: ClaimHome,
        database: DatabaseConnection,
        claimants: readonly Claimant[],
        types: readonly ObjectType[],
        request: { readonly now: number; readonly claimer: Claimer },
    ): Promise<void> {
        // replace in the store's own database, else at the claim type's home
        const refused: Claim[] = [];
        if (claims.home === undefined) {
            for (const claimant of claimants) {
                refused.push(...(await Claim.replace(database, claimant)));
            }
        } else if (claimants.length > 0) {
            const calls = claimants.map((claimant) =>
                Call.wire(claims.object.call("replace", claimant)),
            );
            const mutation = { id: RequestId.create(request.now), calls };
            const results = await claims.home.mutate(mutation, request.claimer);
            refused.push(...results.flatMap((result) => ClaimRefusal.parse(result).refused));
        }

        // refuse names other objects own
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
            ? Claim.expired(new ObjectReader(claimTable, Snapshot.live(database)), indexes, now)
            : Expiry.parse(
                  await claims.home.query(
                      Call.wire(claims.object.call("expired", { indexes: [...indexes] })),
                  ),
              );
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
            throw new CallError("CONFLICT", {
                message: `${present(name, `the index of claim ${first.index}`)} is taken`,
            });
        }
    },
};

/** Claim the names in the committing transaction of the store serving the claim type. */
function local(claimants: readonly Claimant[], types: readonly ObjectType[]): Reservation {
    return {
        commit: async (committing) => {
            // replace each object's names at once, refusing names other objects own
            const refused: Claim[] = [];
            for (const claimant of claimants) {
                refused.push(...(await Claim.replace(committing, claimant)));
            }
            Reservation.refuse(types, refused);
        },
        confirm: async () => {},
        release: async () => {},
    };
}

/** Reserve the names at the claim type's home as their claimer until the write commits. */
async function remote(
    claim: ClaimType,
    home: RemoteStore,
    claimants: readonly Claimant[],
    types: readonly ObjectType[],
    request: ClaimRequest,
): Promise<Reservation> {
    // reserve the claiming objects' names in one request
    const { id, now, claimer } = request;
    const claiming = claimants.filter((claimant) => claimant.claims.length > 0);
    const results =
        claiming.length === 0
            ? []
            : await home.mutate(
                  {
                      id: RequestId.create(now),
                      calls: claiming.map((claimant) =>
                          Call.wire(claim.call("reserve", { ...claimant, request: id })),
                      ),
                  },
                  claimer,
              );
    const refused = results.flatMap((result) => ClaimRefusal.parse(result).refused);

    // confirm every object's names once the write commits, and release the reserved ones once it fails
    const reservation: Reservation = {
        commit: async () => {},
        confirm: async () => {
            if (claimants.length > 0) {
                await home.mutate(
                    {
                        id: RequestId.create(now),
                        calls: claimants.map((claimant) =>
                            Call.wire(claim.call("confirm", { ...claimant, request: id })),
                        ),
                    },
                    claimer,
                );
            }
        },
        release: async () => {
            if (claiming.length > 0) {
                await home.mutate(
                    {
                        id: RequestId.create(now),
                        calls: claiming.map(({ objectId, owner }) =>
                            Call.wire(claim.call("release", { objectId, owner, request: id })),
                        ),
                    },
                    claimer,
                );
            }
        },
    };

    // release the reserved names and refuse the write once other objects own some
    if (refused.length > 0) {
        await reservation.release();
        Reservation.refuse(types, refused);
    }

    return reservation;
}
