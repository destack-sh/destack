import { Space, space, placement } from "@destack/account/object";
import { MAIN_DATABASE } from "../../declare/index.ts";
import {
    and,
    Change,
    eq,
    inArray,
    type Database,
    type DatabaseConnection,
    type Table,
    type TableState,
    Condition,
} from "@destack/db";
import { type Resource, type ResourceType } from "@destack/object";
import { DRAINING, type ObjectStore } from "@destack/object/server";
import type { ResourceState } from "@destack/package/declare";
import { Digest, present } from "@destack/schema";
import type { ReconcileController } from "@destack/db";
import { binding, capture, database, Deployment, deployment } from "../../object/index.ts";
import { RequestId } from "@destack/sync";

/** Write each resource's specification from its space and consumers where the resource lives. */
export class ConsumerController implements ReconcileController {
    /** The controller's name in reports. */
    readonly name = "consumer";
    /** The tables of the consumers, the spaces and their placements, and every kind's resources. */
    readonly watches: readonly Table[];
    /** The object store writing the resources and stopping deployments as its principal. */
    readonly #store: Pick<
        ObjectStore,
        "database" | "blobs" | "as" | "mutate" | "clock" | "environment"
    >;
    /** The objects of every kind whose resources the spaces use, stored here or replicated. */
    readonly #objects: readonly ResourceType<typeof space>[];
    /** The main database declaring the platform's tables in every space's main database. */
    readonly #main: Database;
    /** The state of the platform's tables by its digest, per set of replicated tables left out. */
    readonly #platform = new Map<string, Promise<ReadonlyMap<Digest, ResourceState>>>();

