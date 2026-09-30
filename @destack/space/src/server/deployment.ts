import { and, type DatabaseConnection, eq, inArray, isNull, or } from "@destack/db";
import { Condition } from "@destack/db/query";
import type { ObjectControl, StatusCondition } from "@destack/object";
import { SystemCall } from "@destack/object/server";
import { RetryPolicy } from "@destack/service/timer";
import { identifier, type Identifier, Version } from "@destack/schema";
import type { Runtime } from "@destack/host/runtime";
import { ServiceBindingState, ServiceKind } from "@destack/service/declare";
import {
    capture,
    Deployment,
    deployment,
    type Instance,
    instance,
    installation,
    installationRevision,
    resource,
    space,
    SpaceCell,
} from "../object/index.ts";
import { SpaceError } from "../error/index.ts";
import type { OpenBuild } from "./installation/release.ts";

/** The instance statuses of a recorded workload not stopped for good. */
const RECORDED = ["starting", "running", "failed"] as const;

/** How a failed instance restarts: a second, doubling, at most a minute apart. */
const RESTART = RetryPolicy.of({ maximumInterval: 60_000 });

/** The generation of the immutable deployment an instance runs. */
const DEPLOYMENT_GENERATION = 1;

/** What a cell runs deployments with: its host or region, its runtimes and its builds. */
export interface DeploymentOptions {
    /** The host or region running the deployments. */
    readonly cell: SpaceCell;
    /** The runtimes running the instances, by server runtime. */
    readonly runtimes: ReadonlyMap<string, Runtime>;
    /** Open a package's build through the cell's store of builds. */
    readonly openBuild: OpenBuild;
}

/** Serve deployments and instances: record instances starting and stopping, and keep one instance of each active deployment the cell runs. */
export function serveDeployments(options: DeploymentOptions) {
    return {
        deployment: deployment
            .handle({
                start: async (call) => {
                    // record an instance starting on the host
                    const target = call.target as unknown as Deployment;
                    const { hostId } = call.input as { readonly hostId: Identifier<"host"> };

                    return call.invoke(instance, "create", {
                        deploymentId: target.id,
                        hostId,
                        status: "starting",
                        observedAt: call.now,
                    });
                },
            })
            .control({
                pending: Condition.oneOf("status", ["active", "draining", "stopping"]),
                watches: [
                    {
                        // wake an instance's deployment and the deployments of its workload it may replace
                        table: instance.table,
                        keys: async (row, database) => [
                            { id: row.deploymentId },
                            ...(await replacedBy(
                                row.deploymentId as Identifier<"deployment">,
                                database,
                            )),
                        ],
                    },
                    {
                        // wake the space's draining deployments when another turns active or retires
                        table: deployment.table,
                        keys: async (row, database) =>
                            row.status === "active" || row.status === "retired"
                                ? database
                                      .select({ id: deployment.table.id })
                                      .from(deployment.table)
                                      .where(
                                          and(
                                              eq(
                                                  deployment.table.scope,
                                                  row.scope as Identifier<"space">,
                                              ),
                                              eq(deployment.table.status, "draining"),
                                          ),
                                      )
                                : [],
                    },
                    {
                        // reconcile the live deployments capturing a changed resource
                        table: resource.table,
                        keys: async (row, database) =>
                            database
                                .selectDistinct({ id: capture.table.deploymentId })
                                .from(capture.table)
                                .innerJoin(
                                    deployment.table,
                                    eq(deployment.table.id, capture.table.deploymentId),
                                )
                                .where(
                                    and(
                                        eq(capture.table.scope, row.scope as Identifier<"space">),
                                        eq(capture.table.target, row.id as string),
                                        Deployment.live(),
                                    ),
                                ),
                    },
                ],
                reconcile: (control) =>
                    reconcileDeployment(control.rows[0]!.id as string, control, options),
            }),
        instance: instance.handle({
            stop: (call) =>
                call.revise({ status: "stopped", stoppedAt: call.now, observedAt: call.now }),
        }),
    };
}

