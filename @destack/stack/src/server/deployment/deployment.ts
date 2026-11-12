import {
    installation,
    InstallationName,
    type ObjectReconciliation,
    ObjectWatch,
    type StatusCondition,
    InstallationBindingState,
    InstallationKind,
} from "@destack/object";
import { Space, space } from "@destack/account/object";
import { ResourceId } from "@destack/resource";
import {
    and,
    type DatabaseConnection,
    eq,
    inArray,
    isNull,
    now,
    or,
    Snapshot,
    Condition,
} from "@destack/db";
import { PackageError, type PackageId } from "@destack/package";
import {
    type Digest,
    nth,
    type JsonValue,
    schema,
    type Identifier,
    present,
    Version,
} from "@destack/schema";
import { RetryPolicy } from "@destack/environment";
import { Scope } from "@destack/sync";
import { Failure } from "@destack/error";
import {
    InstanceExit,
    RUNTIME_SUPPORT,
    type Runtime,
    type WorkloadInstallation,
    type WorkloadResource,
    type WorkloadServed,
} from "../../runtime/index.ts";
import type { BuildReader } from "@destack/package/manifest";
import {
    Binding,
    Build,
    Database,
    Deployment,
    deployment,
    type Instance,
    instance,
    type InstanceCondition,
    installationRevision,
    type InstallationRevision,
    dependency,
} from "../../object/index.ts";
import { MAIN_DATABASE } from "../../declare/index.ts";
import { StackError } from "../../error/index.ts";
import type { ProviderIndex } from "../resource/resource.ts";
import type { ObjectBindable } from "../resource/binding.ts";
import type { SpaceBuildStore } from "../installation/build.ts";
import { RolloutTarget } from "../installation/target.ts";
import type { Kind, Provisioning } from "@destack/package";

/** The instance statuses of a recorded workload not stopped for good. */
const UNSTOPPED_STATUSES = ["starting", "running", "failed"] as const;

/** How a failed instance restarts: a second, doubling, at most a minute apart. */
const RESTART = RetryPolicy.of({ maximumInterval: 60_000 });

/** What a machine runs deployments with: its identity, its runtimes and its builds. */
export interface DeploymentOptions {
    /** The machine serving the spaces. */
    readonly machine: Identifier<"machine">;
    /** The runtimes running the instances, by server runtime. */
    readonly runtimes: ReadonlyMap<string, Runtime>;
    /** The space's builds, read through to the machine's cache. */
    readonly builds: SpaceBuildStore;
    /** The providers of the machine's resource kinds, with the object each kind's resources are. */
    readonly providers: ProviderIndex;
    /** The kinds of package objects that bindings target other than resources, such as secrets. */
    readonly bindables: readonly ObjectBindable[];
}

/** A resource a deployment binds, with its kind and readiness. */
interface BoundResource {
    /** The package declaring the binding. */
    readonly packageId: PackageId;
    /** The declaration's name within its package. */
    readonly name: string;
    /** The resource. */
    readonly id: ResourceId;
    /** The space of the resource. */
    readonly scope: Identifier<"space">;
    /** The resource kind, such as database. */
    readonly kind: Kind<schema.Schema, Provisioning>["name"];
    /** The declared specification. */
    readonly spec: JsonValue;
    /** The provider's reference once provisioned. */
    readonly reference: string | null;
    /** The provider of the resource once provisioned. */
    readonly provider: string | null;
    /** The desired state's generation. */
    readonly generation: number;
    /** The conditions the controller reports. */
    readonly conditions: Readonly<Record<string, StatusCondition>>;
}

