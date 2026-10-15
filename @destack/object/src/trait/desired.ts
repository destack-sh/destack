import { defineTable, json, type Snapshot, text, Transaction } from "@destack/db";
import type { ResourceState } from "@destack/package/declare";
import { Digest, present, schema, zip } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import type { Call } from "../method/call.ts";
import { Provisioned } from "./provisioned.ts";
import { Execution } from "../method/execution.ts";

/** The desired states of a scope's resources, each kept once under the digest of its JSON. */
export const desiredState = defineTable(
    "desired_state",
    {
        /** The scope whose resources need the state. */
        scope: text("scope").primaryKey(),
        /** The SHA-256 digest of the state's canonical JSON. */
        digest: text("digest").validate(Digest).primaryKey(),
        /** The state, such as a database's tables. */
        state: json("state", schema.jsonObject()).notNull(),
    },
    { log: {} },
);

/** The desired states of resources, which their rows list by digest. */
export const DesiredState = {
    /** Keep the states a scope's resources need, writing missing ones, answering their digests. */
    async keep(
        transaction: Transaction,
        scope: string,
        states: readonly ResourceState[],
    ): Promise<Digest[]> {
        // digest each state and read the states the scope keeps already
        const digests = await Promise.all(states.map((state) => Digest.json(state)));
        const kept = await transaction
            .read(desiredState)
            .findMany({ where: { scope, digest: { in: digests } } });
        const known = new Set(kept.map((row) => row.digest));

        // write each other state once
        for (const [digest, state] of zip(digests, states)) {
            if (!known.has(digest)) {
                known.add(digest);
                await transaction.write({
                    table: desiredState,
                    before: null,
                    after: { scope, digest, state },
                });
            }
        }

        return digests;
    },

    /** Read the states of some digests in a scope in order, refusing a digest without one. */
    async read(
        view: Pick<Snapshot, "read">,
        scope: string,
        digests: readonly Digest[],
    ): Promise<ResourceState[]> {
        // read the kept states
        const rows = await view.read(desiredState).findMany({
            where: {
                scope,
                digest: { in: [...digests] },
            },
        });
        const states = new Map(rows.map((row) => [row.digest, row.state]));

        // refuse a digest without a state in the scope
        const missing = digests.filter((digest) => !states.has(digest));
        if (missing.length > 0) {
            throw new CallError("BAD_REQUEST", {
                message: `${scope} has no desired states ${missing.join(", ")}`,
            });
        }

        return digests.map((digest) => present(states.get(digest), digest));
    },

    /** Drop the states among some digests that no resource of a call's space needs any more. */
    async drop(call: Call, execution: Execution, digests: readonly Digest[]): Promise<void> {
        // collect the digests every resource of the space needs
        const scope = schema.identifier("space").parse(call.scope);
        const needed = new Set<Digest>();
        for (const object of call.objects.filter((each) => Provisioned.is(each))) {
            const resources = await call.read(object.table).findMany({ where: { scope } });
            for (const resource of resources) {
                for (const digest of [...resource.states, ...resource.drainingStates]) {
                    needed.add(digest);
                }
            }
        }

        // delete the others
        const unneeded = digests.filter((digest) => !needed.has(digest));
        if (unneeded.length > 0) {
            const dropped = await call.read(desiredState).findMany({
                where: {
                    scope,
                    digest: { in: unneeded },
                },
            });
            await Execution.write(
                execution,
                call,
                ...dropped.map((row) => ({ table: desiredState, before: row, after: null })),
            );
        }
    },
};