/** Run the instance of an active deployment, stop the instances of one no longer active, and retire a drained one. */
async function reconcileDeployment(
    id: string,
    control: ObjectControl,
    options: DeploymentOptions,
): Promise<number | undefined> {
    // read the deployment of a served space that the cell runs, and its recorded instances
    const database = control.database;
    const [served] = await database
        .select({ deployment: deployment.table })
        .from(deployment.table)
        .innerJoin(space.table, eq(space.table.id, deployment.table.scope))
        .where(
            and(
                eq(deployment.table.id, identifier("deployment").parse(id)),
                SpaceCell.served(),
                runs(options.cell),
            ),
        );
    if (served === undefined) {
        return undefined;
    }
    const current = served.deployment;
    const recorded = await database
        .select()
        .from(instance.table)
        .where(
            and(
                eq(instance.table.deploymentId, current.id),
                inArray(instance.table.status, [...RECORDED]),
            ),
        );
    const now = control.now;

    // run the active deployment's instance, and stop the instances of other hosts
    if (current.status === "active") {
        const host = SpaceCell.host(options.cell);
        const foreign = host === null ? [] : recorded.filter((entry) => entry.hostId !== host);
        if (foreign.length > 0) {
            await control.server.executeAsSystem(
                instance,
                "stop",
                foreign.map((row) => SystemCall.of(row)),
                now,
            );
        }
        const own = recorded.find((entry) => host === null || entry.hostId === host);

        return run(
            current,
            own ?? (await start(current, now, control, options)),
            now,
            control,
            options,
        );
    }
    // keep a draining deployment serving until the deployment replacing it runs
    else if (
        current.status === "draining" &&
        recorded.length > 0 &&
        !(await isReplaced(current, control))
    ) {
        return undefined;
    }
    // stop the instances of a deployment no longer active
    else if (recorded.length > 0) {
        for (const stopped of recorded) {
            await runtime(current, options).stop(stopped.id);
        }
        await control.server.executeAsSystem(
            instance,
            "stop",
            recorded.map((row) => SystemCall.of(row)),
            now,
        );
    }
    // retire a draining or stopping deployment once none of its instances runs
    else if (current.status === "draining" || current.status === "stopping") {
        await control.server.executeAsSystem(
            deployment,
            "update",
            [{ ...SystemCall.of(current), input: { status: "retired", retiredAt: now } }],
            now,
        );
    }

    return undefined;
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

/** Report whether a draining deployment may stop: nothing replaces its workload, or its replacement runs and serves every release a caller in its space pins. */
async function isReplaced(current: Deployment, control: ObjectControl): Promise<boolean> {
    // stop once no active deployment replaces the workload, as after a suspension
    const database = control.database;
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

    // keep serving until the replacement runs an instance
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
    } else if (replacing.since === null) {
        return true;
    }

    // keep serving while a serving caller in the space pins a release the replacement no longer serves
    const since = replacing.since;
    const [called] = await database
        .select({ alias: installation.table.alias })
        .from(installation.table)
        .where(eq(installation.table.id, current.installationId));
    const pinned = await database
        .select({ state: capture.table.state })
        .from(capture.table)
        .innerJoin(resource.table, eq(resource.table.id, capture.table.target))
        .innerJoin(deployment.table, eq(deployment.table.id, capture.table.deploymentId))
        .where(
            and(
                eq(resource.table.scope, current.scope),
                eq(resource.table.kind, ServiceKind.name),
                eq(resource.table.reference, called!.alias),
                Deployment.serving(),
            ),
        );

    return pinned.every(
        (entry) => Version.compare(ServiceBindingState.parse(entry.state).release, since) >= 0,
    );
}

/** Record a new instance of a deployment on this cell's host, admitting the host to the space first. */
async function start(
    current: Deployment,
    now: number,
    control: ObjectControl,
    options: DeploymentOptions,
): Promise<Instance> {
    // TODO #Incomplete: schedule a region's instances onto the hosts serving it
    const hostId = SpaceCell.host(options.cell);
    if (hostId === null) {
        throw new SpaceError(
            "NOT_IMPLEMENTED",
            `region ${SpaceCell.id(options.cell)} schedules no instances`,
        );
    }

    // record the instance starting
    const [started] = await control.server.executeAsSystem(
        deployment,
        "start",
        [{ ...SystemCall.of(current), input: { hostId } }],
        now,
    );

    return started as Instance;
}

