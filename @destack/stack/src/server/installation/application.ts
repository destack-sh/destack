import { account, Entitlement, Space, space } from "@destack/account/object";
import type { ObjectType } from "@destack/object";
import { Setting } from "@destack/setting";
import { SettingValue } from "@destack/setting/object";
import { DeclarationReference } from "@destack/package/declare";
import {
    AccessContext,
    Caller,
    Manager,
    PermissionReference,
    PermissionScope,
    principal,
} from "@destack/access";
import {
    and,
    type BlobStore,
    type DatabaseConnection,
    DatabaseKind,
    eq,
    Snapshot,
} from "@destack/db";
import {
    installation,
    InstallationBindingSpec,
    InstallationKind,
    type MethodContext,
    type MethodContextOf,
    type ObjectReconciliation,
    PackageDescription,
} from "@destack/object";
import {
    HOST_SANDBOX,
    Sandbox,
    DeclarationName,
    DirectoryGrant,
    graph,
    PackageDefinition,
    type PackageId,
    type DependencyRelease,
    type PackageRelease,
} from "@destack/package";
import { type BuildReader, Lock, PackageManifest } from "@destack/package/manifest";
import { ServerRuntime } from "@destack/package/runtime";
import {
    Address,
    Plan,
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
    type VersionRange,
} from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import type { FamilyError } from "@destack/error";
import { StackError, type StackReason } from "../../error/index.ts";
import type { Runtime } from "../../runtime/runtime.ts";
import {
    Build,
    Deployment,
    deployment,
    InstallationRevision,
    installationRevision,
    instance,
    Rollout,
    rollout,
    type InstallationRollout,
    type ResourceNeed,
    type StackEvaluation,
} from "../../object/index.ts";
import type { Binder } from "../resource/binding.ts";
import { SpaceBuildStore, type Registry } from "./build.ts";
import { Approval } from "./approval.ts";
import type { Forge } from "./follow.ts";
import { HOOK_WORKLOAD } from "../../workload/hook.ts";
import { type DeploymentRuntime, PROCESS_RUNTIME } from "../../object/index.ts";
import { BOOTSTRAP_PACKAGES } from "../../declare/index.ts";
import { RolloutTarget } from "./target.ts";
import { InstallationAccess } from "./access.ts";

/** The condition of an installation whose bound targets are each ready. */
const BOUND = { status: "true", reason: "Bound", message: "every bound target is ready" } as const;

/** The description of a declaration of a kind other than resource: its name and specification. */
const OtherDescription = schema.object({ name: DeclarationName }).catchall(schema.json());

/** What a machine deploys applications with: its binder, its builds and its server runtimes. */
export interface ApplicationOptions {
    /** The kinds of objects bindings target. */
    readonly binder: Binder;
    /** The space's builds, read through to the machine's cache. */
    readonly builds: SpaceBuildStore;
    /** The registry of published releases. */
    readonly registry: Registry;
    /** The server runtimes the machine runs, in preference order. */
    readonly runtimes: readonly Runtime[];
    /** The Destack versions whose releases the machine runs: its own and the oldest it supports. */
    readonly destack: VersionRange;
    /** Whether the machine checks capabilities' features against their accounts' entitlements. */
    readonly isEnforcingEntitlements: boolean;
    /** The space's database, with the replicas of its account's entitlements. */
    readonly database: DatabaseConnection;
    /** The release the machine links of each package, by package. */
    readonly linked: ReadonlyMap<PackageId, PackageRelease>;
    /** The releases the machine ships, by package. */
    readonly shipped: ReadonlyMap<PackageId, PackageRelease>;
    /** The locks of the releases the machine runs in its process: those it ships or links. */
    readonly trusted: ReadonlySet<Digest>;
    /** The space's blobs, storing the states its revisions require. */
    readonly blobs: BlobStore;
    /** The forge whose references and tags installations select, null without one. */
    readonly forge: Forge | null;
    /** The types every space knows beside those it serves, which descriptions refer to. */
    readonly known: readonly ObjectType[];
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
    /** The package its build declares: requests and capabilities, absent for workloads alone. */
    readonly description: PackageDescription | undefined;
}

/** The releases a deployment plan runs and calls. */
interface Releases {
    /** The release the build serves and the oldest caller release its package serves. */
    readonly served: Awaited<ReturnType<typeof servedRelease>>;
}

/** One workload of a build's server output. */
interface BuildWorkload {
    /** The output with it. */
    readonly output: string;
    /** The package-local workload name. */
    readonly name: string;
    /** The output's runtime, or the machine's process for a server the store runs. */
    readonly runtime: DeploymentRuntime;
    /** The workload's description, with the sandbox it runs with. */
    readonly description: (typeof deployment.table.$inferSelect)["description"];
    /** The directories its `fs` capability runs with, none without the capability. */
    readonly directories: readonly DirectoryGrant[];
    /** The package's capabilities it runs with. */
    readonly enabled: readonly string[];
}

/** The workload of a server the store runs in the machine's process. */
const PROCESS_WORKLOAD = "server";

/** The ready reasons of an application refusing to deploy a build, apart from those it waits on. */
/** Why an application refuses to deploy a build, the lasting failure its Synced records. */
type Refusal = FamilyError<StackReason>;

