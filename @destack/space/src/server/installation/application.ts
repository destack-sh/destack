import { and, eq, inArray, isNotNull, type DatabaseConnection } from "@destack/db";
import type { ObjectReconciliation } from "@destack/object";
import { SystemCall } from "@destack/object/server";
import type { PackageId } from "@destack/package";
import type { ResourceState } from "@destack/package/declare";
import type { BuildReader } from "@destack/package/manifest";
import { ServerRuntime } from "@destack/package/runtime";
import { Address, type Plan, ResourceDescription, type Step, Upgrade } from "@destack/resource";
import { type Identifier, schema, Version } from "@destack/schema";
import type { DeclarationDescription } from "@destack/package/inspect";
import {
    ServiceBindingSpec,
    type ServiceBindingState,
    ServiceKind,
} from "@destack/service/declare";
import { ScheduleDescription, ServiceDescription } from "@destack/service/inspect";
import { canonicalize } from "@destack/schema/json";
import { SpaceError } from "../../error/index.ts";
import {
    binding,
    capture,
    Deployment,
    deployment,
    type Installation,
    installation,
    type InstallationRevision,
    installationRevision,
    instance,
    networkPolicy,
    networkPolicyVersion,
    packagePolicy,
    packagePolicyVersion,
    type Schedule,
    schedule,
    space,
    SpaceCell,
} from "../../object/index.ts";
import type { InstallationBuild } from "../../declare/installation.ts";
import type { PolicySelection } from "../../policy/selection.ts";
import type { Binder, ResourceNeed } from "../binding.ts";
import { Approval } from "./approval.ts";
import type { OpenBuild } from "./release.ts";
import type { Invoke } from "./stack.ts";

/** What a cell deploys applications with: its binder, its builds and the server runtimes it runs. */
export interface ApplicationOptions {
    /** The kinds of objects bindings target and deployments capture. */
    readonly binder: Binder;
    /** Open a package's build through the cell's store of builds. */
    readonly openBuild: OpenBuild;
    /** The server runtimes the cell runs, in preference order. */
    readonly runtimes: readonly ServerRuntime[];
}

/** Deploy an enabled application, drain a suspended one, remove a drained deleted one, and relate it as reader of what its live deployments captured. */
export async function reconcileApplication(
    id: Identifier<"installation">,
    reconciliation: ObjectReconciliation,
    options: ApplicationOptions,
): Promise<undefined> {
    // read the application of a served space and the revision it follows
    const database = reconciliation.database;
    const [served] = await database
        .select({ installation: installation.table, revision: installationRevision.table })
        .from(installation.table)
        .innerJoin(space.table, eq(space.table.id, installation.table.scope))
        .leftJoin(
            installationRevision.table,
            eq(installationRevision.table.id, installation.table.revisionId),
        )
        .where(
            and(
                eq(installation.table.id, id),
                eq(installation.table.role, "application"),
                SpaceCell.served(),
            ),
        );
    if (served === undefined) {
        return undefined;
    }
    const { installation: target, revision } = served;
    const live = await database
        .select()
        .from(deployment.table)
        .where(and(eq(deployment.table.installationId, target.id), Deployment.live()));
    const now = reconciliation.now;

    // drain every deployment of a deleted or suspended application, or of one following no revision
    const isDeleted = target.deletionRequestedAt !== null;
    if (isDeleted || target.status !== "enabled" || revision === null) {
        await drain(
            live.filter((entry) => entry.status === "active"),
            now,
            reconciliation,
        );

        // remove a deleted application once none of its deployments runs
        if (isDeleted && live.length === 0) {
            await finalize(target, now, reconciliation, options);

            return undefined;
        }
    }
    // deploy the revision until each workload runs from the output the cell chooses
    else {
        await release(target, revision, live, now, reconciliation, options);
    }

    // relate the application as reader of exactly what its live deployments captured
    await database.transaction((transaction) =>
        options.binder.relate(transaction, target.scope, target.id, now),
    );

    return undefined;
}

