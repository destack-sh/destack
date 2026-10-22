import { Space, space, placement } from "@destack/account/object";
import {
    and,
    Change,
    eq,
    inArray,
    type Database,
    type DatabaseConnection,
    type Table,
    Condition,
    Transaction,
} from "@destack/db";
import { DesiredState, type ResourceType, type ResourceRow } from "@destack/object";
import { DRAINING, type ObjectStore } from "@destack/object/server";
import type { ResourceState } from "@destack/package/declare";
import { nth, Digest, present } from "@destack/schema";
import type { ResourceKind } from "@destack/resource";
import type { Controller } from "@destack/db";
import { binding, capture, Deployment, deployment, MAIN_DATABASE } from "../../object/index.ts";
import { RequestId } from "@destack/sync";

/** Write each resource's specification from its space and consumers where the resource lives. */
export class ConsumerController implements Controller {
    /** The controller's name in reports. */
    readonly name = "consumer";
    /** The tables of the consumers, the spaces and their placements, and every kind's resources. */
    readonly watches: readonly Table[];
    /** The object store writing the resources and stopping deployments as its principal. */
    readonly #store: Pick<ObjectStore, "database" | "serving" | "mutate" | "clock">;
    /** The objects of every kind whose resources the spaces use, kept here or replicated. */
    readonly #objects: readonly ResourceType<typeof space>[];
    /** The main database declaring the platform's tables every space's main database keeps. */
    readonly #main: Database;
    /** The state of the platform's tables by its digest, read on first use. */
    #platform: Promise<ReadonlyMap<Digest, ResourceState>> | undefined;

    /** Reconcile the resources of some kinds in the spaces a machine serves. */
    constructor(
        store: Pick<ObjectStore, "database" | "serving" | "mutate" | "clock">,
        objects: readonly ResourceType<typeof space>[],
        main: Database,
    ) {
        // keep the store, the kinds' objects and the platform's main database
        this.#store = store;
        this.#objects = objects;
        this.#main = main;

        // watch the consumers, the spaces and the resources
        this.watches = [
            binding.table,
            capture.table,
            deployment.table,
            space.table,
            placement.table,
            ...this.#objects.map((object) => object.table),
        ];
    }

    /** List the resources a change of a consumer, a space, its placement or a resource touches. */
    async keys(change: Change): Promise<readonly string[]> {
        const { database } = this.#store;

        // select the resource a changed binding or capture targets
        if (Change.of(change, binding.table) || Change.of(change, capture.table)) {
            return this.#targets([
                Change.image<typeof binding.table | typeof capture.table>(change).target,
            ]);
        }
        // select the resources a changed deployment captured
        else if (Change.of(change, deployment.table)) {
            const captured = await database
                .selectDistinct({ target: capture.table.target })
                .from(capture.table)
                .where(eq(capture.table.deploymentId, Change.image(change).id));

            return this.#targets(captured.map((entry) => entry.target));
        }
        // select the resources of a space whose approval threshold or placement changed
        else if (Change.of(change, space.table) || Change.of(change, placement.table)) {
            return this.#resourcesIn(database, [
                Change.image<typeof space.table | typeof placement.table>(change).id,
            ]);
        }
        // select a changed resource
        else {
            const changed = Change.image(change)["id"];

            return typeof changed === "string" ? [changed] : [];
        }
    }

    /** List the resources of the spaces the machine serves. */
    async list(): Promise<readonly string[]> {
        const { database } = this.#store;
        const served = await database
            .select({ id: space.table.id })
            .from(space.table)
            .where(Space.served());

        return this.#resourcesIn(
            database,
            served.map((entry) => entry.id),
        );
    }

    /** Write a resource's desired states and approval threshold. */
    async reconcile(key: string): Promise<undefined> {
        // read the resource and the served space it lives in, none for a resource of an account
        const { database } = this.#store;
        const object = this.#objects.find((each) => each.identifies(key));
        const [row] =
            object === undefined
                ? []
                : await database
                      .select()
                      .from(object.table)
                      .where(object.withIds([key]));
        const [served] =
            row === undefined || !space.identifies(row.scope)
                ? []
                : await database
                      .select({ approval: space.table.approval })
                      .from(space.table)
                      .where(and(eq(space.table.id, row.scope), Space.served()));
        if (object === undefined || row === undefined || served === undefined) {
            return undefined;
        }

        // read the states its consumers need by digest
        const now = this.#store.clock.now();
        const platform = row.name === MAIN_DATABASE ? await this.#platformState() : new Map();
        const { active, draining } = await desiredStates(database, object, row, platform);
        const input = {
            id: row.id,
            states: [...active.keys()],
            drainingStates: [...draining.keys()],
            approval: served.approval,
        };

        // keep the states and write their digests and its space's approval threshold once they changed
        const isChanged =
            input.states.join() !== row.states.join() ||
            input.drainingStates.join() !== row.drainingStates.join() ||
            input.approval !== row.approval;
        if (isChanged) {
            await Transaction.run(database, async (transaction) => {
                await DesiredState.upsert(transaction, row.scope, [
                    ...active.values(),
                    ...draining.values(),
                ]);
                await this.#store.mutate(
                    this.#store.serving(object),
                    { id: RequestId.create(now), calls: [object.call("specify", input)] },
                    { scope: row.scope, component: this.name, transaction },
                );
            });
        }

        // stop the draining deployments that captured a draining resource
        if (row.conditions[DRAINING]?.status === "true") {
            await this.#stopDraining(row, now);
        }

        return undefined;
    }

    /** Read the state of the platform's tables by its digest once. */
    #platformState(): Promise<ReadonlyMap<Digest, ResourceState>> {
        this.#platform ??= (async () => {
            const state = this.#main.state();

            return new Map([[await Digest.json(state), state]]);
        })();

        return this.#platform;
    }

    /** Stop the draining deployments that captured a resource. */
    async #stopDraining(row: Pick<ResourceRow, "id" | "scope">, now: number): Promise<void> {
        // find the draining deployments capturing the resource
        const { database } = this.#store;
        const draining = await database
            .selectDistinct({ deployment: deployment.table })
            .from(capture.table)
            .innerJoin(deployment.table, eq(deployment.table.id, capture.table.deploymentId))
            .where(
                and(
                    eq(capture.table.scope, row.scope),
                    eq(capture.table.target, row.id),
                    eq(deployment.table.status, "draining"),
                ),
            );
        await this.#store.mutate(
            this.#store.serving(deployment),
            {
                id: RequestId.create(now),
                calls: draining.map((entry) =>
                    deployment.call("update", {
                        ...deployment.target(entry.deployment),
                        status: "stopping",
                    }),
                ),
            },
            { component: this.name },
        );
    }

    /** Keep the targets that are resources of a served kind. */
    #targets(targets: readonly string[]): string[] {
        return targets.filter((target) =>
            this.#objects.some((object) => object.identifies(target)),
        );
    }

    /** List the resources of every kind in some spaces. */
    async #resourcesIn(database: DatabaseConnection, spaces: readonly string[]): Promise<string[]> {
        const resources: string[] = [];
        for (const object of this.#objects) {
            const rows = await database
                .select({ id: object.table.id })
                .from(object.table)
                .where(inArray(object.table.scope, [...spaces]));
            resources.push(...rows.map((entry) => entry.id));
        }

        return resources;
    }
}