/** Implement deployments: roll each out on this machine's runtimes. */
export function implementDeployment(options: DeploymentOptions) {
    return deployment.implement({
        controllers: [
            {
                pending: { status: { in: ["active", "draining", "stopping"] } },
                watches: [
                    // wake an instance's deployment and the deployments of its workload it may replace
                    ObjectWatch.of(instance.table, async (row, database) => [
                        { id: row.deploymentId },
                        ...(await replacedBy(row.deploymentId, database)),
                    ]),
                    // wake the space's draining deployments when another turns active or retires
                    ObjectWatch.of(deployment.table, async (row, database) =>
                        row.status === "active" || row.status === "retired"
                            ? database
                                  .select({ id: deployment.table.id })
                                  .from(deployment.table)
                                  .where(
                                      and(
                                          eq(deployment.table.scope, row.scope),
                                          eq(deployment.table.status, "draining"),
                                      ),
                                  )
                            : [],
                    ),
                    // wake a space's live deployments once a move fences or lifts its scope or its placement changes
                    ObjectWatch.of(Scope.table, async (row, database) => {
                        const fenced = schema.identifier("space").safeParse(row.scope);

                        return fenced.success ? liveDeployments(fenced.data, database) : [];
                    }),
                    Space.watch(liveDeployments),
                    // reconcile the live deployments whose revision binds a changed resource of any kind, or keeps tables in a changed main
                    ...options.providers.objects().map((object) =>
                        ObjectWatch.of(object.table, async (row, connection) => {
                            // leave a resource outside spaces
                            const scope = schema.identifier("space").safeParse(row.scope);
                            if (!scope.success) {
                                return [];
                            }
                            const isMain = Database.isMain(object, row);
                            const live = await connection
                                .select()
                                .from(deployment.table)
                                .where(
                                    and(
                                        eq(deployment.table.scope, scope.data),
                                        Condition.render(Deployment.live(), deployment.table),
                                    ),
                                );
                            const revisions = await runningRevisions(connection, scope.data);

                            return live
                                .filter((entry) => {
                                    const revision = revisions.find(
                                        (each) => each.id === entry.revisionId,
                                    );

                                    return (
                                        revision !== undefined &&
                                        (isMain
                                            ? revision.states[MAIN_DATABASE] !== undefined
                                            : revision.bindings.some(
                                                  (bound) => bound.target === row.id,
                                              ))
                                    );
                                })
                                .map((entry) => ({ id: entry.id }));
                        }),
                    ),
                ],
                reconcile: async (reconciliation) => {
                    // reconcile a deployment of a served space the machine runs
                    const id = nth(reconciliation.rows, 0).id;
                    const pass = await Reconcile.read(id, reconciliation, options);

                    return pass?.reconcile();
                },
            },
        ],
    });
}

/** Implement instances: record one stopped. */
export function implementInstance() {
    return instance.implement({
        methods: {
            stop: (call) =>
                call.update({ status: "stopped", stoppedAt: call.now, observedAt: call.now }),
        },
        controllers: [
            {
                // clear a failed instance's restart once its backoff passed, which wakes its deployment
                name: "restart",
                pending: { status: "failed", restartAt: { lte: now } },
                reconcile: async ({ rows, mutate }) => {
                    await mutate(
                        rows.map((row) =>
                            instance.call("update", { ...instance.target(row), restartAt: null }),
                        ),
                    );
                },
            },
        ],
    });
}

/** One reconcile of one deployment of a served space the machine runs, with its instances. */
class Reconcile {
    /** The deployment. */
    readonly #current: Deployment;
    /** Its instances not stopped for good. */
    readonly #recorded: readonly Instance[];
    /** The reconciliation running as the store's principal. */
    readonly #reconciliation: ObjectReconciliation;
    /** What the machine runs deployments with. */
    readonly #options: DeploymentOptions;

    /** Create the rollout of a deployment with its recorded instances. */
    constructor(
        current: Deployment,
        recorded: readonly Instance[],
        reconciliation: ObjectReconciliation,
        options: DeploymentOptions,
    ) {
        // store the deployment with its instances, the reconciliation and the options
        this.#current = current;
        this.#recorded = recorded;
        this.#reconciliation = reconciliation;
        this.#options = options;
    }