/** Deploy each new workload output of a revision's build and drain the replaced deployments. */
async function release(
    target: Installation,
    revision: InstallationRevision,
    live: readonly Deployment[],
    now: number,
    reconciliation: ObjectReconciliation,
    options: ApplicationOptions,
): Promise<void> {
    // read the workloads the cell runs and refuse a build with an unrunnable workload
    const build = await options.openBuild(target.packageId, revision.build);
    const { workloads, stranded } = workloadsOf(build, options.runtimes);
    if (stranded.length > 0) {
        const message = `workloads ${stranded.join(", ")} have no output on ${options.runtimes.join(", ")}`;
        await refuse(target, "NoRuntime", message, now, reconciliation);

        return;
    }

    // wait for an approval of the upgrade from the applied release, refusing a broken upgrade chain
    if (target.appliedRevisionId !== null && target.appliedRevisionId !== revision.id) {
        const [applied] = await reconciliation.database
            .select({ build: installationRevision.table.build })
            .from(installationRevision.table)
            .where(eq(installationRevision.table.id, target.appliedRevisionId));
        const upgrade = await upgradeOf(options.openBuild, target, applied!.build, build);
        if (typeof upgrade === "string") {
            await refuse(target, "NoUpgrade", upgrade, now, reconciliation);

            return;
        } else if (await Approval.awaits(target, upgrade, reconciliation)) {
            return;
        }
    }

    // keep the active deployments that run a chosen output of the revision with the bindings as they are
    const active = live.filter((entry) => entry.status === "active");
    const isChosen = (entry: (typeof live)[number]) =>
        entry.revisionId === revision.id &&
        workloads.some(
            (workload) => workload.name === entry.workload && workload.output === entry.output,
        );
    const isRebound = await rebound(target, active, reconciliation.database);
    const kept = isRebound ? [] : active.filter(isChosen);
    const fresh = workloads.filter(
        (workload) => !kept.some((entry) => entry.workload === workload.name),
    );
    if (
        target.appliedRevisionId === revision.id &&
        fresh.length === 0 &&
        kept.length === active.length
    ) {
        return;
    }
    const needs = await needsOf(build, target.packageId, workloads);
    const served = await servedOf(build);
    const pins = await pinsOf(build, needs);

    // refuse service needs without a bound address
    const bound = await reconciliation.database
        .select({ packageId: binding.table.packageId, name: binding.table.name })
        .from(binding.table)
        .where(
            and(eq(binding.table.scope, target.scope), eq(binding.table.installationId, target.id)),
        );
    const unbound = needs.filter(
        (need) =>
            need.kind === "service" &&
            !bound.some((entry) => entry.packageId === need.package.id && entry.name === need.name),
    );
    if (unbound.length > 0) {
        const names = unbound.map((need) => `${need.package.id} ${need.name}`).join(", ");
        await refuse(
            target,
            "MissingBinding",
            `bind services ${names} to their addresses`,
            now,
            reconciliation,
        );

        return;
    }

    // attach, drain the replaced deployments, then prepare, capture and activate the fresh ones in one transaction
    await reconciliation.database.transaction(async (transaction) => {
        // attach the resources the installation owns
        const call = system(transaction, target, now, reconciliation);
        const { invoke } = call;
        await options.binder.attach(call, target, needs);

        // drain the replaced deployments down to one active deployment per workload
        for (const replaced of active.filter((entry) => !kept.includes(entry))) {
            await invoke(deployment, "update", { id: replaced.id, status: "draining" });
        }

        // activate each fresh workload's deployment with its bindings captured
        for (const workload of fresh) {
            const activated = (await invoke(deployment, "create", {
                installationId: target.id,
                packageId: target.packageId,
                revisionId: revision.id,
                output: workload.output,
                workload: workload.name,
                runtime: workload.runtime,
                ...served,
                description: workload.description,
                policies: await policiesOf(transaction, target, workload.name),
                activatedAt: now,
            })) as typeof deployment.table.$inferSelect;
            await options.binder.capture(call, activated, pins);
        }

        // keep the schedules the build declares
        await declareSchedules(transaction, target, build, invoke);

        // observe the revision applied
        await invoke(installation, "observe", {
            id: target.id,
            observedGeneration: target.generation,
            conditions: {
                ready: {
                    status: "true",
                    reason: "Deployed",
                    message: `${workloads.length} workloads deployed`,
                },
            },
            fields: { appliedRevisionId: revision.id, plan: null, approvedPlan: null },
        });
    });
}