/** Read the distinct desired states of a resource, active and draining, by digest in order. */
async function desiredStates(
    database: DatabaseConnection,
    object: ResourceType<typeof space>,
    row: ResourceRow,
    platform: ReadonlyMap<Digest, ResourceState>,
): Promise<{
    readonly active: ReadonlyMap<Digest, ResourceState>;
    readonly draining: ReadonlyMap<Digest, ResourceState>;
}> {
    // write no states for a kind without a state
    const { kind } = present(object.provisioned, `the kind of ${object.name}`);
    if (!kind.isStateful) {
        return { active: new Map(), draining: new Map() };
    }

    // read the bindings' states and the states the serving deployments run with
    const bound = await database
        .select({ state: binding.table.state })
        .from(binding.table)
        .where(and(eq(binding.table.scope, row.scope), eq(binding.table.target, row.id)));
    const captured = await database
        .select({ state: capture.table.state, status: deployment.table.status })
        .from(capture.table)
        .innerJoin(deployment.table, eq(deployment.table.id, capture.table.deploymentId))
        .where(
            and(
                eq(capture.table.scope, row.scope),
                eq(capture.table.target, row.id),
                Condition.render(Deployment.serving(), deployment.table),
            ),
        );

    // check each stored state once, beside the platform's
    const active = sorted(
        new Map([
            ...platform,
            ...(await checked(kind, [
                ...bound.map((entry) => entry.state),
                ...captured
                    .filter((entry) => entry.status === "active")
                    .map((entry) => entry.state),
            ])),
        ]),
    );

    // leave out the draining states an active consumer needs
    const draining = await checked(
        kind,
        captured.filter((entry) => entry.status === "draining").map((entry) => entry.state),
    );

    return {
        active,
        draining: sorted(new Map([...draining].filter(([digest]) => !active.has(digest)))),
    };
}

/** Check each distinct stored state by its kind, keyed by its digest. */
async function checked(
    kind: ResourceKind,
    stored: readonly ResourceState[],
): Promise<Map<Digest, ResourceState>> {
    // key the stored states by digest, then refuse a distinct one its kind does not accept
    const digests = await Promise.all(stored.map((state) => Digest.json(state)));
    const byDigest = new Map(digests.map((digest, index) => [digest, nth(stored, index)]));
    kind.states([...byDigest.values()]);

    return byDigest;
}

/** Order states by their digests. */
function sorted(states: ReadonlyMap<Digest, ResourceState>): Map<Digest, ResourceState> {
    return new Map([...states].toSorted(([left], [right]) => (left < right ? -1 : 1)));
}