/** A space's application installation: followed by its controller, deployed by its deploy call. */
export class ApplicationInstallation {
    /** The installation's rollout, with its installation's fields. */
    readonly #target: InstallationRollout;
    /** What the machine follows releases and deploys with. */
    readonly #options: ApplicationOptions;

    /** Reconcile or deploy an application installation with the machine's options. */
    constructor(target: InstallationRollout, options: ApplicationOptions) {
        this.#target = target;
        this.#options = options;
    }

    /** Revise an unmanaged installation, deploy it in a served space, then relate it. */
    async reconcile(reconciliation: ObjectReconciliation): Promise<undefined> {
        // leave the application of a space not served here
        const target = this.#target;
        const [served] = await reconciliation.database
            .select({ id: space.table.id })
            .from(space.table)
            .where(and(eq(space.table.id, target.scope), Space.served()));
        if (served === undefined) {
            return undefined;
        }

        // revise an unmanaged installation once an input changed, its manager revising a managed one, then deploy it
        const isRevised = Manager.isManaged(target.installation)
            ? await this.#isSuperseded(reconciliation)
            : await this.#isChanged(reconciliation);
        if (isRevised) {
            await RolloutTarget.revise(reconciliation, [target], this.#options);
        }
        await reconciliation.mutate([
            rollout.call(
                "deploy",
                rollout.target({ scope: target.scope, id: target.installationId }),
            ),
        ]);

        // read what the committed deploy left: the rollout targeting its revision, or none once removed
        const snapshot = Snapshot.live(reconciliation.database);
        const rows = await snapshot
            .read(rollout.table)
            .findMany({ where: { installationId: target.installationId } });
        const [deployed] = await Rollout.read(snapshot, rows);
        const { builds } = this.#options;
        const build =
            deployed?.revisionId === null || deployed === undefined
                ? undefined
                : await InstallationRevision.reader(snapshot, deployed.revisionId, builds);
        const description = build === undefined ? undefined : await PackageDescription.of(build);
        const set = Rollout.permissions((deployed ?? target).installation, description, "space");

        // relate it at the universe as its space, through the uplink, converging on each pass
        const calls = await InstallationAccess.relate(snapshot, target, deployed ?? target, set);
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

    /** Report whether an input of the targeted revision changed. */
    async #isChanged(reconciliation: ObjectReconciliation): Promise<boolean> {
        // revise a release or tag selection, which may pick another build, and an installation without a revision
        const target = this.#target;
        const { kind } = target.installation.selection;
        if (kind === "release" || kind === "tag" || target.revisionId === null) {
            return true;
        }

        // revise again past the generation the revision was made at, or once a bound target moved
        const snapshot = Snapshot.live(reconciliation.database);
        const revision = await snapshot
            .read(installationRevision.table)
            .findFirst({ where: { id: target.revisionId } });

        return (
            revision === undefined ||
            revision.generation !== target.installation.generation ||
            (await this.#options.binder.isOutdated(snapshot, target, revision))
        );
    }

    /** Report whether the installation targets a build this machine no longer runs. */
    async #isSuperseded(reconciliation: ObjectReconciliation): Promise<boolean> {
        // read the targeted revision's build, none without a revision
        const target = this.#target;
        if (target.revisionId === null) {
            return false;
        }
        const snapshot = Snapshot.live(reconciliation.database);
        const revision = present(
            await snapshot
                .read(installationRevision.table)
                .findFirst({ where: { id: target.revisionId } }),
            `revision ${target.revisionId}`,
        );
        const built = await Build.require(snapshot, revision.scope, revision.buildId);

        return Rollout.isSuperseded(target.installation, built.lock, this.#options.trusted);
    }

    /** Revise the installation: a build at its bindings, retaining each revision once. */
    async revise(
        call: MethodContext,
        input: {
            readonly build: Identifier<"build">;
            readonly dependencies: InstallationRevision["dependencies"];
            readonly states: InstallationRevision["states"];
            readonly evaluation?: StackEvaluation | undefined;
        },
    ): Promise<InstallationRevision> {
        // bind an application's needs, creating and following the installation's own resources
        const target = this.#target;
        const built = await Build.require(
            { read: (table) => call.read(table) },
            target.scope,
            input.build,
        );
        const reader = await this.#options.builds.root(built.lock);
        const description = await PackageDescription.of(reader);
        const { stranded } = this.#selectWorkloads(built.lock, reader, description);
        const isRefused =
            this.#describeRefusal(built.lock, reader, stranded, description) !== undefined ||
            (await this.#describeDatabases(reader)) !== undefined;

        // bind a stack's nothing, and a refused build's selected targets alone, as it creates nothing
        const { bindings: selected } = target.installation;
        const needs = (await this.#readNeeds(reader, [])).filter(
            (need) =>
                !this.#isUnaddressed(need) &&
                (!isRefused || selected[need.package.id]?.[need.name] !== undefined),
        );
        const bindings =
            target.installation.export !== null
                ? []
                : await this.#options.binder.bind(call, target, needs, await this.#running(call));

        // retain the revision once per digest
        const [revision] = await call.mutate(call.self, [
            await RolloutTarget.create(
                target.installation,
                { id: built.id, lock: built.lock },
                input,
                input.evaluation,
                bindings,
                call.environment.crypto,
            ),
        ]);

        return revision;
    }

    /** Declare the revision's needs with the live deployments' needs. */
    async #redeclare(call: MethodContext, revision: InstallationRevision): Promise<void> {
        await this.#options.binder.declare(
            call,
            this.#target,
            { needs: await this.#needs(call, revision), running: await this.#running(call) },
            revision.bindings,
        );
    }

    /** Read the resource needs of a revision's build. */
    async #needs(
        call: MethodContext,
        revision: Pick<InstallationRevision, "buildId">,
    ): Promise<ResourceNeed[]> {
        const built = await Build.require(
            { read: (table) => call.read(table) },
            this.#target.scope,
            revision.buildId,
        );

        return this.#readNeeds(await this.#options.builds.root(built.lock), []);
    }

    /** Read the needs of the revisions the installation's live deployments still run. */
    async #running(call: MethodContext): Promise<ResourceNeed[]> {
        // read the revisions the live deployments run
        const target = this.#target;
        const live = await call.read(deployment).findMany({
            where: { ...Deployment.live(), installationId: target.installationId },
        });
        const running = await call.read(installationRevision).findMany({
            where: { id: { in: [...new Set(live.map((entry) => entry.revisionId))] } },
        });

        // read each one's needs from its build
        const needs = await Promise.all(running.map((revision) => this.#needs(call, revision)));

        return needs.flat();
    }

    /** Deploy the revision of an enabled application, and drain a suspended one. */
    async deploy(call: MethodContextOf<typeof rollout, "deploy">): Promise<void> {
        // read the revision it targets and its live deployments
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
                installationId: target.installationId,
            },
        });

