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
    isNotNull,
    isNull,
    ne,
    or,
} from "@destack/db";
import { installation, type Resource, type ResourceType } from "@destack/object";
import { DRAINING, type ObjectStore } from "@destack/object/server";
import type { ResourceState } from "@destack/package/declare";
import { Digest, present } from "@destack/schema";
import type { ReconcileController } from "@destack/db";
import {
    binding,
    capture,
    database,
    Deployment,
    deployment,
    installationRevision,
    rollout,
} from "../../object/index.ts";
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
            rollout.table,
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

        // select the space's main database and the resources an installation targeting another revision binds
        if (Change.of(change, rollout.table)) {
            const changed = Change.image(change);
            const main = await connection
                .select({ id: database.table.id })
                .from(database.table)
                .where(
                    and(
                        eq(database.table.scope, changed.scope),
                        eq(database.table.name, MAIN_DATABASE),
                    ),
                );
            const bound = await connection
                .selectDistinct({ target: binding.table.target })
                .from(binding.table)
                .where(eq(binding.table.installationId, changed.id));

            return [
                ...main.map((entry) => entry.id),
                ...this.#targets(bound.map((entry) => entry.target)),
            ];
        }
        // select the resource a changed binding or capture targets
        else if (Change.of(change, binding.table) || Change.of(change, capture.table)) {
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

        // read the states its consumers need by digest, the platform's beside them for the main database
        const isMain = object === database && row.name === MAIN_DATABASE;
        const consumed = await consumedStates(connection, object, row, isMain);
        const platform = isMain
            ? await this.#platformState(new Set(consumed.targeted.map((entry) => entry.packageId)))
            : new Map<Digest, ResourceState>();
        const { active, draining } = desiredStates(consumed, platform);
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
            // write the platform's state as a blob, as the revisions wrote the others
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
        // find the deployments capturing the resource on their way out: draining, or active on a revision their installation no longer targets
        const draining = await this.#store.database
            .selectDistinct({ deployment: deployment.table })
            .from(capture.table)
            .innerJoin(deployment.table, eq(deployment.table.id, capture.table.deploymentId))
            .innerJoin(rollout.table, eq(rollout.table.id, deployment.table.installationId))
            .innerJoin(installation.table, eq(installation.table.id, rollout.table.id))
            .where(
                and(
                    eq(capture.table.scope, row.scope),
                    eq(capture.table.target, row.id),
                    or(
                        eq(deployment.table.status, "draining"),
                        and(
                            eq(deployment.table.status, "active"),
                            or(
                                isNotNull(installation.table.deletedAt),
                                isNull(rollout.table.revisionId),
                                ne(deployment.table.revisionId, rollout.table.revisionId),
                            ),
                        ),
                    ),
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

/** The states a resource's consumers require: targeted revisions' and deployed revisions'. */
interface ConsumedStates {
    /** The state each installation's targeted revision requires, with its package. */
    readonly targeted: readonly { readonly packageId: string; readonly state: Digest }[];
    /** The state each deployment's revision requires, current while its revision is targeted. */
    readonly captured: readonly { readonly isCurrent: boolean; readonly state: Digest }[];
}

/** Read the states consumers require of a resource: all installations for main, else binders. */
async function consumedStates(
    connection: DatabaseConnection,
    object: ResourceType<typeof space>,
    row: Resource,
    isMain: boolean,
): Promise<ConsumedStates> {
    // require no states of a kind without a state
    const { kind } = present(object.provisioned, `the kind of ${object.name}`);
    if (!kind.isStateful) {
        return { targeted: [], captured: [] };
    }

    // read the revisions the consuming installations target, under the declaration they require the state for
    const targeted = isMain
        ? await connection
              .select({
                  packageId: installation.table.packageId,
                  revisionId: rollout.table.revisionId,
              })
              .from(rollout.table)
              .innerJoin(
                  installation.table,
                  and(
                      eq(installation.table.id, rollout.table.id),
                      eq(installation.table.scope, rollout.table.scope),
                  ),
              )
              .where(and(eq(rollout.table.scope, row.scope), isNull(installation.table.deletedAt)))
              .then((rows) => rows.map((entry) => ({ ...entry, name: MAIN_DATABASE })))
        : await connection
              .select({
                  packageId: binding.table.packageId,
                  name: binding.table.name,
                  revisionId: rollout.table.revisionId,
              })
              .from(binding.table)
              .innerJoin(rollout.table, eq(rollout.table.id, binding.table.installationId))
              .where(and(eq(binding.table.scope, row.scope), eq(binding.table.target, row.id)));

    // read the revisions the available deployments capturing it run, and the ones their installations target
    const captured = await connection
        .select({
            name: capture.table.name,
            revisionId: deployment.table.revisionId,
            status: deployment.table.status,
            targetedId: rollout.table.revisionId,
            deletedAt: installation.table.deletedAt,
        })
        .from(capture.table)
        .innerJoin(deployment.table, eq(deployment.table.id, capture.table.deploymentId))
        .innerJoin(rollout.table, eq(rollout.table.id, deployment.table.installationId))
        .innerJoin(installation.table, eq(installation.table.id, rollout.table.id))
        .where(
            and(
                eq(capture.table.scope, row.scope),
                eq(capture.table.target, row.id),
                Condition.render(Deployment.available(), deployment.table),
            ),
        );

    // read each revision's states once, and take the state each consumer's declaration requires
    const ids = [...targeted, ...captured].flatMap((entry) =>
        entry.revisionId === null ? [] : [entry.revisionId],
    );
    const revisions = new Map<string, Readonly<Record<string, Digest>>>(
        (
            await connection
                .select({
                    id: installationRevision.table.id,
                    states: installationRevision.table.states,
                })
                .from(installationRevision.table)
                .where(inArray(installationRevision.table.id, [...new Set(ids)]))
        ).map((entry) => [entry.id, entry.states]),
    );
    const required = <Entry extends { readonly revisionId: string | null; readonly name: string }>(
        entry: Entry,
    ) => {
        const state =
            entry.revisionId === null ? undefined : revisions.get(entry.revisionId)?.[entry.name];

        return state === undefined ? [] : [{ ...entry, state }];
    };

    // count a deployment current while it is active on the revision its live installation targets
    const current = captured.map((entry) => ({
        ...entry,
        isCurrent:
            entry.status === "active" &&
            entry.deletedAt === null &&
            entry.revisionId === entry.targetedId,
    }));

    return { targeted: targeted.flatMap(required), captured: current.flatMap(required) };
}

/** List the distinct desired states of a resource, active and draining, by digest in order. */
function desiredStates(
    consumed: ConsumedStates,
    platform: ReadonlyMap<Digest, ResourceState>,
): { readonly active: Digest[]; readonly draining: Digest[] } {
    // list each targeted or current state once, then the states of deployments on their way out as draining
    const active = new Set([
        ...platform.keys(),
        ...consumed.targeted.map((entry) => entry.state),
        ...consumed.captured.filter((entry) => entry.isCurrent).map((entry) => entry.state),
    ]);
    const draining = new Set(
        consumed.captured
            .filter((entry) => !entry.isCurrent && !active.has(entry.state))
            .map((entry) => entry.state),
    );

    return { active: [...active].toSorted(), draining: [...draining].toSorted() };
}