    /** Read a deployment of a served space the machine runs, with its recorded instances. */
    static async read(
        id: string,
        reconciliation: ObjectReconciliation,
        options: DeploymentOptions,
    ): Promise<Reconcile | undefined> {
        // read the deployment of a served space that the machine runs
        const database = reconciliation.database;
        const [served] = await database
            .select({ deployment: deployment.table })
            .from(deployment.table)
            .innerJoin(space.table, eq(space.table.id, deployment.table.scope))
            .where(
                and(
                    eq(deployment.table.id, schema.identifier("deployment").parse(id)),
                    Space.served(),
                    selectAssigned(options.machine),
                ),
            );
        if (served === undefined) {
            return undefined;
        }

        // read its recorded instances
        const current = served.deployment;
        const recorded = await database
            .select()
            .from(instance.table)
            .where(
                and(
                    eq(instance.table.deploymentId, current.id),
                    inArray(instance.table.status, [...UNSTOPPED_STATUSES]),
                ),
            );

        return new Reconcile(current, recorded, reconciliation, options);
    }

    /** Run the instance of an active deployment, and stop the instances of one no longer active. */
    async reconcile(): Promise<void> {
        // run the active deployment's instance
        if (this.#current.status === "active") {
            return this.#runActive();
        }
        // let a draining deployment serve until the deployment replacing it runs
        else if (
            this.#current.status === "draining" &&
            this.#recorded.length > 0 &&
            !(await this.#isReplaced())
        ) {
            return;
        }
        // stop the instances of a deployment no longer active, running only this machine's
        else if (this.#recorded.length > 0) {
            const own = this.#recorded.filter((entry) => entry.machineId === this.#options.machine);
            for (const stopped of own) {
                await this.#runtime().stop(stopped.id);
            }
            await this.#reconciliation.mutate(
                this.#recorded.map((row) => instance.call("stop", instance.target(row))),
            );
        }
        // retire a draining or stopping deployment once none of its instances runs
        else if (this.#current.status === "draining" || this.#current.status === "stopping") {
            await this.#reconciliation.mutate([
                deployment.call("update", {
                    ...deployment.target(this.#current),
                    status: "retired",
                    retiredAt: this.#reconciliation.now,
                }),
            ]);
        }