    /** Reconcile the resources of some kinds in the spaces a machine serves. */
    constructor(
        store: Pick<ObjectStore, "database" | "blobs" | "as" | "mutate" | "clock" | "environment">,
        objects: readonly ResourceType<typeof space>[],
        main: Database,
    ) {
        // store the object store, the kinds' objects and the platform's main database
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
        const connection = this.#store.database;

        // select the resource a changed binding or capture targets
        if (Change.of(change, binding.table) || Change.of(change, capture.table)) {
            return this.#targets([
                Change.image<typeof binding.table | typeof capture.table>(change).target,
            ]);
        }
        // select the resources a changed deployment captured
        else if (Change.of(change, deployment.table)) {
            const captured = await connection
                .selectDistinct({ target: capture.table.target })
                .from(capture.table)
                .where(eq(capture.table.deploymentId, Change.image(change).id));

            return this.#targets(captured.map((entry) => entry.target));
        }
        // select the resources of a space whose approval threshold or placement changed
        else if (Change.of(change, space.table) || Change.of(change, placement.table)) {
            return this.#resourcesIn(connection, [
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
        const connection = this.#store.database;
        const served = await connection
            .select({ id: space.table.id })
            .from(space.table)
            .where(Space.served());

        return this.#resourcesIn(
            connection,
            served.map((entry) => entry.id),
        );
    }

    /** Write a resource's desired states and approval threshold. */
    async reconcile(key: string): Promise<undefined> {
        // read the resource and the served space it lives in, none for a resource of an account
        const connection = this.#store.database;
        const object = this.#objects.find((each) => each.identifies(key));
        const [row] =
            object === undefined
                ? []
                : await connection
                      .select()
                      .from(object.table)
                      .where(object.withIds([key]));
        const [served] =
            row === undefined || !space.identifies(row.scope)
                ? []
                : await connection
                      .select({ approval: space.table.approval })
                      .from(space.table)
                      .where(and(eq(space.table.id, row.scope), Space.served()));
        if (object === undefined || row === undefined || served === undefined) {
            return undefined;
        }

        // read the states its consumers need by digest, the platform's for the main database
        const platform =
            object === database && row.name === MAIN_DATABASE
                ? await this.#platformState(await this.#bound(row.id))
                : new Map<Digest, ResourceState>();
        const { active, draining } = await desiredStates(connection, object, row, platform);
        const input = {
            id: row.id,
            states: active,
            drainingStates: draining,
            approval: served.approval,
        };

        // record the states, and write their digests and its space's approval threshold once they changed
        const isChanged =
            input.states.join() !== row.states.join() ||
            input.drainingStates.join() !== row.drainingStates.join() ||
            input.approval !== row.approval;
        if (isChanged) {
            // write the platform's state as a blob, as the deploys wrote the others
            for (const state of platform.values()) {
                await this.#store.blobs.writeJson(state);
            }
            await this.#store.mutate(
                this.#store.as(object),
                {
                    id: RequestId.create(this.#store.environment),
                    calls: [object.call("specify", input)],
                },
                { scope: row.scope, component: this.name },
            );
        }

        // stop the draining deployments that captured a draining resource
        if (row.conditions[DRAINING]?.status === "true") {
            await this.#stopDraining(row);
        }

        return undefined;
    }

    /** Read the packages bound to a database, which are the owners of their tables there. */
    async #bound(id: string): Promise<ReadonlySet<string>> {
        const rows = await this.#store.database
            .selectDistinct({ packageId: binding.table.packageId })
            .from(binding.table)
            .where(eq(binding.table.target, id));

        return new Set(rows.map((row) => row.packageId));
    }

    /** Read the platform's tables' state, without the replicated tables of a bound package. */
    #platformState(owning: ReadonlySet<string>): Promise<ReadonlyMap<Digest, ResourceState>> {
        // find the replicated tables of a bound package, which its own state describes
        const main = this.#main;
        const { tables } = main.state();
        const isOwned = (state: TableState) =>
            main.replicated.has(state.table.name) && owning.has(state.package.id);
        const key = tables.sqlite
            .filter(isOwned)
            .map((state) => state.table.name)
            .join(" ");

        // describe the other tables once per set left out
        let read = this.#platform.get(key);
        if (read === undefined) {
            const isKept = (state: TableState) => !isOwned(state);
            const state = {
                tables: {
                    sqlite: tables.sqlite.filter(isKept),
                    postgresql: tables.postgresql.filter(isKept),
                },
            };
            read = Digest.json(state, this.#store.environment.crypto).then(
                (digest) => new Map([[digest, state]]),
            );
            this.#platform.set(key, read);
        }

        return read;
    }

    /** Stop the draining deployments that captured a resource. */
    async #stopDraining(row: Pick<Resource, "id" | "scope">): Promise<void> {
        // find the draining deployments capturing the resource
        const draining = await this.#store.database
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
            this.#store.as(deployment),
            {
                id: RequestId.create(this.#store.environment),
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

    /** Filter the targets to the resources of a served kind. */
    #targets(targets: readonly string[]): string[] {
        return targets.filter((target) =>
            this.#objects.some((object) => object.identifies(target)),
        );
    }

    /** List the resources of every kind in some spaces. */
    async #resourcesIn(
        connection: DatabaseConnection,
        spaces: readonly string[],
    ): Promise<string[]> {
        const resources: string[] = [];
        for (const object of this.#objects) {
            const rows = await connection
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
    connection: DatabaseConnection,
    object: ResourceType<typeof space>,
    row: Resource,
    platform: ReadonlyMap<Digest, ResourceState>,
): Promise<{ readonly active: Digest[]; readonly draining: Digest[] }> {
    // write no states for a kind without a state
    const { kind } = present(object.provisioned, `the kind of ${object.name}`);
    if (!kind.isStateful) {
        return { active: [], draining: [] };
    }

    // read the bindings' states and the states the serving deployments run with
    const bound = await connection
        .select({ state: binding.table.state })
        .from(binding.table)
        .where(and(eq(binding.table.scope, row.scope), eq(binding.table.target, row.id)));
    const captured = await connection
        .select({ state: capture.table.state, status: deployment.table.status })
        .from(capture.table)
        .innerJoin(deployment.table, eq(deployment.table.id, capture.table.deploymentId))
        .where(
            and(
                eq(capture.table.scope, row.scope),
                eq(capture.table.target, row.id),
                Condition.render(Deployment.available(), deployment.table),
            ),
        );

    // list each active state once, then the draining states no active consumer needs
    const active = new Set([
        ...platform.keys(),
        ...bound.map((entry) => entry.state),
        ...captured.filter((entry) => entry.status === "active").map((entry) => entry.state),
    ]);
    const draining = new Set(
        captured
            .filter((entry) => entry.status === "draining" && !active.has(entry.state))
            .map((entry) => entry.state),
    );

    return { active: [...active].toSorted(), draining: [...draining].toSorted() };
}
