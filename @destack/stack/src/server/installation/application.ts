import { Space, space } from "@destack/account/object";
import { and, DatabaseKind, DatabaseState, eq } from "@destack/db";
import { ObjectStore } from "@destack/object/server";
import accountPackage from "@destack/account/package";
import { type CallOf, type ObjectReconciliation, type ResultOf } from "@destack/object";
import { declareInstalled } from "@destack/object/inspect";
import {
    Capabilities,
    DeclarationName,
    type DirectoryGrant,
    type graph,
    type PackageId,
} from "@destack/package";
import type { BuildReader, Declared } from "@destack/package/manifest";
import { ServerRuntime } from "@destack/package/runtime";
import {
    Address,
    type Plan,
    RESOURCE_DECLARATION,
    ResourceDescription,
    type Step,
    Upgrade,
} from "@destack/resource";
import { canonicalize, present, schema, Version } from "@destack/schema";
import { readReleases } from "./stack.ts";
import {
    InstallationBindingSpec,
    type InstallationBindingState,
    InstallationKind,
} from "@destack/runtime/declare";
import { PackageDescription } from "@destack/runtime";
import { TriggerDescription } from "@destack/runtime/trigger";
import { StackError } from "../../error/index.ts";
import {
    binding,
    Binding,
    capture,
    Deployment,
    deployment,
    Installation,
    installation,
    type InstallationRevision,
    installationRevision,
    instance,
    MAIN_DATABASE,
    trigger,
} from "../../object/index.ts";
import type { InstallationBuild } from "../../declare/installation.ts";
import type { Binder, ResourceNeed } from "../resource/binding.ts";
import { BuildStore, type Registry } from "./build.ts";
import { Approval } from "./approval.ts";
import { InstallationGrant } from "./grant.ts";
import { BindingGrantRole } from "../resource/grant.ts";

/** The description of a declaration of a kind other than resource: its name and specification. */
const OtherDescription = schema.object({ name: DeclarationName }).catchall(schema.json());

/** What a machine deploys applications with: its binder, its builds and its server runtimes. */
export interface ApplicationOptions {
    /** The kinds of objects bindings target and deployments capture. */
    readonly binder: Binder;
    /** The machine's store of every build. */
    readonly builds: BuildStore;
    /** The registry of published releases. */
    readonly registry: Registry;
    /** The server runtimes the machine runs, in preference order. */
    readonly runtimes: readonly ServerRuntime[];
}

/** What a revision rolls out. */
interface RolloutPlan {
    /** The revision's build. */
    readonly build: BuildReader;
    /** The workloads the machine runs. */
    readonly workloads: readonly BuildWorkload[];
    /** The workloads deployed anew. */
    readonly fresh: readonly BuildWorkload[];
    /** The active deployments the fresh ones replace. */
    readonly replaced: readonly Deployment[];
    /** The declarations the installation's workloads bind. */
    readonly needs: readonly ResourceNeed[];
}

/** The permissions a fresh deploy's workloads request in the submitter's home and account. */
type Requested = NonNullable<ResultOf<typeof installation, "deploy">["requested"]>;

/** The releases a rollout's deployments run and call. */
interface Releases {
    /** The release the build serves and the oldest caller release its package serves. */
    readonly served: Awaited<ReturnType<typeof servedRelease>>;
    /** The callee release each installation need pins. */
    readonly pins: ReadonlyMap<string, InstallationBindingState>;
}

/** One workload of a build's server output. */
interface BuildWorkload {
    /** The output with it. */
    readonly output: string;
    /** The package-local workload name. */
    readonly name: string;
    /** The output's server runtime. */
    readonly runtime: ServerRuntime;
    /** The workload's description, with the capabilities it runs with. */
    readonly description: (typeof deployment.table.$inferSelect)["description"];
    /** The directories its `fs` capability runs with, none without the capability. */
    readonly directories: readonly DirectoryGrant[];
}