/** Report whether an installation's bindings changed since its active deployments captured them: a target, a pin, or a declaration added or removed. */
async function rebound(
    target: Installation,
    active: readonly Deployment[],
    database: DatabaseConnection,
): Promise<boolean> {
    // read the bindings and the active deployments' captures
    if (active.length === 0) {
        return false;
    }
    const bindings = await database
        .select()
        .from(binding.table)
        .where(
            and(eq(binding.table.scope, target.scope), eq(binding.table.installationId, target.id)),
        );
    const captures = await database
        .select()
        .from(capture.table)
        .where(
            and(
                eq(capture.table.scope, target.scope),
                inArray(
                    capture.table.deploymentId,
                    active.map((entry) => entry.id),
                ),
            ),
        );

    // compare each deployment's captures with the bindings, by declaration
    const key = (entry: { readonly packageId: string; readonly name: string }) =>
        `${entry.packageId} ${entry.name}`;
    const bound = new Map(bindings.map((entry) => [key(entry), entry]));

    return active.some((deployed) => {
        const captured = captures.filter((entry) => entry.deploymentId === deployed.id);
        const isSame =
            captured.length === bound.size &&
            captured.every((entry) => {
                const current = bound.get(key(entry));

                return (
                    current !== undefined &&
                    current.target === entry.target &&
                    (current.version === null || current.version === entry.version)
                );
            });

        return !isSame;
    });
}

/** Keep exactly the schedules an installation's build declares, leaving the ones the installation created. */
async function declareSchedules(
    transaction: DatabaseConnection,
    target: Installation,
    build: BuildReader,
    invoke: Invoke,
): Promise<void> {
    // read the package's own schedules
    const declared = (await ownDeclarations(build, "schedule", ScheduleDescription)).map(
        (declaration) => declaration.description,
    );
    const kept = await transaction
        .select()
        .from(schedule.table)
        .where(
            and(eq(schedule.table.installation, target.id), isNotNull(schedule.table.packageId)),
        );

    // declare the new schedules and redeclare the changed ones
    for (const { name, call, concurrency, deadline, ...timing } of declared) {
        const fields = { timing, call, concurrency, deadline };
        const existing = kept.find((entry) => entry.name === name);
        if (existing === undefined) {
            await invoke(schedule, "declare", {
                installation: target.id,
                packageId: target.packageId,
                name,
                ...fields,
            });
        } else if (canonicalize(fields) !== canonicalize(pick(existing))) {
            await invoke(schedule, "redeclare", { id: existing.id, ...fields });
        }
    }

    // retire the schedules the build no longer declares
    for (const retired of kept.filter(
        (entry) => !declared.some((next) => next.name === entry.name),
    )) {
        await invoke(schedule, "retire", { id: retired.id });
    }
}

/** The fields of a declared schedule its build may change. */
function pick(entry: Schedule) {
    return {
        timing: entry.timing,
        call: entry.call,
        concurrency: entry.concurrency,
        deadline: entry.deadline,
    };
}

/** Remove a drained application: release the resources it owns, delete its deployments with their instances and captures, then finalize it. */
async function finalize(
    target: Installation,
    now: number,
    reconciliation: ObjectReconciliation,
    options: ApplicationOptions,
): Promise<void> {
    await reconciliation.database.transaction(async (transaction) => {
        // release the resources it owns to their retention
        const call = system(transaction, target, now, reconciliation);
        const { invoke } = call;
        await options.binder.attach(call, target, []);

        // delete its retired deployments with their instances and captures
        const retired = await transaction
            .select({ id: deployment.table.id })
            .from(deployment.table)
            .where(eq(deployment.table.installationId, target.id));
        const ids = retired.map((entry) => entry.id);
        const instances = await transaction
            .select({ id: instance.table.id })
            .from(instance.table)
            .where(inArray(instance.table.deploymentId, ids));
        const captures = await transaction
            .select({ id: capture.table.id })
            .from(capture.table)
            .where(inArray(capture.table.deploymentId, ids));
        for (const [object, rows] of [
            [instance, instances],
            [capture, captures],
            [deployment, retired],
        ] as const) {
            for (const row of rows) {
                await invoke(object, "delete", { id: row.id });
            }
        }

        // finalize the installation with its revisions and bindings
        await invoke(installation, "finalize", { id: target.id });
    });
}

/** Run system calls in a transaction, in an installation's space. */
function system(
    transaction: DatabaseConnection,
    target: Installation,
    now: number,
    reconciliation: ObjectReconciliation,
): { readonly database: DatabaseConnection; readonly invoke: Invoke } {
    return {
        database: transaction,
        invoke: (object, name, input) =>
            reconciliation.server.invoke(transaction, target.scope, object, name, input, now),
    };
}