        // retire a deleted application's declarations, keeping it running until its own code finalized its records
        const isDeleted = target.installation.deletedAt !== null;
        if (isDeleted && revision !== undefined) {
            const needs = await this.#needs(call, revision);
            await this.#options.binder.retire(
                call,
                target,
                [...needs, ...(await this.#running(call))],
                revision.bindings,
            );
            if (await this.#options.binder.isFinalizing(call, target)) {
                await this.#relate(call);

                return;
            }
        }

        // drain every deployment of a deleted or suspended application or of one targeting no revision
        if (isDeleted || target.installation.status !== "enabled" || revision === undefined) {
            const active = live.filter((entry) => entry.status === "active");
            await call.mutate(
                call.self,
                active.map((entry) => Deployment.drain(entry)),
            );

            // report an enabled application whose selected release is not published yet
            const { kind } = target.installation.selection;
            const isEnabled = !isDeleted && target.installation.status === "enabled";
            if (isEnabled && revision === undefined && (kind === "release" || kind === "tag")) {
                const message = `${target.installation.name} has no published release for its selection`;
                await call.observe(installation, target.installation, {
                    conditions: {
                        Ready: { status: "false", reason: "ReleaseNotPublished", message },
                    },
                });
            }

            // remove a deleted application once none of its deployments runs and nothing it consumed relates it
            if (isDeleted && live.length === 0) {
                await this.#relate(call);
                await this.#finalize(call);
                await call.mutate(call.self, [
                    installation.call("finalize", installation.target(target.installation)),
                ]);

                return;
            }
        }
        // deploy the revision until each workload runs from the output the machine chooses
        else {
            await this.#deploy(call, revision, live);
        }

        // relate the application as consumer of exactly what its live deployments' revisions bind
        await this.#relate(call);
    }

    /** Release a drained application's resources, delete its deployments and drop its grants. */
    async #finalize(call: MethodContextOf<typeof rollout, "deploy">): Promise<void> {
        // retire every record it declares, and let go of the ones their types keep, such as retained resources
        const target = this.#target;
        await this.#options.binder.retire(call, target, [], []);
        await this.#options.binder.disown(call, target);

        // delete its retired deployments with their instances
        const retired = await call
            .read(deployment)
            .findMany({ where: { installationId: target.installationId } });
        const ids = retired.map((entry) => entry.id);
        const instances = await call
            .read(instance)
            .findMany({ where: { deploymentId: { in: ids } } });
        await call.mutate(call.self, [
            ...instances.map((row) => instance.call("delete", { id: row.id })),
            ...retired.map((row) => deployment.call("delete", { id: row.id })),
        ]);