        return;
    }

    /** Run an active deployment's instance on this machine, stopping other machines' instances. */
    async #runActive(): Promise<void> {
        // stop the instances of other machines
        const machine = this.#options.machine;
        const foreign = this.#recorded.filter((entry) => entry.machineId !== machine);
        await this.#reconciliation.mutate(
            foreign.map((row) => instance.call("stop", instance.target(row))),
        );

        // run this machine's instance or start one when none is recorded
        const own = this.#recorded.find((entry) => entry.machineId === machine);

        return this.#run(own ?? (await this.#start()));
    }

    /** Report whether a draining deployment may stop. */
    async #isReplaced(): Promise<boolean> {
        // stop once no active deployment replaces the workload
        const current = this.#current;
        const database = this.#reconciliation.database;
        const [replacing] = await database
            .select({ id: deployment.table.id, since: deployment.table.since })
            .from(deployment.table)
            .where(
                and(
                    eq(deployment.table.installationId, current.installationId),
                    eq(deployment.table.workload, current.workload),
                    eq(deployment.table.status, "active"),
                ),
            );
        if (replacing === undefined) {
            return true;
        }

        // serve until the replacement runs an instance
        const [running] = await database
            .select({ id: instance.table.id })
            .from(instance.table)
            .where(
                and(
                    eq(instance.table.deploymentId, replacing.id),
                    eq(instance.table.status, "running"),
                ),
            );
        if (running === undefined) {
            return false;
        }
        // stop once the replacement serves every release
        else if (replacing.since === null) {
            return true;
        }

        return this.#servesPinned(replacing.since);
    }

    /** Report whether every serving caller in the space is pinned to a release since one. */
    async #servesPinned(since: Version): Promise<boolean> {
        // read the pinned releases of the serving callers of the deployment's installation
        const current = this.#current;
        const database = this.#reconciliation.database;
        const [called] = await database
            .select({ name: installation.table.name })
            .from(installation.table)
            .where(eq(installation.table.id, current.installationId));
        const name = present(
            present(called, `installation ${current.installationId}`).name,
            `the name of installation ${current.installationId}`,
        );
        const dependencies = await database
            .select({ id: dependency.table.id })
            .from(dependency.table)
            .where(
                and(
                    eq(dependency.table.scope, current.scope),
                    eq(dependency.table.reference, name),
                ),
            );
        const callers = new Set<string>(dependencies.map((entry) => entry.id));
        const pinned = (await runningRevisions(database, current.scope)).flatMap((revision) =>
            revision.bindings
                .filter((entry) => entry.targetSpaceId === null && callers.has(entry.target))
                .map((entry) => ({ name: entry.name, states: revision.states })),
        );

        // serve while one caller is pinned to a release the replacement no longer serves, as its revision records
        const { blobs } = this.#reconciliation.store;
        const releases = await Promise.all(
            pinned.map(async (entry) => {
                const state = present(
                    entry.states[entry.name],
                    `the pinned release of ${entry.name}`,
                );

                return (await blobs.readJson(state, InstallationBindingState)).release;
            }),
        );

        return releases.every((release) => Version.compare(release, since) >= 0);
    }

    /** Record a new instance of the deployment on this machine. */
    async #start(): Promise<Instance> {
        // record the instance starting on this machine
        const current = this.#current;
        const [started] = await this.#reconciliation.mutate([
            instance.call("create", {
                ...instance.in(current.scope),
                deploymentId: current.id,
                machineId: this.#options.machine,
                status: "starting",
                observedAt: this.#reconciliation.now,
            }),
        ]);

        return present(started, `the instance of ${current.id}`);
    }

    /** Run an instance after its bound resources apply. */
    async #run(recorded: Instance): Promise<void> {
        // leave a running instance running while its process runs
        const reconciliation = this.#reconciliation;
        const running = this.#runtime();
        if (recorded.status === "running" && running.isRunning(recorded.id)) {
            return;
        }
        // leave a failed instance until its backoff passes
        else if (recorded.status === "failed" && recorded.restartAt !== null) {
            return;
        }

        // wait until every bound resource applied its current generation, observing which
        const bindings = await this.#readBindings();
        const resources = await this.#readResources(bindings);
        const waiting = Reconcile.#waiting(resources);
        if (waiting.length > 0) {
            const names = waiting.map((entry) => entry.id).join(", ");
            await this.#observe(recorded, {
                status: recorded.status === "failed" ? "starting" : recorded.status,
                ready: ["false", "WaitingForResources", `resources ${names} are not applied`],
            });

            return;
        }

        // run the revision's build and restart a failed instance in place
        const restarts = recorded.status === "failed" ? recorded.restarts + 1 : recorded.restarts;
        const snapshot = Snapshot.live(reconciliation.database);
        const revision = present(
            await snapshot
                .read(installationRevision.table)
                .findFirst({ where: { id: this.#current.revisionId } }),
            `revision ${this.#current.revisionId}`,
        );
        const built = await Build.require(snapshot, revision.scope, revision.buildId);
        try {
            await this.#launch(recorded, built.lock, bindings, resources, running);
        } catch (error) {
            // record the failure on the instance and restart it after its backoff
            return this.#recordFailure(recorded, restarts, error);
        }

        // record the instance running
        const startedAt = reconciliation.store.clock.now();
        await this.#observe(recorded, {
            status: "running",
            restarts,
            startedAt,
            stoppedAt: null,
            ready: ["true", "Running", ""],
        });
    }

    /** Record an instance failed to start, restarting after a backoff drawn from its restarts. */
    async #recordFailure(recorded: Instance, restarts: number, error: unknown): Promise<void> {
        // record the failure, restarting after the backoff drawn from the instance and its restarts
        const { message } = Failure.of(error);
        const stoppedAt = this.#reconciliation.store.clock.now();
        const drawn = RetryPolicy.draw(`${recorded.id} ${restarts + 1}`);
        const backoff = RetryPolicy.interval(RESTART, restarts + 1, () => drawn);
        await this.#observe(recorded, {
            status: "failed",
            restarts,
            stoppedAt,
            restartAt: stoppedAt + backoff,
            ready: ["false", "StartFailed", message],
        });
    }

    /** List the bound resources yet to apply their current generation. */
    static #waiting(resources: readonly BoundResource[]): BoundResource[] {
        return resources.filter(
            (entry) =>
                entry.conditions["Ready"]?.status !== "true" ||
                entry.conditions["Ready"].observedGeneration !== entry.generation,
        );
    }

    /** Start an instance's workload on its runtime from the revision's build with its resources. */
    async #launch(
        recorded: Instance,
        lock: Digest,
        bindings: readonly Binding[],
        resources: readonly BoundResource[],
        running: Runtime,
    ): Promise<void> {
        // refuse what the runtime cannot grant
        const current = this.#current;
        this.#refuseUnsupported(running);

        // start the workload as its installation, by its name, with its bindings, and record an unasked exit
        const reader = await this.#options.builds.root(lock);
        const installed = present(
            await Snapshot.live(this.#reconciliation.database)
                .read(installation.table)
                .findFirst({
                    where: {
                        id: current.installationId,
                    },
                }),
            `the installation ${current.installationId}`,
        );
        await running.start(
            {
                instanceId: recorded.id,
                installationId: current.installationId,
                name: present(installed.name, `the name of installation ${installed.id}`),
                scope: current.scope,
                deploymentId: current.id,
                build: reader,
                lock,
                output: current.output,
                workload: current.workload,
                sandbox: current.description.sandbox,
                enabled: current.enabled,
                directories: current.directories,
                served: this.#served(bindings),
                installations: await Reconcile.#installations(bindings, reader),
                resources: resources
                    .filter((entry) => Reconcile.#isUsed(current.description, entry))
                    .map((entry) => this.#workloadResource(entry)),
                settings: await RolloutTarget.settings(this.#reconciliation.database, reader, {
                    scope: current.scope,
                    installation: current.installationId,
                }),
            },
            (exit) => this.#recordExit(recorded.id, exit),
        );
    }

    /** Report whether a workload uses a bound resource, as its description lists it. */
    static #isUsed(
        description: Deployment["description"],
        resource: Pick<BoundResource, "packageId" | "name">,
    ): boolean {
        return description.resources.some(
            (used) => used.packageId === resource.packageId && used.name === resource.name,
        );
    }

    /** Refuse files on a runtime without a filesystem. */
    #refuseUnsupported(running: Runtime): void {
        // refuse files on a runtime without a filesystem to grant
        const current = this.#current;
        const granted = RUNTIME_SUPPORT[running.name];
        if (current.description.sandbox.files !== undefined && !granted.hasFilesystem) {
            throw PackageError.of(
                "ABILITY_INVALID",
                `filesystem ability is invalid: the '${current.runtime}' runtime has no filesystem to grant`,
                { data: { ability: "files" } },
            );
        }
    }

    /** Record a running instance failed when its workload exits unasked. */
    async #recordExit(instanceId: Identifier<"instance">, exit: InstanceExit): Promise<void> {
        const reconciliation = this.#reconciliation;
        const [recorded] = await reconciliation.database
            .select()
            .from(instance.table)
            .where(eq(instance.table.id, instanceId));
        if (recorded?.status === "running") {
            await this.#observe(recorded, {
                status: "failed",
                stoppedAt: reconciliation.store.clock.now(),
                ready: ["false", "Exited", InstanceExit.describe(exit)],
            });
        }
    }

    /** Find the runtime the deployment runs on, refusing one this machine does not run. */
    #runtime(): Runtime {
        const running = this.#options.runtimes.get(this.#current.runtime);
        if (running === undefined) {
            throw StackError.of(
                "INSTALLATION_BLOCKED",
                `cannot start the installation: this machine has no '${this.#current.runtime}' runtime`,
            );
        }

        return running;
    }

    /** Read the bound resources of each kind the providers serve, by the bindings' targets. */
    async #readResources(bindings: readonly Binding[]): Promise<BoundResource[]> {
        // read each kind's bound resources at once
        const providers = this.#options.providers;
        const found: BoundResource[] = [];
        for (const object of providers.objects()) {
            const bound = bindings.filter((entry) => object.identifies(entry.target));
            if (bound.length === 0) {
                continue;
            }
            const rows = await this.#reconciliation.database
                .select()
                .from(object.table)
                .where(object.withIds(bound.map((entry) => entry.target)));

            // pair each binding with its resource
            const kind = providers.kind(object).name;
            for (const entry of bound) {
                const row = present(
                    rows.find((each) => each.id === entry.target),
                    `resource ${entry.target}`,
                );
                found.push({
                    packageId: entry.packageId,
                    name: entry.name,
                    id: row.id,
                    scope: row.scope,
                    kind,
                    spec: row.spec,
                    reference: row.reference,
                    provider: row.provider,
                    generation: row.generation,
                    conditions: row.conditions,
                });
            }
        }

        return found;
    }

    /** Read what the deployment runs with: its revision's bindings, and main if it keeps tables. */
    async #readBindings(): Promise<Binding[]> {
        // read the revision the deployment runs
        const current = this.#current;
        const snapshot = Snapshot.live(this.#reconciliation.database);
        const revision = present(
            await snapshot
                .read(installationRevision.table)
                .findFirst({ where: { id: current.revisionId } }),
            `revision ${current.revisionId}`,
        );

        // add the space's main database at its current generation, which its migrations move
        if (revision.states[MAIN_DATABASE] === undefined) {
            return revision.bindings;
        }
        const main = await Database.main(snapshot, current.scope);

        return [
            ...revision.bindings,
            {
                packageId: current.packageId,
                name: MAIN_DATABASE,
                target: main.id,
                targetSpaceId: null,
                version: main.generation,
            },
        ];
    }

    /** Select the served objects among the deployment's bindings, such as secret versions. */
    #served(rows: readonly Binding[]): WorkloadServed[] {
        return rows.flatMap((row) => {
            // take the versioned bindings of a served kind
            const kind = this.#options.bindables.find((each) => each.object.identifies(row.target));

            return kind !== undefined && row.version !== null
                ? [
                      {
                          packageId: row.packageId,
                          name: row.name,
                          kind: kind.kind.name,
                          provider: kind.provider,
                          target: ResourceId.parse(row.target),
                          version: row.version,
                          reference: kind.reference,
                      },
                  ]
                : [];
        });
    }

    /** Select the installations of other spaces among a deployment's bindings. */
    static async #installations(
        bindings: readonly Binding[],
        build: BuildReader,
    ): Promise<WorkloadInstallation[]> {
        // read the package each bound declaration calls from the build
        const rows = bindings.filter((row) => row.targetSpaceId !== null);
        const declarations = rows.length === 0 ? [] : await build.declarations();

        return rows.map((row) => {
            const declared = declarations.find(
                (declaration) =>
                    declaration.provisioned === true &&
                    declaration.name === row.name &&
                    declaration.moniker.startsWith(`${row.packageId}/`),
            );
            const description = present(
                declared,
                `the declaration ${row.name} of ${row.packageId}`,
            );

            return {
                packageId: row.packageId,
                name: row.name,
                package: InstallationKind.description.parse(description.description).spec.package,
                installation: InstallationName.parse(row.target),
                scope: present(row.targetSpaceId, `the space of ${row.target}`),
            };
        });
    }

    /** Read a bound resource as a workload binds it, refusing one not provisioned. */
    #workloadResource(bound: BoundResource): WorkloadResource {
        // require the provider of an applied resource
        const { generation: _generation, conditions: _conditions, ...entry } = bound;
        if (entry.provider === null || entry.reference === null) {
            throw StackError.of(
                "RESOURCE_UNAVAILABLE",
                `resource '${entry.id}' is unavailable: it is not provisioned`,
            );
        }

        // give the workload the resource's provider and reference on this machine
        const reached = { kind: entry.kind, provider: entry.provider, reference: entry.reference };

        return {
            ...entry,
            provider: entry.provider,
            reference: this.#options.providers.resolve(reached),
        };
    }

    /** Record an instance's fields and ready condition, unless it has them already. */
    async #observe(
        recorded: Instance,
        observed: {
            /** The instance status. */
            readonly status: Instance["status"];
            /** The ready condition's status, reason and message. */
            readonly ready: readonly [InstanceCondition["status"], string, string];
            /** The restarts after failures. */
            readonly restarts?: number;
            /** The start time. */
            readonly startedAt?: number;
            /** The finish or failure time. */
            readonly stoppedAt?: number | null;
            /** When a failed instance restarts. */
            readonly restartAt?: number | null;
        },
    ): Promise<void> {
        // skip an observation the instance has
        const {
            ready: [status, reason, message],
            ...fields
        } = observed;
        const previous = recorded.conditions["Ready"];
        const columns: Readonly<Record<string, unknown>> = recorded;
        const isRecorded =
            previous?.status === status &&
            previous.reason === reason &&
            previous.message === message &&
            Object.entries(fields).every(([name, value]) => columns[name] === value);
        if (isRecorded) {
            return;
        }

        // record the fields and the condition, with a transition time that stays while the status stays
        const time = this.#reconciliation.now;
        const ready: InstanceCondition = {
            status,
            reason,
            message,
            lastTransitionAt: previous?.status === status ? previous.lastTransitionAt : time,
        };
        await this.#reconciliation.mutate([
            instance.call("update", {
                ...instance.target(recorded),
                ...fields,
                conditions: { ...recorded.conditions, Ready: ready },
                observedAt: time,
            }),
        ]);
    }
}

