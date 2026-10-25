import { AccessContext, Caller } from "@destack/access";
import { Snapshot } from "@destack/db";
import { IdentityOperation } from "@destack/identity";
import type { ObjectStore } from "@destack/object/server";
import { RequestId, Scope, type Subject } from "@destack/sync";
import type { ClientOf } from "@destack/object/client";
import type accountPackage from "../package.ts";
import { identity, Identity, operation } from "../object/identity.ts";
import type { Environment } from "@destack/environment";

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
    /** Store identities in the universe's store, as its account installation or a caller. */
    of(
        store: Pick<ObjectStore, "mutate" | "database" | "as" | "environment">,
        caller?: Subject,
    ): IdentityRegistry {
        // call identity methods as the caller, else as the installation serving identities
        const call = async (calls: Parameters<ObjectStore["mutate"]>[1]["calls"]) => {
            const now = store.environment.clock.now();
            const acting =
                caller === undefined
                    ? store.as(identity, now)
                    : AccessContext.of(Caller.of(caller), now);
            await store.mutate(acting, {
                id: RequestId.create(store.environment),
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
        environment: Environment,
    ): IdentityRegistry {
        return {
            start: async (signed) => {
                await accounts.identity.create({
                    id: IdentityOperation.read(signed).subject,
                    requestId: RequestId.create(environment),
                    operation: signed,
                });
            },
            extend: async (signed) => {
                await accounts.operation.create({
                    parentId: IdentityOperation.read(signed).subject,
                    requestId: RequestId.create(environment),
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
