import { account, Entitlement, Space, space } from "@destack/account/object";
import { MAIN_DATABASE } from "../../declare/index.ts";
import { AccessContext, Caller, principal } from "@destack/access";
import {
    and,
    type BlobStore,
    type DatabaseConnection,
    DatabaseKind,
    DatabaseState,
    eq,
    Snapshot,
} from "@destack/db";
import { ObjectStore } from "@destack/object/server";
import accountPackage from "@destack/account/package";
import {
    installation,
    InstallationBindingSpec,
    type InstallationBindingState,
    InstallationKind,
    type MethodContextOf,
    ObjectDescription,
    type ObjectReconciliation,
    ObjectTypePool,
    PackageDescription,
} from "@destack/object";
import {
    HOST_SANDBOX,
    Sandbox,
    DeclarationName,
    type DirectoryGrant,
    graph,
    PackageDefinition,
    type PackageId,
    type PackageRelease,
} from "@destack/package";
import { type BuildReader, PackageManifest } from "@destack/package/manifest";
import { ServerRuntime } from "@destack/package/runtime";
import {
    Address,
    Plan,
    RESOURCE_DECLARATION,
    ResourceDescription,
    type Step,
    Upgrade,
    Vocabulary,
} from "@destack/resource";
import {
    canonicalize,
    type Digest,
    type Identifier,
    present,
    schema,
    Version,
} from "@destack/schema";
import { ObjectTypeReference, RequestId, Scope } from "@destack/sync";
import { StackError } from "../../error/index.ts";
import type { Runtime } from "../../runtime/runtime.ts";
import {
    binding,
    Binding,
    Build,
    capture,
    Deployment,
    deployment,
    InstallationRevision,
    installationRevision,
    instance,
    Rollout,
    rollout,
    type DeploymentPreparation,
    type NamedInstallation,
    type ResourceNeed,
} from "../../object/index.ts";
import type { Binder } from "../resource/binding.ts";
import { BuildStore, type Registry } from "./build.ts";
import { Approval } from "./approval.ts";
import { HOOK_WORKLOAD } from "../../workload/hook.ts";
import { type DeploymentRuntime, PROCESS_RUNTIME } from "../../object/index.ts";

/** The condition of an installation whose bound targets are each ready. */
const BOUND = { status: "true", reason: "Bound", message: "every bound target is ready" } as const;

/** The description of a declaration of a kind other than resource: its name and specification. */
const OtherDescription = schema.object({ name: DeclarationName }).catchall(schema.json());

/** What a machine deploys applications with: its binder, its builds and its server runtimes. */
export interface ApplicationOptions {
    /** The kinds of objects bindings target and deployments capture. */
    readonly binder: Binder;
    /** The space's builds, read through to the machine's cache. */
    readonly builds: BuildStore;
    /** The registry of published releases. */
    readonly registry: Registry;
    /** The server runtimes the machine runs, in preference order. */
    readonly runtimes: readonly Runtime[];
    /** Whether the machine checks capabilities' features against their accounts' entitlements. */
    readonly isEnforcingEntitlements: boolean;
    /** The space's database, with the replicas of its account's entitlements. */
    readonly database: DatabaseConnection;
    /** The release the machine serves in process of each package it ships, by package. */
    readonly bundled: ReadonlyMap<PackageId, PackageRelease>;
    /** The space's blobs, storing the states a deploy binds and captures. */
    readonly blobs: BlobStore;
}

/** What a revision deploys: its build, workloads, replaced deployments and needs. */
interface DeploymentPlan {
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
    /** The callee release of each installation need, as a blob, by key. */
    readonly pins: Readonly<Record<string, Digest>>;
    /** The package its build declares: requests and capabilities, absent for workloads alone. */
    readonly description: PackageDescription | undefined;
}

/** The releases a deployment plan runs and calls. */
interface Releases {
    /** The release the build serves and the oldest caller release its package serves. */
    readonly served: Awaited<ReturnType<typeof servedRelease>>;
    /** The callee release of each installation need, as a blob. */
    readonly pins: ReadonlyMap<string, Digest>;
}

/** One workload of a build's server output. */
interface BuildWorkload {
    /** The output with it. */
    readonly output: string;
    /** The package-local workload name. */
    readonly name: string;
    /** The output's runtime, or the machine's process for a shipped server. */
    readonly runtime: DeploymentRuntime;
    /** The workload's description, with the sandbox it runs with. */
    readonly description: (typeof deployment.table.$inferSelect)["description"];
    /** The directories its `fs` capability runs with, none without the capability. */
    readonly directories: readonly DirectoryGrant[];
    /** The package's capabilities it runs with. */
    readonly enabled: readonly string[];
}

/** The workload of a shipped server, which the machine's process serves in its store. */
const PROCESS_WORKLOAD = "server";

/** The ready reasons of an application refusing to deploy a build, apart from those it waits on. */
export const REFUSAL_REASONS = [
    "NoRuntime",
    "NoHooks",
    "UndeclaredCapability",
    "UnentitledCapability",
    "UninstalledDependency",
    "DeclaredDatabase",
    "MissingBinding",
] as const;