/** Plan the upgrade steps from an applied build to the next, or describe the gap in its upgrade chain. */
async function upgradeOf(
    open: OpenBuild,
    target: Installation,
    applied: InstallationBuild,
    next: BuildReader,
): Promise<Plan | string> {
    // skip rollbacks
    const from = (await open(target.packageId, applied)).manifest.package.version;
    if (Version.compare(next.manifest.package.version, from) <= 0) {
        return { steps: [] };
    }

    // walk each release's upgrade back to the applied release
    const steps: Step[] = [];
    let reader = next;
    while (reader.manifest.package.version !== from) {
        const release = reader.manifest.package.version;
        const reference = reader.manifest.upgrade;
        const upgrade = reference && (await reader.read(reference.file, Upgrade));
        if (upgrade === undefined || Version.compare(upgrade.from, from) < 0) {
            return `${target.alias} has no upgrade from ${from} to ${release}`;
        } else if (Version.compare(upgrade.from, release) >= 0) {
            return `${target.alias} ${release} upgrades from ${upgrade.from}, which is not earlier`;
        }
        const prefix = Address.join("installation", target.alias, "release", release);
        steps.unshift(
            ...upgrade.steps.map((step) => ({
                ...step,
                target: Address.join(prefix, step.target),
            })),
        );
        reader = await open(target.packageId, { kind: "release", version: upgrade.from });
    }

    return { steps };
}

/** Observe an installation not ready for a reason it does not report already. */
async function refuse(
    target: Installation,
    reason: "NoRuntime" | "MissingBinding" | "NoUpgrade",
    message: string,
    now: number,
    reconciliation: ObjectReconciliation,
): Promise<void> {
    const ready = target.conditions.ready;
    const isObserved =
        ready?.observedGeneration === target.generation &&
        ready.reason === reason &&
        ready.message === message;
    if (!isObserved) {
        await reconciliation.server.executeAsSystem(
            installation,
            "observe",
            [
                {
                    ...SystemCall.of(target),
                    input: {
                        observedGeneration: target.generation,
                        conditions: {
                            ready: { status: "false", reason, message },
                        },
                    },
                },
            ],
            now,
        );
    }
}

/** Drain active deployments. */
async function drain(
    active: readonly Deployment[],
    now: number,
    reconciliation: ObjectReconciliation,
): Promise<void> {
    if (active.length > 0) {
        await reconciliation.server.executeAsSystem(
            deployment,
            "update",
            active.map((entry) => SystemCall.of(entry, { status: "draining" })),
            now,
        );
    }
}

/** One workload of a build's server output. */
interface BuildWorkload {
    /** The output with it. */
    readonly output: string;
    /** The package-local workload name. */
    readonly name: string;
    /** The output's server runtime. */
    readonly runtime: ServerRuntime;
    /** The workload's description. */
    readonly description: (typeof deployment.table.$inferSelect)["description"];
}

/** Select each workload of a build from the server output of the first runtime the cell runs. */
function workloadsOf(
    build: BuildReader,
    runtimes: readonly ServerRuntime[],
): { readonly workloads: BuildWorkload[]; readonly stranded: string[] } {
    // list the workloads of the emitted server outputs
    const offered = Object.entries(build.manifest.outputs).flatMap(([output, described]) =>
        described.runtime !== "browser" && described.emit
            ? Object.entries(described.workloads).map(([name, description]) => ({
                  output,
                  name,
                  runtime: ServerRuntime.parse(described.runtime),
                  description,
              }))
            : [],
    );

    // pick each workload's output by the cell's runtime preference
    const workloads: BuildWorkload[] = [];
    const stranded: string[] = [];
    for (const [name, outputs] of Map.groupBy(offered, (entry) => entry.name)) {
        const rank = (entry: BuildWorkload) => runtimes.indexOf(entry.runtime);
        const [picked] = outputs
            .filter((entry) => rank(entry) >= 0)
            .sort((left, right) => rank(left) - rank(right));
        if (picked === undefined) {
            stranded.push(name);
        } else {
            workloads.push(picked);
        }
    }

    return { workloads, stranded };
}

/** Read the release a build runs and the oldest caller release its package's service serves. */
async function servedOf(build: BuildReader): Promise<{ release: Version; since?: Version }> {
    // read the package's own service
    const [service] = await ownDeclarations(build, "service", ServiceDescription);

    // take the build's release and the service's oldest served release
    const release = Version.parse(build.manifest.package.version);
    const since = service?.description.since;

    return since === undefined ? { release } : { release, since };
}

