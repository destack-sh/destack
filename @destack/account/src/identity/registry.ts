import { Snapshot } from "@destack/db";
import { IdentityOperation } from "@destack/identity";
import type { ObjectStore } from "@destack/object/server";
import { RequestId, Scope } from "@destack/sync";
import type { ClientOf } from "@destack/object/client";
import type accountPackage from "../package.ts";
import { identity, Identity, operation } from "../object/identity.ts";
import type { Clock } from "@destack/environment";

/** The most operations one read of an identity's log takes: a page, far above a log's length. */
const LOG_PAGE = 1000;

/** The universe's registry of identities, where a keystore starts, changes and reads identities. */
export interface IdentityRegistry {
    /** Start an identity with its first signed operation. */
    readonly start: (signed: string) => Promise<void>;
    /** Append a signed operation to its identity's log. */
    readonly extend: (signed: string) => Promise<void>;
    /** Read an identity's current state, absent before its first operation. */
    readonly find: (subject: string) => Promise<ReturnType<typeof Identity.current>>;
}

/** The registries of identities. */
export const IdentityRegistry = {
    /** Store identities in the universe's own store, calling as the store's principal. */
    of(store: Pick<ObjectStore, "mutate" | "database" | "serving" | "clock">): IdentityRegistry {
        // call identity methods as the store's principal
        const call = async (calls: Parameters<ObjectStore["mutate"]>[1]["calls"]) => {
            const now = store.clock.now();
            await store.mutate(store.serving(identity), {
                id: RequestId.create(now),
                calls,
            });
        };

        return {
            start: (signed) =>
                call([
                    identity.call("create", {
                        ...identity.target({
                            scope: Scope.universe.id,
                            id: IdentityOperation.read(signed).subject,
                        }),
                        operation: signed,
                    }),
                ]),
            extend: (signed) =>
                call([
                    operation.call("create", {
                        ...operation.in(Scope.universe.id),
                        parentId: IdentityOperation.read(signed).subject,
                        operation: signed,
                    }),
                ]),
            find: async (subject) =>
                Identity.current(await Identity.log(Snapshot.live(store.database), subject)),
        };
    },

    /** Store identities at the universe through a client of the account package. */
    client(
        accounts: Pick<ClientOf<typeof accountPackage>, "identity" | "operation">,
        clock: Clock,
    ): IdentityRegistry {
        return {
            start: async (signed) => {
                await accounts.identity.create({
                    id: IdentityOperation.read(signed).subject,
                    requestId: RequestId.create(clock.now()),
                    operation: signed,
                });
            },
            extend: async (signed) => {
                await accounts.operation.create({
                    parentId: IdentityOperation.read(signed).subject,
                    requestId: RequestId.create(clock.now()),
                    operation: signed,
                });
            },
            find: async (subject) => {
                // read the identity's whole log, short after a few rotations
                const { items } = await accounts.operation.list({
                    where: { parentId: subject },
                    limit: LOG_PAGE,
                });

                return Identity.current(
                    items.toSorted((left, right) => left.sequence - right.sequence),
                );
            },
        };
    },
};