/** Why an application refuses to deploy a build, as its ready condition reports it. */
interface Refusal {
    /** The ready condition's reason. */
    readonly reason: (typeof REFUSAL_REASONS)[number];
    /** What the person fixes. */
    readonly message: string;
}

/** An application installation one deploy call deploys, drains and relates. */
export class Application {
    /** The deploy call, or its preparation before the transaction. */
    readonly #context: MethodContextOf<typeof rollout, "deploy", DeploymentPreparation | undefined>;
    /** The rollout deploying, with its installation's fields. */
    readonly #target: Rollout & { readonly installation: NamedInstallation };
    /** What the machine deploys with. */
    readonly #options: ApplicationOptions;

    /** Create an application of the installation a deploy call deploys. */
    constructor(
        context: MethodContextOf<typeof rollout, "deploy", DeploymentPreparation | undefined>,
        target: Rollout & { readonly installation: NamedInstallation },
        options: ApplicationOptions,
    ) {
        this.#context = context;
        this.#target = target;
        this.#options = options;
    }

    /** Write the states a deploy binds and pins as blobs, before its transaction. */
    async prepare(): Promise<DeploymentPreparation> {
        // prepare nothing for a deleted or suspended application, or one following no revision
        const target = this.#target;
        const revision =
            target.revisionId === null
                ? undefined
                : await this.#context
                      .read(installationRevision)
                      .findFirst({ where: { id: target.revisionId } });
        if (
            revision === undefined ||
            target.deletedAt !== null ||
            target.installation.status !== "enabled"
        ) {
            return { revisionId: null, needs: [], pins: {} };
        }

        // prepare nothing for a build its deploy refuses
        const { blobs } = this.#options;
        const selected = await this.#select(revision);
        if ((await this.#describeRefusals(revision, selected)) !== undefined) {
            return { revisionId: revision.id, needs: [], pins: {} };
        }

        // write the needs' states and the pinned releases of the build's workloads
        const { build, workloads, description } = selected;
        const needs = await this.#readNeeds(revision, build, workloads, description);
        const pins = await pinnedReleases(build, needs);
        const pinned = await Promise.all(
            [...pins].map(async ([key, state]) => [key, await blobs.writeJson(state)] as const),
        );

        return { revisionId: revision.id, needs, pins: Object.fromEntries(pinned) };
    }

    /** Deploy the revision of an enabled application, and drain a suspended one. */
    async deploy(): Promise<void> {
        // read the revision it follows and its live deployments
        const call = this.#context;
        const target = this.#target;
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
        const isDeleted = target.deletedAt !== null;
        if (isDeleted || target.installation.status !== "enabled" || revision === undefined) {
            const active = live.filter((entry) => entry.status === "active");
            await call.mutate(
                call.self,
                active.map((entry) => Deployment.drain(entry)),
            );

            // remove a deleted application once none of its deployments runs and nothing it consumed relates it
            if (isDeleted && live.length === 0) {
                await this.#relate();
                await call.mutate(call.self, [rollout.call("finalize", { id: target.id })]);

                return;
            }
        }
        // deploy the revision until each workload runs from the output the machine chooses
        else {
            await this.#deploy(revision, live);
        }

        // relate the application as consumer of exactly what its live deployments captured
        await this.#relate();
    }

    /** Relate the application as consumer of exactly what its live deployments captured. */
    async #relate(): Promise<void> {
        // relate it as consumer of what its live deployments captured
        const call = this.#context;
        const target = this.#target;
        const digest = await this.#options.binder.relate(call, target, target.consumption);

        // record what it related once that changed
        if (digest !== target.consumption) {
            await call.observe(rollout, target, {
                conditions: {},
                fields: { consumption: digest },
            });
        }
    }