/** Why an application refuses to deploy a build, as its ready condition reports it. */
interface Refusal {
    /** The ready condition's reason. */
    readonly reason:
        | "NoRuntime"
        | "UndeclaredCapability"
        | "DeclaredDatabase"
        | "MissingBinding"
        | "NoUpgrade";
    /** What the person fixes. */
    readonly message: string;
}

/** An application installation one deploy call deploys, drains and relates. */
export class Application {
    /** The deploy call. */
    readonly #call: CallOf<typeof installation, "deploy">;
    /** What the machine deploys with. */
    readonly #options: ApplicationOptions;

    /** Create an application of the installation a deploy call deploys. */
    constructor(call: CallOf<typeof installation, "deploy">, options: ApplicationOptions) {
        this.#call = call;
        this.#options = options;
    }

    /** Deploy the revision of an enabled application, and drain a suspended one. */
    async deploy(): Promise<Requested | null> {
        // read the revision it follows and its live deployments
        const call = this.#call;
        const target = this.#call.target;
        const revision =
            target.revisionId === null
                ? undefined
                : await call
                      .read(installationRevision)
                      .findFirst({ where: { id: target.revisionId } });
        const live = await call.read(deployment).findMany({
            where: {
                ...Deployment.live(),
                installationId: target.id,
            },
        });

        // drain every deployment of a deleted or suspended application or of one following no revision
        let requested: Requested | null = null;
        const isDeleted = target.deletedAt !== null;
        if (isDeleted || target.status !== "enabled" || revision === undefined) {
            const active = live.filter((entry) => entry.status === "active");
            await call.mutate(
                call.self,
                active.map((entry) => Deployment.drain(entry)),
            );

            // remove a deleted application once none of its deployments runs and nothing it consumed relates it
            if (isDeleted && live.length === 0) {
                await this.#relate();
                await call.mutate(call.self, [installation.call("finalize", { id: target.id })]);

                return null;
            }
        }
        // deploy the revision until each workload runs from the output the machine chooses
        else {
            requested = await this.#deploy(revision, live);
        }

        // relate the application as consumer of exactly what its live deployments captured
        await this.#relate();

        return requested;
    }