/** Read the callee release each service need pins: the build's own release for its own service, else its exact dependency's. */
async function pinsOf(
    build: BuildReader,
    needs: readonly ResourceNeed[],
): Promise<ReadonlyMap<string, ServiceBindingState>> {
    // read the build's exact dependencies once a service of another package needs them
    let dependencies: Awaited<ReturnType<BuildReader["dependencies"]>>[string][] | undefined;
    const releaseOf = async (callee: PackageId) => {
        if (callee === build.manifest.package.id) {
            return build.manifest.package.version;
        }
        dependencies ??= Object.values(await build.dependencies());
        const found = dependencies.find(
            (entry) => "id" in entry.package && entry.package.id === callee,
        );

        return found?.package.version;
    };

    // pin each service to the release of its package the build compiled against
    const pins = new Map<string, ServiceBindingState>();
    for (const need of needs.filter((entry) => entry.kind === ServiceKind.name)) {
        const callee = ServiceBindingSpec.parse(need.spec).service.packageId;
        const release = await releaseOf(callee);
        if (release === undefined) {
            throw new SpaceError(
                "INVALID_DEFINITION",
                `the build binds service ${need.name} of ${callee} without depending on it`,
            );
        }
        pins.set(`${need.package.id} ${need.name}`, { release: Version.parse(release) });
    }

    return pins;
}

/** Read the resource declarations of an installation's package that its workloads reach, with the state each requires. */
async function needsOf(
    build: BuildReader,
    packageId: PackageId,
    workloads: readonly BuildWorkload[],
): Promise<ResourceNeed[]> {
    // list the package's own resources the workloads reach
    const reached = new Set(
        workloads.flatMap((workload) =>
            workload.description.resources
                .filter((reference) => reference.packageId === packageId)
                .map((reference) => reference.name),
        ),
    );
    if (reached.size === 0) {
        return [];
    }

    // split each reached resource declaration of the package into its description and the state it requires
    const declared = await ownDeclarations(build, "resource", ResourceDescription.passthrough());
    const needs = declared
        .filter((entry) => reached.has(entry.name))
        .map((entry) => {
            const { name, kind, spec, ...state } = entry.description;

            return {
                package: entry.symbol.package,
                name,
                kind,
                spec,
                state: state as ResourceState,
            };
        });
    // require a description of every reached declaration
    const missing = [...reached].filter((name) => !needs.some((need) => need.name === name));
    if (missing.length > 0) {
        throw new SpaceError(
            "INVALID_DEFINITION",
            `the build of ${packageId} describes no resources ${missing.join(", ")}`,
        );
    }

    return needs;
}

/** Select the package and network policy versions in force for an installation's workload. */
async function policiesOf(
    database: DatabaseConnection,
    target: Installation,
    workload: string,
): Promise<PolicySelection> {
    // TODO #Incomplete: select the account's policies, kept by the region administering them
    const owner = { kind: "space" as const, spaceId: target.scope };
    const packages = await database
        .select({
            versionId: packagePolicyVersion.table.id,
            digest: packagePolicyVersion.table.digest,
            definition: packagePolicyVersion.table.definition,
        })
        .from(packagePolicy.table)
        .innerJoin(
            packagePolicyVersion.table,
            eq(packagePolicyVersion.table.id, packagePolicy.table.currentVersionId),
        )
        .where(eq(packagePolicy.table.scope, target.scope));
    const network = await database
        .select({
            versionId: networkPolicyVersion.table.id,
            digest: networkPolicyVersion.table.digest,
            definition: networkPolicyVersion.table.definition,
            level: networkPolicy.table.level,
            installationId: networkPolicy.table.installationId,
            workload: networkPolicy.table.workload,
        })
        .from(networkPolicy.table)
        .innerJoin(
            networkPolicyVersion.table,
            eq(networkPolicyVersion.table.id, networkPolicy.table.currentVersionId),
        )
        .where(
            and(
                eq(networkPolicy.table.scope, target.scope),
                inArray(networkPolicy.table.level, ["space", "installation", "workload"]),
            ),
        );

    return {
        packages: packages.map((entry) => ({ owner, ...entry })),
        network: network
            .filter(
                (entry) =>
                    entry.level === "space" ||
                    (entry.installationId === target.id &&
                        (entry.level === "installation" || entry.workload === workload)),
            )
            .map(({ versionId, digest, definition }) => ({ owner, versionId, digest, definition })),
    };
}

/** Read a build's own declarations of a kind, parsing each description. */
async function ownDeclarations<Item extends schema.Schema>(
    build: BuildReader,
    kind: string,
    item: Item,
): Promise<
    (Omit<DeclarationDescription, "description"> & { readonly description: schema.Output<Item> })[]
> {
    const declarations = await build.declarations();

    return declarations
        .filter((declaration) => declaration.kind === kind)
        .map((declaration) => ({
            ...declaration,
            description: item.parse(declaration.description),
        }));
}