/** Run an instance after its captured resources apply and return the delay before the next restart. */
async function run(
    current: Deployment,
    recorded: Instance,
    now: number,
    control: ObjectControl,
    options: DeploymentOptions,
): Promise<number | undefined> {
    // leave a running instance running while its process runs, and a failed one until its backoff passes
    const backoff = RetryPolicy.interval(RESTART, recorded.restarts + 1);
    const due = (recorded.stoppedAt ?? now) + backoff;
    const running = runtime(current, options);
    if (recorded.status === "running" && running.isRunning(recorded.id)) {
        return undefined;
    } else if (recorded.status === "failed" && due > now) {
        return due - now;
    }

    // wait, observing why, until every captured resource applied its current generation
    const resources = await captured(current, control);
    const waiting = resources.filter(
        (entry) =>
            entry.conditions.ready?.status !== "true" ||
            entry.conditions.ready.observedGeneration !== entry.generation,
    );
    if (waiting.length > 0) {
        const names = waiting.map((entry) => entry.id).join(", ");
        await observe(
            recorded,
            {
                status: recorded.status === "failed" ? "starting" : recorded.status,
                ready: ["false", "WaitingForResources", `resources ${names} are not applied`],
            },
            control,
        );

        return undefined;
    }

    // run the workload, restarting a failed instance in place
    const restarts = recorded.status === "failed" ? recorded.restarts + 1 : recorded.restarts;
    const [revision] = await control.database
        .select({ build: installationRevision.table.build })
        .from(installationRevision.table)
        .where(eq(installationRevision.table.id, current.revisionId));
    try {
        await running.start(
            {
                instanceId: recorded.id,
                installationId: current.installationId,
                scope: current.scope,
                deploymentId: current.id,
                build: await options.openBuild(current.packageId, revision!.build),
                output: current.output,
                workload: current.workload,
                resources: resources.map(
                    ({ generation: _generation, conditions: _conditions, ...entry }) => {
                        // require the provider of an applied resource
                        if (entry.providerCode === null || entry.reference === null) {
                            throw new SpaceError(
                                "NOT_READY",
                                `resource ${entry.id} is not provisioned`,
                            );
                        }

                        return { ...entry, providerCode: entry.providerCode };
                    },
                ),
            },
            (code) => exited(recorded.id, code, control),
        );
    } catch (error) {
        // record the failure on the instance, restarting it after its backoff
        const message = error instanceof Error ? error.message : String(error);
        const failedAt = Date.now();
        await observe(
            recorded,
            {
                status: "failed",
                restarts,
                stoppedAt: failedAt,
                ready: ["false", "StartFailed", message],
            },
            control,
        );

        return RetryPolicy.interval(RESTART, restarts + 1);
    }

    // record the instance running
    const startedAt = Date.now();
    await observe(
        recorded,
        {
            status: "running",
            restarts,
            startedAt,
            stoppedAt: null,
            ready: ["true", "Running", ""],
        },
        control,
    );

    return undefined;
}

/** Record a running instance failed when its workload exits unasked, restarting it after its backoff. */
async function exited(
    instanceId: Identifier<"instance">,
    code: number,
    control: ObjectControl,
): Promise<void> {
    const [recorded] = await control.database
        .select()
        .from(instance.table)
        .where(eq(instance.table.id, instanceId));
    if (recorded?.status === "running") {
        await observe(
            recorded,
            {
                status: "failed",
                stoppedAt: Date.now(),
                ready: ["false", "Exited", `the workload exited with code ${code}`],
            },
            control,
        );
    }
}

/** Find the runtime a deployment runs on, refusing one this cell does not run. */
function runtime(current: Deployment, options: DeploymentOptions): Runtime {
    const running = options.runtimes.get(current.runtime);
    if (running === undefined) {
        throw new SpaceError("NOT_IMPLEMENTED", `this cell runs no ${current.runtime} runtime`);
    }

    return running;
}

/** Read the resources a deployment captured, with their readiness. */
function captured(current: Deployment, control: ObjectControl) {
    return control.database
        .select({
            packageId: capture.table.packageId,
            name: capture.table.name,
            id: resource.table.id,
            scope: resource.table.scope,
            kind: resource.table.kind,
            spec: resource.table.spec,
            reference: resource.table.reference,
            providerCode: resource.table.providerCode,
            generation: resource.table.generation,
            conditions: resource.table.conditions,
        })
        .from(capture.table)
        .innerJoin(resource.table, eq(resource.table.id, capture.table.target))
        .where(eq(capture.table.deploymentId, current.id));
}

/** Record an instance's fields and ready condition, unless it has them already. */
async function observe(
    recorded: Instance,
    observed: {
        /** The instance status. */
        readonly status: Instance["status"];
        /** The ready condition's status, reason and message. */
        readonly ready: readonly [StatusCondition["status"], string, string];
        /** The restarts after failures. */
        readonly restarts?: number;
        /** The start time. */
        readonly startedAt?: number;
        /** The finish or failure time. */
        readonly stoppedAt?: number | null;
    },
    control: ObjectControl,
): Promise<void> {
    // skip an observation the instance has
    const {
        ready: [status, reason, message],
        ...fields
    } = observed;
    const previous = recorded.conditions.ready;
    const isServed =
        previous?.status === status &&
        previous.reason === reason &&
        previous.message === message &&
        Object.entries(fields).every(([name, value]) => recorded[name as keyof Instance] === value);
    if (isServed) {
        return;
    }

    // record the fields and the condition with its transition time kept while the status stays
    const now = control.now;
    const ready: StatusCondition = {
        status,
        reason,
        message,
        observedGeneration: DEPLOYMENT_GENERATION,
        lastTransitionAt: previous?.status === status ? previous.lastTransitionAt : now,
    };
    await control.server.executeAsSystem(
        instance,
        "update",
        [
            {
                ...SystemCall.of(recorded),
                input: {
                    ...fields,
                    conditions: { ...recorded.conditions, ready },
                    observedAt: now,
                },
            },
        ],
        now,
    );
}

/** Match the deployments the cell runs: those selecting its host, or any for a region. */
function runs(cell: SpaceCell) {
    const host = SpaceCell.host(cell);

    return host === null
        ? undefined
        : or(isNull(deployment.table.hostId), eq(deployment.table.hostId, host));
}