    /** Relate the application as consumer of exactly what its live deployments captured. */
    async #relate(): Promise<void> {
        // relate it as consumer of what its live deployments captured
        const call = this.#call;
        const target = this.#call.target;
        const digest = await this.#options.binder.relate(
            call,
            target.scope,
            target.id,
            target.consumption,
        );

        // record what it related once that changed
        if (digest !== target.consumption) {
            await call.observe(installation, target, {
                conditions: {},
                fields: { consumption: digest },
            });
        }
    }

    /** Deploy each new workload output of a revision's build and drain the replaced deployments. */
    async #deploy(
        revision: InstallationRevision,
        live: readonly Deployment[],
    ): Promise<Requested | null> {
        // read the workloads the machine runs
        const target = this.#call.target;
        const build = await this.#open(revision.build.manifest);
        const { workloads, stranded } = this.#selectWorkloads(build);

        // refuse an unrunnable workload, an undeclared capability or directory, or a database
        const refusal =
            this.#describeRefusal(build, stranded) ?? (await this.#describeDatabases(build));
        if (refusal !== undefined) {
            await this.#refuse(refusal);

            return null;
        }

        // hold for a broken upgrade chain or an approval of the upgrade from the applied release
        if (await this.#holdForUpgrade(revision, build)) {
            return null;
        }

        // keep the active deployments that run a chosen output of the revision with the bindings and grants as they are
        const { fresh, replaced } = await this.#partition(revision, workloads, live);
        if (
            target.appliedRevisionId === revision.id &&
            fresh.length === 0 &&
            replaced.length === 0
        ) {
            return null;
        }

        // refuse installation needs without a bound address
        const needs = await this.#readNeeds(build, workloads);
        const unbound = await this.#describeUnboundInstallations(needs);
        if (unbound !== undefined) {
            await this.#refuse(unbound);

            return null;
        }

        // attach, drain the replaced deployments and activate the fresh ones
        return this.#rollOut(revision, { build, workloads, fresh, replaced, needs });
    }

    /** Attach the application's needs, then drain the replaced deployments. */
    async #rollOut(
        revision: InstallationRevision,
        rollout: RolloutPlan,
    ): Promise<Requested | null> {
        // attach what the installation owns and wait for targets not ready to capture
        const call = this.#call;
        const target = this.#call.target;
        const { build, workloads, fresh, replaced, needs } = rollout;
        await this.#options.binder.attach(call, target, needs);
        if (!(await this.#options.binder.ready(call, target, needs))) {
            const message = "bound targets wait for their resources";
            await call.observe(installation, target, {
                conditions: {
                    ready: { status: "false", reason: "WaitingForResources", message },
                },
            });

            return null;
        }

        // grant the installation exactly the space permissions its workloads request
        const descriptions = workloads.map((workload) => workload.description);
        const requested = InstallationGrant.requested(descriptions, "space");
        await InstallationGrant.keep(call, target, requested);

        // drain the replaced deployments down to one active deployment per workload
        await call.mutate(
            call.self,
            replaced.map((entry) => Deployment.drain(entry)),
        );

        // activate each fresh workload's deployment with its bindings captured
        const releases = {
            served: await servedRelease(build),
            pins: await pinnedReleases(build, needs),
        };
        for (const workload of fresh) {
            await this.#activate(revision, workload, releases);
        }

        // keep the triggers the build declares and observe the revision applied
        await this.#declareTriggers(build);
        await call.observe(installation, target, {
            conditions: {
                ready: {
                    status: "true",
                    reason: "Deployed",
                    message: `${workloads.length} workloads deployed`,
                },
            },
            fields: {
                ...Installation.applied(target, revision.id),
                plan: null,
                approvedPlan: null,
            },
        });

        return {
            home: InstallationGrant.requested(descriptions, "home"),
            account: InstallationGrant.requested(descriptions, "account"),
        };
    }

    /** Describe why a build cannot deploy. */
    #describeRefusal(build: BuildReader, stranded: readonly string[]): Refusal | undefined {
        // refuse a build with an unrunnable workload
        if (stranded.length > 0) {
            const runtimes = this.#options.runtimes.join(", ");
            const message = `workloads ${stranded.join(", ")} have no output on ${runtimes}`;

            return { reason: "NoRuntime", message };
        }

        // refuse allowing a capability or granting a directory the build does not declare
        const undeclared = this.#describeUndeclaredGrants(build);

        return undeclared === undefined
            ? undefined
            : { reason: "UndeclaredCapability", message: undeclared };
    }

    /** Describe the databases a build declares, which the space's main database refuses. */
    async #describeDatabases(build: BuildReader): Promise<Refusal | undefined> {
        // read the database declarations
        const declarations = await build.declarations();
        const databases = declarations.filter(
            (declaration) =>
                declaration.kind === RESOURCE_DECLARATION &&
                ResourceDescription.loose().parse(declaration.description).kind ===
                    DatabaseKind.name,
        );
        if (databases.length === 0) {
            return undefined;
        }

        // refuse them, as the space's main database keeps every installation's tables
        const names = databases.map((declaration) => declaration.name).join(", ");
        const message = `${this.#call.target.packageId} declares the databases ${names}: the space's main database keeps every installation's tables`;

        return { reason: "DeclaredDatabase", message };
    }

    /** Observe the application refusing to deploy, with the reason and message why. */
    async #refuse(refusal: Refusal): Promise<void> {
        const ready = { status: "false" as const, ...refusal };
        await this.#call.observe(installation, this.#call.target, { conditions: { ready } });
    }

    /** Hold the deploy for a refused or unapproved upgrade, observing why, and report a hold. */
    async #holdForUpgrade(revision: InstallationRevision, build: BuildReader): Promise<boolean> {
        // pass a first deploy and a redeploy of the applied revision
        const call = this.#call;
        const target = this.#call.target;
        if (target.appliedRevisionId === null || target.appliedRevisionId === revision.id) {
            return false;
        }

        // read the applied build and the upgrade from it
        const applied = await call
            .read(installationRevision)
            .findFirst({ where: { id: target.appliedRevisionId } });
        const appliedBuild = present(applied, `revision ${target.appliedRevisionId}`).build;
        const upgrade = await this.#planUpgrade(appliedBuild, build);

        // refuse a broken upgrade chain
        if (upgrade.kind === "refused") {
            await this.#refuse(upgrade.refusal);

            return true;
        }

        // observe the upgrade waiting for approval
        const waiting = await Approval.waiting(call, target, upgrade.plan);
        if (waiting !== undefined) {
            await call.observe(installation, target, waiting);
        }

        return waiting !== undefined;
    }

    /** Split a revision's workloads into fresh ones and the active deployments they replace. */
    async #partition(
        revision: InstallationRevision,
        workloads: readonly BuildWorkload[],
        live: readonly Deployment[],
    ): Promise<{ readonly fresh: BuildWorkload[]; readonly replaced: Deployment[] }> {
        // keep the chosen active deployments
        const active = live.filter((entry) => entry.status === "active");
        const isRebound = await this.#isRebound(active);
        const chosen = isRebound
            ? []
            : active.filter((entry) => isChosen(entry, revision, workloads));

        // deploy the other workloads anew in place of the other active deployments
        const fresh = workloads.filter(
            (workload) => !chosen.some((entry) => entry.workload === workload.name),
        );
        const replaced = active.filter((entry) => !chosen.includes(entry));

        return { fresh, replaced };
    }

    /** Describe the installation needs of the application without an address. */
    async #describeUnboundInstallations(
        needs: readonly ResourceNeed[],
    ): Promise<Refusal | undefined> {
        // read the installation's bindings
        const bound = await this.#call.read(binding).findMany({
            where: {
                scope: this.#call.target.scope,
                installationId: this.#call.target.id,
            },
        });

        // list the unbound installation needs that have no address of their own
        const unbound = needs.filter(
            (need) =>
                need.kind === InstallationKind.name &&
                InstallationBindingSpec.parse(need.spec).address === undefined &&
                !bound.some(
                    (entry) => entry.packageId === need.package.id && entry.name === need.name,
                ),
        );
        if (unbound.length === 0) {
            return undefined;
        }
        const names = unbound.map((need) => `${need.package.id} ${need.name}`).join(", ");

        return {
            reason: "MissingBinding",
            message: `bind installations ${names} to their addresses`,
        };
    }

    /** Activate a fresh workload's deployment of a revision and capture its bindings. */
    async #activate(
        revision: InstallationRevision,
        workload: BuildWorkload,
        releases: Releases,
    ): Promise<void> {
        const call = this.#call;
        const [activated] = await call.mutate(call.self, [
            deployment.call("create", {
                installationId: this.#call.target.id,
                packageId: this.#call.target.packageId,
                revisionId: revision.id,
                output: workload.output,
                workload: workload.name,
                runtime: workload.runtime,
                ...releases.served,
                description: workload.description,
                directories: [...workload.directories],
                activatedAt: call.now,
            }),
        ]);
        await this.#options.binder.capture(call, activated, releases.pins);
    }

    /** Report whether the bindings changed since the active deployments captured them. */
    async #isRebound(active: readonly Deployment[]): Promise<boolean> {
        // report no change without active deployments
        if (active.length === 0) {
            return false;
        }

        // read the bindings and the active deployments' captures
        const call = this.#call;
        const target = this.#call.target;
        const bindings = await call.read(binding).findMany({
            where: {
                scope: target.scope,
                installationId: target.id,
            },
        });
        const captures = await call.read(capture).findMany({
            where: {
                scope: target.scope,
                deploymentId: { in: active.map((entry) => entry.id) },
            },
        });

        // compare each deployment's captures with the bindings by declaration
        const bound = new Map(
            bindings.map((entry) => [Binding.key(entry.packageId, entry.name), entry]),
        );

        return active.some((deployed) => {
            const captured = captures.filter((entry) => entry.deploymentId === deployed.id);
            const isSame =
                captured.length === bound.size &&
                captured.every((entry) => {
                    const current = bound.get(Binding.key(entry.packageId, entry.name));

                    return (
                        current !== undefined &&
                        current.target === entry.target &&
                        current.targetSpaceId === entry.targetSpaceId &&
                        (current.version === null || current.version === entry.version)
                    );
                });

            return !isSame;
        });
    }

    /** Keep exactly the triggers the build declares, leaving those the installation created. */
    async #declareTriggers(build: BuildReader): Promise<void> {
        // read the package's own triggers
        const call = this.#call;
        const target = this.#call.target;
        const declared = (await ownDeclarations(build, "trigger", TriggerDescription)).map(
            (declaration) => declaration.description,
        );
        const triggers = await call.read(trigger).findMany({
            where: {
                installationId: target.id,
                origin: "declared",
            },
        });

        // declare the new triggers and redeclare the changed ones
        const declarations = declared.flatMap(({ name, on }) => {
            const fields = { on };
            const existing = triggers.find((entry) => entry.name === name);
            // declare a new trigger
            if (existing === undefined) {
                return [
                    trigger.call("declare", {
                        installationId: target.id,
                        origin: "declared",
                        name,
                        ...fields,
                    }),
                ];
            }
            // redeclare a changed trigger
            else if (canonicalize(fields) !== canonicalize({ on: existing.on })) {
                return [trigger.call("redeclare", { id: existing.id, ...fields })];
            }
            // keep an unchanged trigger
            else {
                return [];
            }
        });

        // retire the triggers the build no longer declares
        const retirements = triggers
            .filter((entry) => !declared.some((next) => next.name === entry.name))
            .map((retired) => trigger.call("retire", { id: retired.id }));
        await call.mutate(call.self, [...declarations, ...retirements]);
    }

    /** Open a build the build store keeps by its manifest's digest. */
    async #open(manifest: string): Promise<BuildReader> {
        return this.#options.builds.packages.contents(manifest);
    }

    /** Plan the upgrade steps from an applied build to the next, or refuse a gap in its chain. */
    async #planUpgrade(
        applied: InstallationBuild,
        next: BuildReader,
    ): Promise<
        | { readonly kind: "planned"; readonly plan: Plan }
        | { readonly kind: "refused"; readonly refusal: Refusal }
    > {
        // skip rollbacks
        const target = this.#call.target;
        const from = (await this.#open(applied.manifest)).manifest.package.version;
        if (Version.compare(next.manifest.package.version, from) <= 0) {
            return { kind: "planned", plan: { steps: [] } };
        }

        // walk each release's upgrade back to the applied release
        const steps: Step[] = [];
        let reader = next;
        while (reader.manifest.package.version !== from) {
            const release = reader.manifest.package.version;
            const reference = reader.manifest.upgrade;
            const upgrade = reference && (await reader.read(reference.file, Upgrade));
            // refuse a gap in the chain
            if (upgrade === undefined || Version.compare(upgrade.from, from) < 0) {
                const message = `${target.name} has no upgrade from ${from} to ${release}`;

                return { kind: "refused", refusal: { reason: "NoUpgrade", message } };
            }
            // refuse an upgrade from a release that is not earlier
            else if (Version.compare(upgrade.from, release) >= 0) {
                const message = `${target.name} ${release} upgrades from ${upgrade.from}, which is not earlier`;

                return { kind: "refused", refusal: { reason: "NoUpgrade", message } };
            }
            const prefix = Address.join("installation", target.name, "release", release);
            steps.unshift(
                ...upgrade.steps.map((step) => ({
                    ...step,
                    target: Address.join(prefix, step.target),
                })),
            );
            const { builds, registry } = this.#options;
            const earlier = await registry.find(target.packageId, upgrade.from);
            reader = await BuildStore.open(builds, registry, target.packageId, earlier);
        }

        return { kind: "planned", plan: { steps } };
    }

    /** Select each workload of a build from the server output of the machine's first runtime. */
    #selectWorkloads(build: BuildReader): {
        readonly workloads: BuildWorkload[];
        readonly stranded: string[];
    } {
        // list the workloads of the emitted server outputs with their required and allowed capabilities
        const target = this.#call.target;
        const offered = Object.entries(build.manifest.outputs).flatMap(([output, described]) =>
            described.runtime !== "browser" && described.emit
                ? Object.entries(described.workloads).map(([name, description]) => {
                      const capabilities = Capabilities.grant(
                          description.capabilities,
                          target.capabilities,
                      );

                      return {
                          output,
                          name,
                          runtime: ServerRuntime.parse(described.runtime),
                          description: { ...description, capabilities },
                          directories: capabilities.fs === undefined ? [] : target.directories,
                      };
                  })
                : [],
        );

        // pick each workload's output by the machine's runtime preference
        const runtimes = this.#options.runtimes;
        const workloads: BuildWorkload[] = [];
        const stranded: string[] = [];
        for (const [name, outputs] of Map.groupBy(offered, (entry) => entry.name)) {
            const rank = (entry: BuildWorkload) => runtimes.indexOf(entry.runtime);
            const [picked] = outputs
                .filter((entry) => rank(entry) >= 0)
                .toSorted((left, right) => rank(left) - rank(right));
            // strand a workload without an output on the machine's runtimes
            if (picked === undefined) {
                stranded.push(name);
            }
            // pick the output of the most preferred runtime
            else {
                workloads.push(picked);
            }
        }

        return { workloads, stranded };
    }

    /** Describe what the application allows or grants beyond what its build declares. */
    #describeUndeclaredGrants(build: BuildReader): string | undefined {
        // collect the capabilities the build's workloads and views declare
        const target = this.#call.target;
        const outputs = Object.values(build.manifest.outputs);
        const workloads = outputs.flatMap((output) =>
            Object.values(output.workloads).map((description) => description.capabilities),
        );
        const views = outputs.flatMap((output) =>
            Object.values(output.views).map((description) => description.capabilities),
        );
        const declared = new Set<string>(
            [...workloads, ...views].flatMap((capabilities) =>
                Object.entries(capabilities).flatMap(([name, entry]) =>
                    entry === undefined ? [] : [name],
                ),
            ),
        );
        const release = `${target.name} ${build.manifest.package.version}`;

        // refuse allowing a capability no workload or view declares
        const undeclared = target.capabilities.filter((name) => !declared.has(name));
        if (undeclared.length > 0) {
            return `${release} declares no capabilities ${undeclared.join(", ")} to allow`;
        }

        // refuse granting directories without fs and writing directories fs declares for reading
        const access = workloads.flatMap((capabilities) =>
            capabilities.fs === undefined ? [] : [capabilities.fs.access],
        );
        const written = target.directories.filter((grant) => grant.access === "write");
        // refuse granting directories without fs
        if (target.directories.length > 0 && access.length === 0) {
            return `${release} declares no fs capability to grant directories ${joinPaths(target.directories)}`;
        }
        // refuse writing directories fs declares for reading
        else if (written.length > 0 && !access.includes("write")) {
            return `${release} declares fs for reading only, refusing to grant writing ${joinPaths(written)}`;
        }

        return undefined;
    }

    /** Read the needs the binder binds: its objects' tables and its workloads' declarations. */
    async #readNeeds(
        build: BuildReader,
        workloads: readonly BuildWorkload[],
    ): Promise<ResourceNeed[]> {
        // list the package's own resources and secrets the workloads use
        const packageId = this.#call.target.packageId;
        const declarations = await build.declarations();
        const used = new Set(
            workloads.flatMap((workload) =>
                [...workload.description.resources, ...workload.description.secrets]
                    .filter((reference) => reference.packageId === packageId)
                    .map((reference) => reference.name),
            ),
        );

        // describe each used declaration of a kind the binder binds
        const needs = declarations
            .filter(
                (declaration) =>
                    used.has(declaration.name) && this.#options.binder.binds(declaration.kind),
            )
            .map((declaration) => describeNeed(build, declaration));

        // require a description of every used declaration
        const missing = [...used].filter((name) => !needs.some((entry) => entry.name === name));
        if (missing.length > 0) {
            throw new StackError(
                "INVALID_DEFINITION",
                `the build of ${packageId} describes no declarations ${missing.join(", ")} its machine binds`,
            );
        }

        return [...(await this.#mainNeed(build, declarations)), ...needs];
    }

    /** Describe the tables of the package's objects, which the space's main database keeps. */
    async #mainNeed(
        build: BuildReader,
        declarations: readonly graph.Declaration[],
    ): Promise<ResourceNeed[]> {
        // read the object types of the build's dependency closure, which its own types refer to
        const packageId = build.manifest.package.id;
        const depended = await Promise.all(
            (await readReleases(build)).map(async (release) => {
                const reader = await BuildStore.open(
                    this.#options.builds,
                    this.#options.registry,
                    release.package.id,
                    release.manifest,
                );

                return (await reader.declarations()).map((declaration) => ({
                    kind: declaration.kind,
                    packageId: release.package.id,
                    description: declaration.description,
                }));
            }),
        );

        // declare the package's types against the store's, its closure's and account's, a source stack depends on
        const owned = declarations.map((declaration) => ({
            kind: declaration.kind,
            packageId,
            description: declaration.description,
        }));
        const declared = declareInstalled(
            [...depended.flat(), ...owned],
            ObjectStore.known(this.#call.objects, Object.values(accountPackage.objects)),
        );
        if (!declared.some((type) => type.package.id === packageId)) {
            return [];
        }

        // describe its durable objects' tables, placed beside the closure's and platform's they reference
        const served = this.#call.objects;
        const tables = new Set(
            [...declared, ...served]
                .filter((type) => type.storage === "durable")
                .flatMap((type) => type.tables),
        );
        const state = DatabaseState.of([...tables], { package: build.manifest.package.id });

        return [
            {
                package: build.manifest.package,
                name: MAIN_DATABASE,
                kind: DatabaseKind.name,
                spec: { replicated: [] },
                state,
            },
        ];
    }
}

/** Deploy an application of a served space through its deploy method. */
export async function reconcileApplication(
    target: Installation,
    reconciliation: ObjectReconciliation,
): Promise<undefined> {
    // deploy the application of a served space
    const [served] = await reconciliation.database
        .select({ id: space.table.id })
        .from(space.table)
        .where(and(eq(space.table.id, target.scope), Space.served()));
    if (served === undefined) {
        return undefined;
    }
    await reconciliation.mutate([installation.call("deploy", installation.target(target))]);

    return undefined;
}

/** Remove a drained application's deployments with their instances and captures. */
export async function finalizeApplication(
    call: CallOf<typeof installation, "finalize">,
    options: Pick<ApplicationOptions, "binder">,
): Promise<void> {
    // release the resources it owns to their retention
    const target = call.target;
    await options.binder.attach(call, target, []);

    // delete its retired deployments with their instances and captures
    const retired = await call.read(deployment).findMany({ where: { installationId: target.id } });
    const ids = retired.map((entry) => entry.id);
    const instances = await call.read(instance).findMany({ where: { deploymentId: { in: ids } } });
    const captures = await call.read(capture).findMany({ where: { deploymentId: { in: ids } } });
    await call.mutate(call.self, [
        ...instances.map((row) => instance.call("delete", { id: row.id })),
        ...captures.map((row) => capture.call("delete", { id: row.id })),
        ...retired.map((row) => deployment.call("delete", { id: row.id })),
    ]);

    // drop its grant and the roles others have on it
    await InstallationGrant.keep(call, target, []);
    await BindingGrantRole.drop(call, target);
}

/** Report whether an active deployment runs a chosen output of the revision. */
function isChosen(
    entry: Deployment,
    revision: InstallationRevision,
    workloads: readonly BuildWorkload[],
): boolean {
    return (
        entry.revisionId === revision.id &&
        workloads.some(
            (workload) =>
                workload.name === entry.workload &&
                workload.output === entry.output &&
                canonicalize(workload.description.capabilities) ===
                    canonicalize(entry.description.capabilities) &&
                canonicalize(workload.directories) === canonicalize(entry.directories),
        )
    );
}

/** List the paths of directory grants. */
function joinPaths(grants: readonly DirectoryGrant[]): string {
    return grants.map((grant) => grant.path).join(", ");
}

/** Read the release a build runs and the oldest caller release its package serves. */
async function servedRelease(build: BuildReader): Promise<{ release: Version; since?: Version }> {
    // read the package's handle
    const [handle] = await ownDeclarations(build, "package", PackageDescription);

    // take the build's release and the package's oldest served release
    const release = Version.parse(build.manifest.package.version);
    const since = handle?.description.since;

    return since === undefined ? { release } : { release, since };
}

/** Read the callee release each installation need pins. */
async function pinnedReleases(
    build: BuildReader,
    needs: readonly ResourceNeed[],
): Promise<ReadonlyMap<string, InstallationBindingState>> {
    // read the build's exact dependencies when an installation of another package is needed
    const own = build.manifest.package;
    const callees = needs
        .filter((entry) => entry.kind === InstallationKind.name)
        .map((need) => ({ need, callee: InstallationBindingSpec.parse(need.spec).package }));
    const dependencies = callees.some(({ callee }) => callee !== own.id)
        ? Object.values(await build.dependencies())
        : [];
    const compiled = (callee: PackageId) =>
        callee === own.id
            ? own.version
            : dependencies.find((entry) => "id" in entry.package && entry.package.id === callee)
                  ?.package.version;

    // pin each installation need to the release of its package the build compiled against
    const pins = new Map<string, InstallationBindingState>();
    for (const { need, callee } of callees) {
        const release = compiled(callee);
        if (release === undefined) {
            throw new StackError(
                "INVALID_DEFINITION",
                `the build binds installation ${need.name} of ${callee} without depending on it`,
            );
        }
        pins.set(Binding.key(need.package.id, need.name), { release: Version.parse(release) });
    }

    return pins;
}

/** Describe a declaration as a need. */
function describeNeed(
    build: BuildReader,
    declaration: Awaited<ReturnType<BuildReader["declarations"]>>[number],
): ResourceNeed {
    // split a resource's description into its kind, specification and required state
    const owner = build.manifest.package;
    if (declaration.kind === RESOURCE_DECLARATION) {
        const described = ResourceDescription.catchall(schema.json()).parse(
            declaration.description,
        );
        const { name, kind, spec, ...state } = described;

        return { package: owner, name, kind, spec, state };
    }

    // take another kind's description as its specification
    const described = OtherDescription.parse(declaration.description);
    const { name, ...spec } = described;

    return { package: owner, name, kind: declaration.kind, spec, state: {} };
}

/** Read a build's own declarations of a kind, parsing each description. */
async function ownDeclarations<Item extends schema.Schema>(
    build: BuildReader,
    kind: string,
    item: Item,
): Promise<Declared<schema.Output<Item>>[]> {
    const declarations = await build.declarations();

    return declarations
        .filter((declaration) => declaration.kind === kind)
        .map((declaration) => ({
            ...declaration,
            description: item.parse(declaration.description),
        }));
}