    /** Deploy each new workload output of a revision's build and drain the replaced deployments. */
    async #deploy(revision: InstallationRevision, live: readonly Deployment[]): Promise<void> {
        // deploy only the prepared revision
        const prepared = present(this.#context.prepared, "the deploy's preparation");
        if (prepared.revisionId !== revision.id) {
            return;
        }

        // read the workloads the machine runs
        const target = this.#target;
        const { build, built, description, workloads, stranded } = await this.#select(revision);

        // refuse an unrunnable workload, an undeclared or unentitled capability or directory, or a database
        const refusal = await this.#describeRefusals(revision, {
            build,
            built,
            description,
            stranded,
        });
        if (refusal !== undefined) {
            await this.#refuse(refusal);

            return;
        }

        // stop for a broken upgrade chain or an approval of the upgrade from the applied release
        if (await this.#holdForUpgrade(revision, build)) {
            return;
        }

        // split the workloads into fresh and replaced ones, and upsert only the access when nothing changes
        const { fresh, replaced } = await this.#partition(revision, workloads, live);
        if (
            target.appliedRevisionId === revision.id &&
            fresh.length === 0 &&
            replaced.length === 0
        ) {
            await Rollout.upsertAccess(this.#context, target, description);

            return;
        }

        // refuse installation needs without a bound address
        const { needs } = prepared;
        const unbound = await this.#describeUnboundInstallations(needs);
        if (unbound !== undefined) {
            await this.#refuse(unbound);

            return;
        }

        // attach, drain the replaced deployments and activate the fresh ones
        const { pins } = prepared;
        await this.#deployPlan(revision, {
            build,
            workloads,
            fresh,
            replaced,
            needs,
            pins,
            description,
        });
    }

    /** Attach the application's needs, wait for its space, grant, drain and activate. */
    async #deployPlan(revision: InstallationRevision, plan: DeploymentPlan): Promise<void> {
        // wait for a provider of each kind no provider here serves yet
        const call = this.#context;
        const target = this.#target;
        const { build, workloads, fresh, replaced, needs, pins, description } = plan;
        const unprovided = this.#options.binder.unprovided(needs);
        if (unprovided.length > 0) {
            const message = `waiting for a provider of ${unprovided.join(", ")} resources`;
            await call.observe(rollout, target, {
                conditions: {
                    Bound: { status: "false", reason: "WaitingForResources", message },
                    Ready: { status: "false", reason: "WaitingForResources", message },
                },
            });

            return;
        }

        // attach the resources it needs
        await this.#options.binder.attach(call, target, needs);
        if (!(await this.#options.binder.ready(call, target, needs))) {
            const message = "bound targets wait for their resources";
            await call.observe(rollout, target, {
                conditions: {
                    Bound: { status: "false", reason: "WaitingForResources", message },
                    Ready: { status: "false", reason: "WaitingForResources", message },
                },
            });

            return;
        }

        // wait for the space to serve the revision once its targets are bound, and the packages whose types its grants name
        const unserved = await this.#describeUnserved(revision, description);
        if (unserved !== undefined) {
            await call.observe(rollout, target, {
                conditions: {
                    Bound: BOUND,
                    Ready: { status: "false", reason: "WaitingForSpace", message: unserved },
                },
            });

            return;
        }

        // grant the installation exactly what its package and its enabled capabilities request
        await Rollout.upsertAccess(call, target, description);

        // drain the replaced deployments down to one active deployment per workload
        await call.mutate(
            call.self,
            replaced.map((entry) => Deployment.drain(entry)),
        );

        // activate each fresh workload's deployment with its bindings captured
        const releases = {
            served: await servedRelease(build),
            pins: new Map(Object.entries(pins)),
        };
        for (const workload of fresh) {
            await this.#activate(revision, workload, releases);
        }

        // observe the revision applied
        await call.observe(rollout, target, {
            conditions: {
                Bound: BOUND,
                Ready: {
                    status: "true",
                    reason: "Deployed",
                    message: `${workloads.length} workloads deployed`,
                },
            },
            fields: {
                ...Rollout.applied(target, revision.id),
                plan: null,
                approvedPlan: null,
            },
        });
    }

    /** Describe why a revision's build cannot deploy, none for a deployable one. */
    async #describeRefusals(
        revision: InstallationRevision,
        selected: {
            readonly build: BuildReader;
            readonly built: { readonly manifest: Digest };
            readonly description: PackageDescription | undefined;
            readonly stranded: readonly string[];
        },
    ): Promise<Refusal | undefined> {
        const { build, built, description, stranded } = selected;

        return (
            this.#describeRefusal(built.manifest, build, stranded, description) ??
            (await this.#describeUnentitled(description)) ??
            (await this.#describeUninstalled(revision, description)) ??
            (await this.#describeDatabases(build))
        );
    }

    /** Describe why a build cannot deploy. */
    #describeRefusal(
        manifest: Digest,
        build: BuildReader,
        stranded: readonly string[],
        description: PackageDescription | undefined,
    ): Refusal | undefined {
        // refuse a build with an unrunnable workload
        if (stranded.length > 0) {
            const runtimes = this.#options.runtimes.map((runtime) => runtime.name).join(", ");
            const message = `workloads ${stranded.join(", ")} have no output on ${runtimes}`;

            return { reason: "NoRuntime", message };
        }

        // refuse a package routing hooks that nothing serves: its build's hook workload, or the machine in process
        const isHooked = Object.values(description?.objects ?? {}).some(
            (routed) => Object.keys(routed.hooks).length > 0,
        );
        const isHosted =
            this.#options.bundled.get(PackageManifest.package(build.manifest).id)?.manifest ===
            manifest;
        const isServed =
            isHosted ||
            Object.values(build.manifest.outputs).some((output) =>
                Object.hasOwn(output.workloads, HOOK_WORKLOAD),
            );
        if (isHooked && !isServed) {
            const message = `${build.manifest.package.name} routes hooks, but nothing serves them: declare @destack/stack`;

            return { reason: "NoHooks", message };
        }

        // refuse allowing a capability or granting a directory the build does not declare
        const undeclared = this.#describeUndeclaredGrants(build, description);

        return undeclared === undefined
            ? undefined
            : { reason: "UndeclaredCapability", message: undeclared };
    }

    /** Describe an enabled capability needing a feature the space's account lacks. */
    async #describeUnentitled(
        description: PackageDescription | undefined,
    ): Promise<Refusal | undefined> {
        // check nothing on a machine enforcing no entitlements
        if (!this.#options.isEnforcingEntitlements) {
            return undefined;
        }

        // list the features the enabled capabilities need
        const call = this.#context;
        const enabled = Rollout.enabled(this.#target.installation, description);
        const needed = enabled.flatMap((name) =>
            (description?.capabilities[name]?.features ?? []).map((feature) => ({ name, feature })),
        );

        // refuse the first feature the account's entitlements leave out
        const placed = present(
            await call
                .read(space)
                .findFirst({ where: { id: space.identifier(call.target.scope) } }),
            `the space ${call.target.scope}`,
        );
        const accountId = account.identifier(placed.scope);
        for (const { name, feature } of needed) {
            const database = this.#options.database;
            const condition = await Entitlement.check(database, accountId, { feature });
            if (condition.status !== "true") {
                return {
                    reason: "UnentitledCapability",
                    message: `capability ${name} needs ${feature.name}: ${condition.message}`,
                };
            }
        }

        return undefined;
    }

    /** Describe a package the application needs that no scope on the space's chain installed. */
    async #describeUninstalled(
        revision: InstallationRevision,
        description: PackageDescription | undefined,
    ): Promise<Refusal | undefined> {
        // read the packages installed on the space's chain
        const target = this.#target;
        const installed = new Set(
            (await chainInstallations(this.#context, this.#target.scope)).map(
                (row) => row.packageId,
            ),
        );

        // refuse the first needed package missing, naming the capability needing it
        const missing = Rollout.needed(target.installation, revision, description).find(
            (entry) => !installed.has(entry.package.id),
        );
        if (missing === undefined) {
            return undefined;
        }
        const { name } = missing.package;

        return {
            reason: "UninstalledDependency",
            message:
                missing.capability === undefined
                    ? `${target.installation.name} needs ${name} installed`
                    : `capability ${missing.capability} needs ${name} installed`,
        };
    }

    /** Describe what the space does not serve yet: the revision, or a package it needs. */
    async #describeUnserved(
        revision: InstallationRevision,
        description: PackageDescription | undefined,
    ): Promise<string | undefined> {
        // wait for the revision itself
        const call = this.#context;
        const target = this.#target;
        if (target.servedRevisionId !== revision.id) {
            return `waiting for the space to serve revision ${revision.id}`;
        }

        // wait for each package the application needs, and for the types in its space grant
        return describeUngranted(call, target, revision, description);
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

        // refuse them, since the space's main database stores every installation's tables
        const names = databases.map((declaration) => declaration.name).join(", ");
        const message = `${this.#target.installation.packageId} declares the databases ${names}: the space's main database keeps every installation's tables`;

        return { reason: "DeclaredDatabase", message };
    }

    /** Observe the application refusing to deploy, with the reason and message why. */
    async #refuse(refusal: Refusal): Promise<void> {
        const ready = { status: "false" as const, ...refusal };
        await this.#context.observe(rollout, this.#context.target, {
            conditions: { Ready: ready },
        });
    }

    /** Stop the deploy for an unapproved upgrade, observing why, and report the stop. */
    async #holdForUpgrade(revision: InstallationRevision, build: BuildReader): Promise<boolean> {
        // pass a first deploy and a redeploy of the applied revision
        const call = this.#context;
        const target = this.#target;
        if (target.appliedRevisionId === null || target.appliedRevisionId === revision.id) {
            return false;
        }

        // read the applied build and the upgrade from it
        const applied = await call
            .read(installationRevision)
            .findFirst({ where: { id: target.appliedRevisionId } });
        const upgrade = await this.#planUpgrade(
            present(applied, `revision ${target.appliedRevisionId}`),
            build,
        );

        // observe the upgrade waiting for approval
        const waiting = await Approval.waiting(call, target, upgrade.plan, call.environment.crypto);
        if (waiting !== undefined) {
            await call.observe(rollout, target, waiting);
        }

        return waiting !== undefined;
    }

    /** Split a revision's workloads into fresh ones and the active deployments they replace. */
    async #partition(
        revision: InstallationRevision,
        workloads: readonly BuildWorkload[],
        live: readonly Deployment[],
    ): Promise<{ readonly fresh: BuildWorkload[]; readonly replaced: Deployment[] }> {
        // choose the active deployments to reuse
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
        const bound = await this.#context.read(binding).findMany({
            where: {
                scope: this.#target.scope,
                installationId: this.#target.id,
            },
        });

        // list the installation needs with no address of their own that no binding or stack selection binds
        const selected = this.#target.bindings;
        const unbound = needs.filter(
            (need) =>
                need.kind === InstallationKind.name &&
                InstallationBindingSpec.parse(need.spec).address === undefined &&
                selected[need.package.id]?.[need.name] === undefined &&
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
        const call = this.#context;
        const [activated] = await call.mutate(call.self, [
            deployment.call("create", {
                installationId: this.#target.id,
                packageId: this.#target.installation.packageId,
                revisionId: revision.id,
                output: workload.output,
                workload: workload.name,
                runtime: workload.runtime,
                ...releases.served,
                description: workload.description,
                directories: [...workload.directories],
                enabled: [...workload.enabled],
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
        const call = this.#context;
        const target = this.#target;
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

    /** Read a revision's build, its package and the workloads the machine runs of it. */
    async #select(revision: InstallationRevision) {
        // read the build and its package
        const target = this.#target;
        const build = await this.#open(revision);
        const description = await PackageDescription.of(build);
        const built = await Build.require(
            { read: (table) => this.#context.read(table) },
            revision.scope,
            revision.buildId,
        );

        // serve a shipped server's release in the machine's process, else select its build's workloads
        const isShipped =
            this.#options.bundled.get(target.installation.packageId)?.manifest === built.manifest;
        const { workloads, stranded } = isShipped
            ? { workloads: [this.#processWorkload(description)], stranded: [] }
            : this.#selectWorkloads(build, description);

        return { build, built, description, workloads, stranded };
    }

    /** Open the build a revision evaluated from the build store. */
    #open(revision: Pick<InstallationRevision, "id">): Promise<BuildReader> {
        return InstallationRevision.reader(this.#context, revision.id, this.#options.builds);
    }

    /** Plan the upgrade steps from an applied build to the next by its kinds' comparisons. */
    async #planUpgrade(
        applied: Pick<InstallationRevision, "id">,
        next: BuildReader,
    ): Promise<{ readonly kind: "planned"; readonly plan: Plan }> {
        // skip rollbacks
        const previous = await this.#open(applied);
        const from = previous.manifest.package.version;
        const release = next.manifest.package.version;
        if (Version.compare(release, from) <= 0) {
            return { kind: "planned", plan: { steps: [] } };
        }

        // compare each kept declaration its kind compares, then plan the rest from the applied release
        const declarations = await previous.declarations();
        const history = {
            release: from,
            declarations,
            vocabulary: Vocabulary.advance({}, declarations, from),
        };
        const compared = await this.#compare(declarations, from, next);
        const owner = PackageManifest.package(next.manifest);
        const upgrade = Upgrade.plan(history, owner, await next.declarations(), (declaration) => {
            const plan = compared.get(Address.join(declaration.kind, declaration.name));

            return plan === undefined ? undefined : () => plan;
        });

        // address each step below the installation's release
        const { name } = this.#target.installation;
        const prefix = Address.join("installation", name, "release", release);
        const steps: Step[] = upgrade.steps.map((step) => ({
            ...step,
            target: Address.join(prefix, step.target),
        }));

        return { kind: "planned", plan: { steps } };
    }

    /** Compare the declarations a next build keeps by their kinds' compare, by address. */
    async #compare(
        declarations: readonly graph.Declaration[],
        from: string,
        next: BuildReader,
    ): Promise<ReadonlyMap<string, Plan>> {
        // pair the applied declarations with the next build's by kind and name
        const kept = new Map(
            declarations.map((each) => [Address.join(each.kind, each.name), each] as const),
        );
        // find each kind's package among the Destack builds the next build's outputs import
        const releases = Object.values(next.manifest.outputs).flatMap((output) =>
            Object.values(output.dependencies).flatMap((dependency) =>
                dependency.kind === "destack" ? [dependency] : [],
            ),
        );
        const owner = PackageManifest.package(next.manifest);
        const [runtime] = this.#options.runtimes;
        const compared = new Map<string, Plan>();
        for (const after of await next.declarations()) {
            // find the kind's compare function in its package's build, the next build's own or a release's
            const address = Address.join(after.kind, after.name);
            const before = kept.get(address);
            if (before === undefined) {
                continue;
            }
            const kind =
                after.kindPackageId === owner.id
                    ? next
                    : await this.#pullRelease(releases, after.kindPackageId);
            const reference = await kindCompare(kind, after.kind);
            if (reference === undefined) {
                continue;
            }

            // call it on the machine's preferred server runtime, as it runs the kind's package code
            const machine = present(runtime, "a server runtime of the machine");
            const output = present(
                Object.values(kind.manifest.outputs).find((each) => each.runtime === machine.name),
                `the ${machine.name} output of ${kind.manifest.package.name}`,
            );
            const exported = present(
                output.exports[reference.subpath],
                `the export ${reference.subpath} of ${kind.manifest.package.name}`,
            );
            const entry = (declaration: graph.Declaration, version: string) => ({
                description: declaration.description,
                symbol: { package: { ...owner, version } },
            });
            const plan = await machine.call(kind, exported.path, reference.name, [
                entry(before, from),
                entry(after, owner.version),
            ]);
            compared.set(address, Plan.parse(plan));
        }

        return compared;
    }

    /** Bring a release a build depends on into the machine's cache by its package. */
    async #pullRelease(
        releases: readonly PackageRelease[],
        packageId: PackageId,
    ): Promise<BuildReader> {
        const release = present(
            releases.find((each) => "id" in each.package && each.package.id === packageId),
            `the release of ${packageId}`,
        );

        return await this.#options.builds.pull(this.#options.registry, packageId, release.manifest);
    }

    /** Describe the workload of a shipped server, which the machine serves in its process. */
    #processWorkload(description: PackageDescription | undefined): BuildWorkload {
        return {
            output: PROCESS_RUNTIME,
            name: PROCESS_WORKLOAD,
            runtime: PROCESS_RUNTIME,
            description: {
                entrypoint: ".",
                packages: [],
                triggers: [],
                resources: [],
                secrets: [],
                compute: {},
                sandbox: {},
            },
            directories: [],
            enabled: Rollout.enabled(this.#target.installation, description),
        };
    }

    /** Select each workload of a build from the server output of the machine's first runtime. */
    #selectWorkloads(
        build: BuildReader,
        description: PackageDescription | undefined,
    ): {
        readonly workloads: BuildWorkload[];
        readonly stranded: string[];
    } {
        // list the workloads of the emitted server outputs, each sandbox joined with what the enabled capabilities unlock
        const target = this.#target;
        const enabled = Rollout.enabled(target.installation, description);
        const unlocked = Rollout.sandbox(target.installation, description);
        const host = Sandbox.select(
            unlocked,
            HOST_SANDBOX.filter((name) => unlocked[name] !== undefined),
        );
        const offered = Object.entries(build.manifest.outputs).flatMap(([output, described]) =>
            described.runtime !== "browser" && described.emit
                ? Object.entries(described.workloads).map(([name, workload]) => {
                      const sandbox = Sandbox.merge(workload.sandbox, host);

                      return {
                          output,
                          name,
                          runtime: ServerRuntime.parse(described.runtime),
                          description: { ...workload, sandbox },
                          directories:
                              sandbox.files === undefined ? [] : target.installation.directories,
                          enabled,
                      };
                  })
                : [],
        );

        // pick each workload's output by the machine's runtime preference
        const runtimes: readonly string[] = this.#options.runtimes.map((runtime) => runtime.name);
        const workloads: BuildWorkload[] = [];
        const stranded: string[] = [];
        for (const [name, outputs] of Map.groupBy(offered, (entry) => entry.name)) {
            const rank = (entry: (typeof offered)[number]) => runtimes.indexOf(entry.runtime);
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
    #describeUndeclaredGrants(
        build: BuildReader,
        description: PackageDescription | undefined,
    ): string | undefined {
        // refuse allowing a capability the package does not declare
        const target = this.#target;
        const release = `${target.installation.name} ${build.manifest.package.version}`;
        const declared = Object.keys(description?.capabilities ?? {});
        const undeclared = Rollout.capabilities(target.installation).filter(
            (name) => !declared.includes(name),
        );
        if (undeclared.length > 0) {
            return `${release} declares no capabilities ${undeclared.join(", ")} to allow`;
        }

        // read the directory access of the workloads' sandboxes joined with what the enabled capabilities unlock
        const unlocked = Rollout.sandbox(target.installation, description);
        const access = Object.values(build.manifest.outputs).flatMap((output) =>
            Object.values(output.workloads).flatMap((workload) => {
                const { files } = Sandbox.merge(workload.sandbox, unlocked);

                return files === undefined ? [] : [files.access];
            }),
        );
        const written = target.installation.directories.filter((grant) => grant.access === "write");
        // refuse granting directories without fs
        if (target.installation.directories.length > 0 && access.length === 0) {
            return `${release} enables no fs ability to grant directories ${joinPaths(target.installation.directories)}`;
        }
        // refuse writing directories fs declares for reading
        else if (written.length > 0 && !access.includes("write")) {
            return `${release} enables fs for reading only, refusing to grant writing ${joinPaths(written)}`;
        }

        return undefined;
    }

    /** Read the needs the binder binds: its objects' tables and its workloads' declarations. */
    async #readNeeds(
        revision: InstallationRevision,
        build: BuildReader,
        workloads: readonly BuildWorkload[],
        description: PackageDescription | undefined,
    ): Promise<ResourceNeed[]> {
        // list the package's own resources and secrets the workloads use
        const packageId = this.#target.installation.packageId;
        const declarations = await build.declarations();
        const used = new Set(
            workloads.flatMap((workload) =>
                [...workload.description.resources, ...workload.description.secrets]
                    .filter((reference) => reference.packageId === packageId)
                    .map((reference) => reference.name),
            ),
        );

        // describe each of its declarations the binder binds, which its servers and workloads open
        const needs = await Promise.all(declarations
            .filter((declaration) => this.#options.binder.binds(declaration.kind))
            .map((declaration) => describeNeed(build, declaration, this.#options.blobs)),
        );

        // require a description of every used declaration
        const missing = [...used].filter((name) => !needs.some((entry) => entry.name === name));
        if (missing.length > 0) {
            throw StackError.of(
                "DEFINITION_INVALID",
                `the build of ${packageId} is missing declarations ${missing.join(", ")} that its machine binds`,
            );
        }

        return [...(await this.#mainDatabase(revision, build, description)), ...needs];
    }

    /** Pool the types the space serves and account's, which installed types refer to. */
    #pool(): ObjectTypePool {
        return ObjectTypePool.of(this.#pooled());
    }

    /** List the types the space serves and account's, which installed types refer to. */
    #pooled() {
        return ObjectStore.known(this.#context.objects, Object.values(accountPackage.objects));
    }

    /** Describe the main database an installation needs: its own objects' tables. */
    async #mainDatabase(
        revision: InstallationRevision,
        build: BuildReader,
        description: PackageDescription | undefined,
    ): Promise<ResourceNeed[]> {
        // declare no tables for a package declaring no object types
        const packageId = PackageManifest.package(build.manifest).id;
        const owned = ObjectDescription.installed(await build.declarations());
        if (owned.length === 0) {
            return [];
        }

        // read the types the builds of the packages it needs declare, pulling none of a package the pool knows
        const known = new Set(
            this.#context.objects.map((type) => ObjectTypeReference.key(type.typeReference)),
        );
        const pooled = new Set(this.#pooled().map((type) => type.package.id));
        const needed = new Set(
            Rollout.needed(this.#target.installation, revision, description).map(
                (entry) => entry.package.id,
            ),
        );
        const { builds, registry } = this.#options;
        const depended = await Promise.all(
            revision.dependencies
                .filter(
                    (dependency) =>
                        needed.has(dependency.package.id) && !pooled.has(dependency.package.id),
                )
                .map(async (dependency) => {
                    const { id } = dependency.package;
                    const reader = await builds.pull(registry, id, dependency.manifest);

                    return ObjectDescription.installed(await reader.declarations());
                }),
        );
        const unserved = depended.flat().filter((each) => !known.has(ObjectDescription.key(each)));

        // declare the package's own types against the served ones and those of its dependencies
        const declared = this.#pool().declare([...unserved, ...owned]);

        // describe its durable objects' tables, placing the other packages' they refer to
        const others = this.#context.objects.filter((type) => type.package.id !== packageId);
        const tables = new Set(
            [...others, ...declared]
                .filter((type) => type.storage === "durable")
                .flatMap((type) => type.tables),
        );
        const state = await this.#options.blobs.writeJson(
            DatabaseState.of([...tables], { package: packageId }),
        );

        return [
            {
                package: PackageManifest.package(build.manifest),
                name: MAIN_DATABASE,
                kind: DatabaseKind.name,
                spec: { replicated: [] },
                state,
            },
        ];
    }
}

/** Describe what an installation's grant waits for: a package it needs, or a type it lists. */
export async function describeUngranted(
    call: Pick<MethodContextOf<typeof rollout, "deploy">, "read" | "authorizer">,
    target: Rollout & { readonly installation: NamedInstallation },
    revision: InstallationRevision,
    description: PackageDescription | undefined,
): Promise<string | undefined> {
    // wait for each package the installation needs served on the space's chain
    const rows = await chainInstallations(call, target.scope);
    const rollouts = await call.read(rollout).findMany({ where: { scope: target.scope } });
    const isServed = (row: (typeof rows)[number]) =>
        row.origin === "base" ||
        rollouts.some((each) => each.id === row.id && each.servedRevisionId !== null);
    const waiting = Rollout.needed(target.installation, revision, description)
        .filter((entry) => !rows.some((row) => row.packageId === entry.package.id && isServed(row)))
        .map((entry) => entry.package.name);

    // and for each type of another package in its space grant that the space does not serve yet
    const granted =
        Rollout.permissions(target.installation, description, "installation").space ?? [];
    const unknown = granted
        .filter((permission) => permission.packageId !== target.installation.packageId)
        .filter((permission) => call.authorizer.find(permission) === undefined)
        .map((permission) => `${permission.packageId}/${permission.type}`);
    const unserved = [...new Set([...waiting, ...unknown])];

    return unserved.length === 0
        ? undefined
        : `waiting for the space to serve ${unserved.join(", ")}`;
}

/** Read the installations on a space's chain: its own, its ancestors' and the universe's. */
async function chainInstallations(
    call: Pick<MethodContextOf<typeof rollout, "deploy">, "read">,
    scope: Identifier<"space">,
) {
    // read the space's ancestors, then the installations of each scope on its chain
    const [link] = await Scope.read(call, [scope]);
    const { ancestors } = present(link, `the scope of space ${scope}`);

    return call
        .read(installation)
        .findMany({ where: { scope: { in: [scope, ...ancestors, Scope.universe.id] } } });
}

/** Deploy an application of a served space, then relate it at the residency its space homes. */
export async function reconcileApplication(
    target: Rollout & { readonly installation: NamedInstallation },
    reconciliation: ObjectReconciliation,
    builds: Pick<BuildStore, "contents">,
): Promise<undefined> {
    // deploy the application of a served space
    const [served] = await reconciliation.database
        .select({ id: space.table.id })
        .from(space.table)
        .where(and(eq(space.table.id, target.scope), Space.served()));
    if (served === undefined) {
        return undefined;
    }
    await reconciliation.mutate([rollout.call("deploy", rollout.target(target))]);

    // read what the committed deploy left: the rollout following its revision, or none once removed
    const snapshot = Snapshot.live(reconciliation.database);
    const rows = await snapshot.read(rollout.table).findMany({ where: { id: target.id } });
    const [deployed] = await Rollout.read(snapshot, rows);
    const build =
        deployed?.revisionId === null || deployed === undefined
            ? undefined
            : await InstallationRevision.reader(snapshot, deployed.revisionId, builds);
    const description = build === undefined ? undefined : await PackageDescription.of(build);
    const set = Rollout.permissions((deployed ?? target).installation, description, "installation");

    // relate it at the universe as its space, through the uplink, converging on each pass
    const calls = await Rollout.relate(snapshot, deployed ?? target, set);
    if (calls.length > 0) {
        const delegator = principal.space.reference(Scope.universe.id, target.scope);
        const lending = Caller.acting(delegator, reconciliation.caller.subject);
        await reconciliation.store.mutate(AccessContext.of(lending, reconciliation.now), {
            id: RequestId.create(reconciliation.store.environment),
            calls,
        });
    }

    return undefined;
}

/** Remove a drained application's deployments with their instances and captures. */
export async function finalizeApplication(
    context: MethodContextOf<typeof rollout, "finalize">,
    target: Rollout & { readonly installation: NamedInstallation },
    options: Pick<ApplicationOptions, "binder">,
): Promise<void> {
    // release its owned resources to their retention
    await options.binder.attach(context, target, []);

    // delete its retired deployments with their instances and captures
    const retired = await context
        .read(deployment)
        .findMany({ where: { installationId: target.id } });
    const ids = retired.map((entry) => entry.id);
    const instances = await context
        .read(instance)
        .findMany({ where: { deploymentId: { in: ids } } });
    const captures = await context.read(capture).findMany({ where: { deploymentId: { in: ids } } });
    await context.mutate(context.self, [
        ...instances.map((row) => instance.call("delete", { id: row.id })),
        ...captures.map((row) => capture.call("delete", { id: row.id })),
        ...retired.map((row) => deployment.call("delete", { id: row.id })),
    ]);

    // drop its grant and its own types' permissions
    await Rollout.upsertAccess(context, target, undefined);
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
                canonicalize(workload.description.sandbox) ===
                    canonicalize(entry.description.sandbox) &&
                canonicalize(workload.directories) === canonicalize(entry.directories) &&
                canonicalize(workload.enabled) === canonicalize(entry.enabled),
        )
    );
}

/** List the paths of directory grants. */
function joinPaths(grants: readonly DirectoryGrant[]): string {
    return grants.map((grant) => grant.path).join(", ");
}

/** Read the release a build runs and the oldest caller release its package serves. */
async function servedRelease(build: BuildReader): Promise<{ release: Version; since?: Version }> {
    // take the build's release and the package's oldest served release
    const release = Version.parse(build.manifest.package.version);
    const since = (await PackageDescription.of(build))?.since;

    return since === undefined ? { release } : { release, since };
}

/** Read the callee release of each installation need. */
async function pinnedReleases(
    build: BuildReader,
    needs: readonly ResourceNeed[],
): Promise<ReadonlyMap<string, InstallationBindingState>> {
    // read the build's exact dependencies when an installation of another package is needed
    const own = PackageManifest.package(build.manifest);
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

    // map each installation need to the release of its package the build compiled against
    const pins = new Map<string, InstallationBindingState>();
    for (const { need, callee } of callees) {
        const release = compiled(callee);
        if (release === undefined) {
            throw StackError.of(
                "DEFINITION_INVALID",
                `the build binds installation ${need.name} of ${callee} without depending on it`,
            );
        }
        pins.set(Binding.key(need.package.id, need.name), { release: Version.parse(release) });
    }

    return pins;
}

/** Describe a declaration as a need, writing its required state as a blob. */
async function describeNeed(
    build: BuildReader,
    declaration: Awaited<ReturnType<BuildReader["declarations"]>>[number],
    blobs: BlobStore,
): Promise<ResourceNeed> {
    // split a resource's description into its kind, specification and required state
    const owner = PackageManifest.package(build.manifest);
    if (declaration.kind === RESOURCE_DECLARATION) {
        const described = ResourceDescription.catchall(schema.json()).parse(
            declaration.description,
        );
        const { name, kind, spec, ...state } = described;

        return { package: owner, name, kind, spec, state: await blobs.writeJson(state) };
    }

    // take another kind's description as its specification
    const described = OtherDescription.parse(declaration.description);
    const { name, ...spec } = described;

    return { package: owner, name, kind: declaration.kind, spec, state: await blobs.writeJson({}) };
}

/** Find the compare function a kind's package declares for a kind, as its export and name. */
async function kindCompare(
    build: BuildReader,
    kind: string,
): Promise<{ readonly subpath: string; readonly name: string } | undefined> {
    // read the package's declaration constructors and the kinds they describe
    const definition = PackageDefinition.parse(
        JSON.parse(new TextDecoder().decode(await build.load("destack.json"))),
    );
    const described = Object.values(definition.declarations ?? {}).flatMap(
        (constructor) => constructor.describes,
    );
    const compare = described.find(
        (each) => each.kind === kind && "function" in each && each.compare !== undefined,
    );
    const reference = compare !== undefined && "compare" in compare ? compare.compare : undefined;
    if (reference === undefined) {
        return undefined;
    }

    // split the reference into its export subpath and the function's name
    const [subpath = "", name = ""] = reference.split("#");

    return { subpath, name };
}
