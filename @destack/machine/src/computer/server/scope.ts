import { AccessContext, Authorization, Caller } from "@destack/access";
import * as accountObject from "@destack/account/object";
import { type Controller, type Select, Snapshot } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import type { Identifier } from "@destack/schema";
import { Scope } from "@destack/sync";

/** Receive this machine's scope at its primary, the machine's own store, once its row arrives. */
export const ScopeReceiver = {
    /** Build the controller receiving a machine's scope once its row and its parent's arrive. */
    controller(
        store: Pick<ObjectStore, "database" | "authorizer" | "environment" | "principal">,
        machineId: Identifier<"machine">,
    ): Controller<typeof accountObject.machine.table> {
        return {
            name: "machine-scope",
            table: accountObject.machine.table,
            pending: { id: machineId },
            // receive it again once its parent's scope arrives
            watches: [{ table: Scope.table, keys: () => [{ id: machineId }] }],
            reconcile: (rows) => ScopeReceiver.receive(store, rows),
        };
    },

    /** Receive the machine's scope below its parent, leaving one received or without its parent. */
    async receive(
        store: Pick<ObjectStore, "database" | "authorizer" | "environment" | "principal">,
        rows: readonly Select<typeof accountObject.machine.table>[],
    ): Promise<void> {
        // leave a machine whose scope this store serves
        const { database, authorizer, environment, principal } = store;
        const view = Snapshot.live(database);
        const [row] = rows;
        if (row === undefined || (await Scope.read(view, [row.id])).length > 0) {
            return;
        }

        // receive it once its parent's scope arrived, below the parent's ancestors
        const [parent] = await Scope.read(view, [row.scope]);
        if (parent === undefined) {
            return;
        }
        const now = environment.clock.now();
        const scope = accountObject.machine.reference(row.scope, row.id);
        const ancestors = [row.scope, ...parent.ancestors];
        await Authorization.run(
            authorizer,
            database,
            () => AccessContext.of(Caller.of(principal), now),
            environment,
            (authorization) => authorization.receive(scope, ancestors),
        );
    },
};