        // drop its grant and its own types' permissions
        await InstallationAccess.upsert(call, target, undefined);
    }

    /** Relate the application as consumer of exactly what its live deployments' revisions bind. */
    async #relate(call: MethodContextOf<typeof rollout, "deploy">): Promise<void> {
        // relate it as consumer of what its live deployments' revisions bind
        const target = this.#target;
        const digest = await this.#options.binder.relate(call, target, target.consumption);

        // record what it related once that changed
        if (digest !== target.consumption) {
            await call.mutate(call.self, [
                rollout.call("update", {
                    ...rollout.target({ scope: target.scope, id: target.installationId }),
                    consumption: digest,
                }),
            ]);
        }
    }

    /** Deploy each new workload output of a revision's build and drain the replaced deployments. */
    async #deploy(
        call: MethodContextOf<typeof rollout, "deploy">,
        revision: InstallationRevision,
        live: readonly Deployment[],
    ): Promise<void> {
        // wait while the stack managing a host installation moves it off a build this machine does not run
        const target = this.#target;
        const built = await Build.require(
            { read: (table) => call.read(table) },
            revision.scope,
            revision.buildId,
        );
        if (Rollout.isSuperseded(target.installation, built.lock, this.#options.trusted)) {
            const message = `waiting for the space's stack to move it to a release this machine runs`;
            await call.observe(installation, target.installation, {
                conditions: { Ready: Rollout.waiting("WaitingForStack", message) },
            });

            return;
        }

        // wait for a build this machine lacks, which a pull or its stack brings, as an image pull waits
        if (!(await this.#options.builds.contains(built.lock))) {
            await call.observe(installation, target.installation, {
                conditions: { Ready: Rollout.waitingForBuild(built.lock) },
            });

            return;
        }

        // read the workloads the machine runs
        const build = await this.#open(call, revision);
        const description = await PackageDescription.of(build);

        // select the workloads the machine runs
        const { workloads, stranded } = this.#selectWorkloads(built.lock, build, description);

        // refuse an unrunnable workload, an undeclared or unentitled capability or directory, or a database
        const refusal =
            this.#describeRefusal(built.lock, build, stranded, description) ??
            (await this.#describeOutdated(built.lock)) ??
            (await this.#describeUnentitled(call, description)) ??
            (await this.#describeUninstalled(call, revision, description)) ??
            (await this.#describeDatabases(build));
        if (refusal !== undefined) {
            throw refusal;
        }

        // stop for a broken upgrade chain or an approval of the upgrade from the applied release
        if (await this.#holdForUpgrade(call, revision, build, built.lock)) {
            return;
        }

        // split the workloads into fresh and replaced ones, and upsert only the access when nothing changes
        const { fresh, replaced } = await this.#partition(call, revision, workloads, live);
        if (
            target.appliedRevisionId === revision.id &&
            fresh.length === 0 &&
            replaced.length === 0
        ) {
            // grant again once the space serves the revision and what its grant names
            if (await this.#waitForSpace(call, revision, description)) {
                return;
            }
            await InstallationAccess.upsert(call, target, description);
            await this.#redeclare(call, revision);
            await this.#observeApplied(call, workloads.length);

            return;
        }

        // refuse installation needs without a bound address
        const needs = await this.#readNeeds(build, workloads);
        const unbound = this.#describeUnboundInstallations(revision, needs);
        if (unbound !== undefined) {
            throw unbound;
        }

        // attach, drain the replaced deployments and activate the fresh ones
        await this.#deployPlan(call, revision, {
            build,
            workloads,
            fresh,
            replaced,
            needs,
            description,
        });
    }

    /** Attach the application's needs, wait for its space, grant, drain and activate. */
    async #deployPlan(
        call: MethodContextOf<typeof rollout, "deploy">,
        revision: InstallationRevision,
        plan: DeploymentPlan,
    ): Promise<void> {
        // wait for a provider of each kind no provider here serves yet
        const target = this.#target;
        const { build, workloads, fresh, replaced, needs, description } = plan;
        const unprovided = this.#options.binder.unprovided(needs);
        if (unprovided.length > 0) {
            const message = `waiting for a provider of ${unprovided.join(", ")} resources`;
            await call.observe(installation, target.installation, {
                conditions: {
                    Bound: Rollout.waiting("WaitingForResources", message),
                    Ready: Rollout.waiting("WaitingForResources", message),
                },
            });

            return;
        }

        // wait for the resources its revision binds
        const bindings = revision.bindings;
        if (!(await this.#options.binder.ready(call, target, bindings, needs))) {
            const message = "bound targets wait for their resources";
            await call.observe(installation, target.installation, {
                conditions: {
                    Bound: Rollout.waiting("WaitingForResources", message),
                    Ready: Rollout.waiting("WaitingForResources", message),
                },
            });

            return;
        }

        // wait for the space settings its servers read at start, which its stack may set after installing it
        const placed = (await Setting.list(build)).filter(
            (declared) => declared.definition.scope === "space",
        );
        const unset = await SettingValue.missing({ read: (table) => call.read(table) }, placed, {
            scope: target.scope,
            installation: target.installationId,
        });
        if (unset.length > 0) {
            const keys = unset.map((declared) => DeclarationReference.key(declared.reference));
            const message = `waiting for settings ${keys.join(", ")}`;
            await call.observe(installation, target.installation, {
                conditions: {
                    Bound: Rollout.waiting("WaitingForSettings", message),
                    Ready: Rollout.waiting("WaitingForSettings", message),
                },
            });

            return;
        }

        // wait for the space to serve the revision once its targets are bound, and the packages whose types its grants name
        if (await this.#waitForSpace(call, revision, description)) {
            return;
        }

        // grant the installation exactly what its package and its enabled capabilities request
        await InstallationAccess.upsert(call, target, description);

        // drain the replaced deployments down to one active deployment per workload
        await call.mutate(
            call.self,
            replaced.map((entry) => Deployment.drain(entry)),
        );

        // activate each fresh workload's deployment
        const releases = {
            served: await servedRelease(build),
        };
        for (const workload of fresh) {
            await this.#activate(call, revision, workload, releases);
        }

        // declare again what the revision and the live deployments need, retiring what only drained ones did
        await this.#redeclare(call, revision);

        // observe the revision applied
        await this.#observeApplied(call, workloads.length);
        await call.mutate(call.self, [
            rollout.call("update", {
                ...rollout.target({ scope: target.scope, id: target.installationId }),
                ...Rollout.applied(target, revision.id),
            }),
        ]);
    }

    /** Observe the applied revision deployed at the installation's generation. */
    async #observeApplied(
        call: MethodContextOf<typeof rollout, "deploy">,
        workloads: number,
    ): Promise<void> {
        await call.observe(installation, this.#target.installation, {
            conditions: {
                Bound: BOUND,
                Ready: {
                    status: "true",
                    reason: "Deployed",
                    message: `${workloads} workloads deployed`,
                },
            },
            fields: { plan: null, approvedPlan: null },
        });
    }

    /** Describe a build of a lock requiring a newer release of a builtin than the machine links. */
    async #describeOutdated(lock: Digest): Promise<Refusal | undefined> {
        // index the version of each builtin the machine links by package name
        const { builds, linked } = this.#options;
        const builtins = new Map(
            [...linked.values()].map((release) => [release.package.name, release.package.version]),
        );

        // refuse the first build requiring a release of a builtin newer than the linked one
        for (const build of Lock.builds(await builds.lock(lock))) {
            const manifest = await builds.get(build);
            for (const [name, required] of Object.entries(manifest.requires)) {
                const running = builtins.get(name);
                const isVersion = Version.safeParse(required).success;
                if (
                    running !== undefined &&
                    isVersion &&
                    Version.compare(Version.release(running), Version.release(required)) < 0
                ) {
                    const { name: owner, version } = manifest.package;
                    const message = `cannot deploy '${owner}@${version}': it needs ${name} ${Version.release(required)} or newer, and this machine runs ${running}`;

                    return StackError.of("RELEASE_BLOCKED", message);
                }
            }
        }

        return undefined;
    }

    /** Describe why a build cannot deploy. */
    #describeRefusal(
        lock: Digest,
        build: BuildReader,
        stranded: readonly string[],
        description: PackageDescription | undefined,
    ): Refusal | undefined {
        // refuse a release built with a newer Destack than the machine runs
        const built = build.manifest.destack;
        const { oldest, newest } = this.#options.destack;
        const name = build.manifest.package.name;
        if (Version.compare(built, newest) > 0) {
            const message = `${name} is built with Destack ${built}, newer than this machine's ${newest}`;

            return StackError.of("RELEASE_UNSUPPORTED", message);
        }
        // refuse one built with an older Destack than the machine supports
        else if (Version.compare(Version.release(built), oldest) < 0) {
            const message = `${name} is built with Destack ${built}, older than this machine supports (${oldest})`;

            return StackError.of("RELEASE_UNSUPPORTED", message);
        }
        // refuse a host installation of a build the machine neither ships nor links
        else if (
            this.#target.installation.isolation === "host" &&
            !this.#options.trusted.has(lock)
        ) {
            const message = `${name} runs on the host, which only releases the machine ships or links may`;

            return StackError.of("PROCESS_ISOLATION_DENIED", message);
        }
        // refuse a build with an unrunnable workload
        else if (stranded.length > 0) {
            const runtimes = this.#options.runtimes.map((runtime) => runtime.name).join(", ");
            const message = `workloads ${stranded.join(", ")} have no output on ${runtimes}`;

            return StackError.of("RELEASE_UNSUPPORTED", message);
        }

        // refuse a package routing hooks that nothing serves: its build's hook workload, or the store
        const isHooked = Object.values(description?.objects ?? {}).some(
            (routed) => Object.keys(routed.hooks).length > 0,
        );
        const isHosted = this.#isStored(lock);
        const isServed =
            isHosted ||
            Object.values(build.manifest.outputs).some((output) =>
                Object.hasOwn(output.workloads, HOOK_WORKLOAD),
            );
        if (isHooked && !isServed) {
            const message = `${build.manifest.package.name} routes hooks, but nothing serves them: declare @destack/stack`;

            return StackError.of("HOOK_BLOCKED", message);
        }

        // refuse a package declaring mounts that nothing answers: a workload of its build, or the store
        const isAdapted = Object.keys(description?.mounts ?? {}).length > 0;
        const isAnswered =
            isHosted ||
            Object.values(build.manifest.outputs).some((output) =>
                Object.keys(output.workloads).some((workload) => workload !== HOOK_WORKLOAD),
            );
        if (isAdapted && !isAnswered) {
            const message = `${build.manifest.package.name} declares mounts, but no workload of its build answers them`;

            return StackError.of("MOUNT_BLOCKED", message);
        }

        // refuse allowing a capability or granting a directory the build does not declare
        const undeclared = this.#describeUndeclaredGrants(build, description);

        return undeclared === undefined
            ? undefined
            : StackError.of("CAPABILITY_BLOCKED", undeclared);
    }

    /** Describe an enabled capability needing a feature the space's account lacks. */
    async #describeUnentitled(
        call: MethodContextOf<typeof rollout, "deploy">,
        description: PackageDescription | undefined,
    ): Promise<Refusal | undefined> {
        // check nothing on a machine enforcing no entitlements
        if (!this.#options.isEnforcingEntitlements) {
            return undefined;
        }

        // list the features the enabled capabilities need
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
                return StackError.of(
                    "CAPABILITY_BLOCKED",
                    `cannot use capability '${name}': it needs '${feature.name}': ${condition.message}`,
                );
            }
        }

        return undefined;
    }

    /** Describe a package the application needs that nothing in the space serves. */
    async #describeUninstalled(
        call: MethodContextOf<typeof rollout, "deploy">,
        revision: InstallationRevision,
        description: PackageDescription | undefined,
    ): Promise<Refusal | undefined> {
        // read the space's own packages and the packages installed in it
        const target = this.#target;
        const rows = await call.read(installation).findMany({ where: { scope: target.scope } });
        const installed = new Set([
            ...Object.values(BOOTSTRAP_PACKAGES).map((handle) => handle.id),
            ...rows.map((row) => row.packageId),
        ]);

        // refuse the first needed package missing, naming the capability needing it
        const missing = Rollout.needed(target.installation, revision, description).find(
            (entry) => !installed.has(entry.package.id),
        );
        if (missing === undefined) {
            return undefined;
        }
        const { name } = missing.package;

        return StackError.of(
            "DEPENDENCY_BLOCKED",
            missing.capability === undefined
                ? `${target.installation.name} needs ${name} installed`
                : `capability ${missing.capability} needs ${name} installed`,
        );
    }

    /** Wait while the space does not serve the revision or what its grant names. */
    async #waitForSpace(
        call: MethodContextOf<typeof rollout, "deploy">,
        revision: InstallationRevision,
        description: PackageDescription | undefined,
    ): Promise<boolean> {
        // report the wait on its bound targets' readiness
        const unserved = await this.#describeUnserved(call, revision, description);
        if (unserved !== undefined) {
            await call.observe(installation, this.#target.installation, {
                conditions: { Bound: BOUND, Ready: Rollout.waiting("WaitingForSpace", unserved) },
            });
        }

        return unserved !== undefined;
    }

    /** Describe what the space does not serve yet: the revision, or a package it needs. */
    async #describeUnserved(
        call: MethodContextOf<typeof rollout, "deploy">,
        revision: InstallationRevision,
        description: PackageDescription | undefined,
    ): Promise<string | undefined> {
        // wait for the revision itself
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
        const databases = declarations.filter((declaration) => DatabaseKind.has(declaration));
        if (databases.length === 0) {
            return undefined;
        }

        // refuse them, since the space's main database stores every installation's tables
        const names = databases.map((declaration) => declaration.name).join(", ");
        const message = `${this.#target.installation.packageId} declares the databases ${names}: the space's main database keeps every installation's tables`;

        return StackError.of("DATABASE_BLOCKED", message);
    }

    /** Stop the deploy for an unapproved upgrade or access growth, and report the stop. */
    async #holdForUpgrade(
        call: MethodContextOf<typeof rollout, "deploy">,
        revision: InstallationRevision,
        build: BuildReader,
        lock: Digest,
    ): Promise<boolean> {
        // pass a first deploy and a redeploy of the applied revision
        const target = this.#target;
        if (target.appliedRevisionId === null || target.appliedRevisionId === revision.id) {
            return false;
        }

        // read the applied build, the upgrade from it and the access the build asks beyond it
        const applied = await call
            .read(installationRevision)
            .findFirst({ where: { id: target.appliedRevisionId } });
        const previous = await this.#open(
            call,
            present(applied, `revision ${target.appliedRevisionId}`),
        );
        const upgrade = await this.#planUpgrade(previous, build, lock);
        const access = await this.#planAccess(previous, build);
        const plan = { steps: [...upgrade.steps, ...access.steps] };

        // observe the upgrade waiting for approval
        const waiting = await Approval.waiting(call, target, plan, call.environment.crypto);
        if (waiting !== undefined) {
            await call.observe(installation, target.installation, waiting);
        }

        return waiting !== undefined;
    }

    /** Split a revision's workloads into fresh ones and the active deployments they replace. */
    async #partition(
        call: MethodContextOf<typeof rollout, "deploy">,
        revision: InstallationRevision,
        workloads: readonly BuildWorkload[],
        live: readonly Deployment[],
    ): Promise<{ readonly fresh: BuildWorkload[]; readonly replaced: Deployment[] }> {
        // reuse the active deployments of the revision's workloads, a binding change making another revision
        const active = live.filter((entry) => entry.status === "active");
        const chosen = active.filter((entry) => isChosen(entry, revision, workloads));

        // deploy the other workloads anew in place of the other active deployments
        const fresh = workloads.filter(
            (workload) => !chosen.some((entry) => entry.workload === workload.name),
        );
        const replaced = active.filter((entry) => !chosen.includes(entry));

        return { fresh, replaced };
    }

    /** Report whether a need calls an installation without an address of its own or a selection. */
    #isUnaddressed(need: ResourceNeed): boolean {
        return (
            need.kind === InstallationKind.name &&
            InstallationBindingSpec.parse(need.spec).address === undefined &&
            this.#target.installation.bindings[need.package.id]?.[need.name] === undefined
        );
    }

    /** Describe the installation needs of the application without an address. */
    #describeUnboundInstallations(
        revision: InstallationRevision,
        needs: readonly ResourceNeed[],
    ): Refusal | undefined {
        // list the installation needs with no address of their own that the revision leaves unbound
        const bound = revision.bindings;
        const unbound = needs.filter(
            (need) =>
                this.#isUnaddressed(need) &&
                !bound.some(
                    (entry) => entry.packageId === need.package.id && entry.name === need.name,
                ),
        );
        if (unbound.length === 0) {
            return undefined;
        }
        const names = unbound.map((need) => `${need.package.id} ${need.name}`).join(", ");

        return StackError.of(
            "BINDING_BLOCKED",
            `cannot apply installations '${names}': bind them to their addresses first`,
        );
    }

    /** Activate a fresh workload's deployment of a revision. */
    async #activate(
        call: MethodContextOf<typeof rollout, "deploy">,
        revision: InstallationRevision,
        workload: BuildWorkload,
        releases: Releases,
    ): Promise<void> {
        await call.mutate(call.self, [
            deployment.call("create", {
                installationId: this.#target.installationId,
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
    }

    /** Open the build a revision evaluated from the build store. */
    #open(
        call: MethodContextOf<typeof rollout, "deploy">,
        revision: Pick<InstallationRevision, "id">,
    ): Promise<BuildReader> {
        return InstallationRevision.reader(call, revision.id, this.#options.builds);
    }

    /** Plan the upgrade steps from an applied build to the next by its kinds' comparisons. */
    async #planUpgrade(previous: BuildReader, next: BuildReader, lock: Digest): Promise<Plan> {
        // skip rollbacks
        const from = previous.manifest.package.version;
        const release = next.manifest.package.version;
        if (Version.compare(release, from) <= 0) {
            return { steps: [] };
        }

        // compare each kept declaration its kind compares, then plan the rest from the applied release
        const declarations = await previous.declarations();
        const history = {
            release: from,
            declarations,
            vocabulary: Vocabulary.advance({}, declarations, from),
        };
        const { builds } = this.#options;
        const compared = await this.#compare(declarations, from, next, await builds.lock(lock));
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

        return { steps };
    }

    /** Plan a grant for each permission a next build asks of its space beyond the applied build. */
    async #planAccess(previous: BuildReader, next: BuildReader): Promise<Plan> {
        // read what each build asks of the space with the installation's enabled capabilities
        const installed = this.#target.installation;
        const asked = async (build: BuildReader) =>
            Rollout.permissions(installed, await PackageDescription.of(build), "space");
        const before = await asked(previous);
        const after = await asked(next);

        // grant each permission the applied build did not ask for, scope by scope
        const steps = PermissionScope.options.flatMap((scope) => {
            const held = new Set(
                (before[scope] ?? []).map((each) => PermissionReference.key(each)),
            );

            return (after[scope] ?? [])
                .filter((each) => !held.has(PermissionReference.key(each)))
                .map((each) => ({
                    action: "grant" as const,
                    target: Address.join(
                        "installation",
                        installed.name,
                        "access",
                        scope,
                        each.packageId,
                        each.type,
                        each.name,
                    ),
                    risk: "privileged" as const,
                    detail: `request ${each.name} on ${each.type}${scope === "space" ? "" : ` in its ${scope}`}`,
                }));
        });

        return { steps };
    }

    /** Compare the declarations a next build keeps by their kinds' compare, by address. */
    async #compare(
        declarations: readonly graph.Declaration[],
        from: string,
        next: BuildReader,
        lock: Lock,
    ): Promise<ReadonlyMap<string, Plan>> {
        // pair the applied declarations with the next build's by kind and name
        const kept = new Map(
            declarations.map((each) => [Address.join(each.kind, each.name), each] as const),
        );
        // find each kind's package in the next build's lock
        const { builds } = this.#options;
        const entries = await Lock.entries(lock, (digest) => builds.get(digest));
        const owner = PackageManifest.package(next.manifest);
        const [runtime] = this.#options.runtimes;
        const compared = new Map<string, Plan>();
        for (const after of await next.declarations()) {
            // find the kind's compare function in its package's build
            const address = Address.join(after.kind, after.name);
            const before = kept.get(address);
            if (before === undefined) {
                continue;
            }
            const locked =
                after.kindPackageId === owner.id
                    ? await builds.cache.store.putLock(lock)
                    : await this.#kindLock(lock, entries, after.kindPackageId);
            const kind = await builds.root(locked);
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
            const plan = await machine.call(locked, exported.path, reference.name, [
                entry(before, from),
                entry(after, owner.version),
            ]);
            compared.set(address, Plan.parse(plan));
        }

        return compared;
    }

    /** Read the lock of a package defining a kind: in the next build's, else the linked one. */
    async #kindLock(
        lock: Lock,
        entries: Readonly<Record<string, DependencyRelease>>,
        packageId: PackageId,
    ): Promise<Digest> {
        // take the builtin the machine links, as every build resolves a linked package to it
        const { builds, linked } = this.#options;
        const builtin = linked.get(packageId);
        if (builtin !== undefined) {
            await builds.pull(packageId, builtin.lock);

            return builtin.lock;
        }

        // derive the package's lock within the next build's, stored where the machine reads it
        const entry = present(
            Object.values(entries).find(
                (each) => each.kind === "destack" && each.package.id === packageId,
            ),
            `the build of the kind package ${packageId}`,
        );

        return await builds.cache.store.putLock(Lock.at(lock, entry.manifest));
    }

    /** Decide whether a build runs in the store's process: linked, or a host installation's. */
    #isStored(lock: Digest): boolean {
        const { installation: installed } = this.#target;

        return (
            this.#options.linked.get(installed.packageId)?.lock === lock ||
            installed.isolation === "host"
        );
    }

    /** Describe the workload of a server the store runs in the machine's process. */
    #processWorkload(description: PackageDescription | undefined): BuildWorkload {
        return {
            output: PROCESS_RUNTIME,
            name: PROCESS_WORKLOAD,
            runtime: PROCESS_RUNTIME,
            description: {
                entrypoint: ".",
                packages: [],
                resources: [],
                secrets: [],
                compute: {},
                sandbox: {},
            },
            directories: [],
            enabled: Rollout.enabled(this.#target.installation, description),
        };
    }

    /** Select a build's workloads: one in the store for a linked or process build, else its own. */
    #selectWorkloads(
        lock: Digest,
        build: BuildReader,
        description: PackageDescription | undefined,
    ): {
        readonly workloads: BuildWorkload[];
        readonly stranded: string[];
    } {
        // serve a linked release or a host installation's build in the store
        if (this.#isStored(lock)) {
            return { workloads: [this.#processWorkload(description)], stranded: [] };
        }

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
        const reaching = target.installation.directories.filter((grant) =>
            DirectoryGrant.isReachingSockets(grant),
        );
        // refuse granting directories without fs
        if (target.installation.directories.length > 0 && access.length === 0) {
            return `${release} enables no fs ability to grant directories ${joinPaths(target.installation.directories)}`;
        }
        // refuse writing directories fs declares for reading
        else if (written.length > 0 && !access.includes("write")) {
            return `${release} enables fs for reading only, refusing to grant writing ${joinPaths(written)}`;
        }
        // refuse directories reaching the host's service sockets, which a Linux sandbox lets connect
        else if (reaching.length > 0) {
            return `${release} may not be granted directories reaching the host's service sockets in /run, /var/run or all of /tmp: ${joinPaths(reaching)}`;
        }

        return undefined;
    }

    /** Read the needs the binder binds: its objects' tables and its workloads' declarations. */
    async #readNeeds(
        build: BuildReader,
        workloads: readonly BuildWorkload[],
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
        const needs = declarations
            .filter((declaration) => this.#options.binder.binds(declaration))
            .map((declaration) => describeNeed(build, declaration));

        // require a description of every used declaration
        const missing = [...used].filter((name) => !needs.some((entry) => entry.name === name));
        if (missing.length > 0) {
            throw StackError.of(
                "DEFINITION_INVALID",
                `build of '${build.manifest.package.name}' is invalid: it is missing declarations '${missing.join(", ")}' that its machine binds`,
            );
        }

        return needs;
    }
}

/** Describe what an installation's grant waits for: a package it needs, or a type it lists. */
export async function describeUngranted(
    call: Pick<MethodContextOf<typeof rollout, "deploy">, "read" | "authorizer">,
    target: InstallationRollout,
    revision: InstallationRevision,
    description: PackageDescription | undefined,
): Promise<string | undefined> {
    // wait for each package the installation needs that neither the space's own packages nor a served installation serve
    const own = new Set(Object.values(BOOTSTRAP_PACKAGES).map((handle) => handle.id));
    const rows = await call.read(installation).findMany({ where: { scope: target.scope } });
    const rollouts = await call.read(rollout).findMany({ where: { scope: target.scope } });
    const isServed = (row: (typeof rows)[number]) =>
        rollouts.some((each) => each.installationId === row.id && each.servedRevisionId !== null);
    const waiting = Rollout.needed(target.installation, revision, description)
        .filter(
            (entry) =>
                !own.has(entry.package.id) &&
                !rows.some((row) => row.packageId === entry.package.id && isServed(row)),
        )
        .map((entry) => entry.package.name);

    // and for each type of another package in its space grant that the space does not serve yet
    const granted = Rollout.permissions(target.installation, description, "space").space ?? [];
    const unknown = granted
        .filter((permission) => permission.packageId !== target.installation.packageId)
        .filter((permission) => call.authorizer.find(permission) === undefined)
        .map((permission) => `${permission.packageId}/${permission.type}`);
    const unserved = [...new Set([...waiting, ...unknown])];

    return unserved.length === 0
        ? undefined
        : `waiting for the space to serve ${unserved.join(", ")}`;
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

/** Describe a declaration as a need. */
function describeNeed(
    build: BuildReader,
    declaration: Awaited<ReturnType<BuildReader["declarations"]>>[number],
): ResourceNeed {
    // split a resource's description into its kind and specification
    const owner = PackageManifest.package(build.manifest);
    if (declaration.provisioned === true) {
        const { name, spec } = ResourceDescription.catchall(schema.json()).parse(
            declaration.description,
        );

        return {
            package: owner,
            name,
            kind: declaration.kind,
            kindPackageId: declaration.kindPackageId,
            spec,
        };
    }

    // take another kind's description as its specification
    const { name, ...spec } = OtherDescription.parse(declaration.description);

    return {
        package: owner,
        name,
        kind: declaration.kind,
        kindPackageId: declaration.kindPackageId,
        spec,
    };
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