/** Select a space's live deployments. */
function liveDeployments(spaceId: Identifier<"space">, database: DatabaseConnection) {
    return database
        .select({ id: deployment.table.id })
        .from(deployment.table)
        .where(
            and(
                eq(deployment.table.scope, spaceId),
                Condition.render(Deployment.live(), deployment.table),
            ),
        );
}

/** Select the draining deployments of a deployment's workload. */
async function replacedBy(
    deploymentId: Identifier<"deployment">,
    database: DatabaseConnection,
): Promise<{ readonly id: Identifier<"deployment"> }[]> {
    // read the deployment's workload
    const [current] = await database
        .select({
            installationId: deployment.table.installationId,
            workload: deployment.table.workload,
        })
        .from(deployment.table)
        .where(eq(deployment.table.id, deploymentId));
    if (current === undefined) {
        return [];
    }

    return database
        .select({ id: deployment.table.id })
        .from(deployment.table)
        .where(
            and(
                eq(deployment.table.installationId, current.installationId),
                eq(deployment.table.workload, current.workload),
                eq(deployment.table.status, "draining"),
            ),
        );
}

/** Match the deployments the machine runs: those selecting it, or selecting none. */
function selectAssigned(machine: Identifier<"machine">) {
    return or(isNull(deployment.table.machineId), eq(deployment.table.machineId, machine));
}

/** Read the revisions a space's available deployments run. */
async function runningRevisions(
    database: DatabaseConnection,
    scope: Identifier<"space">,
): Promise<InstallationRevision[]> {
    const running = await database
        .select({ id: deployment.table.revisionId })
        .from(deployment.table)
        .where(
            and(
                eq(deployment.table.scope, scope),
                Condition.render(Deployment.available(), deployment.table),
            ),
        );

    return database
        .select()
        .from(installationRevision.table)
        .where(
            inArray(installationRevision.table.id, [...new Set(running.map((entry) => entry.id))]),
        );
}
