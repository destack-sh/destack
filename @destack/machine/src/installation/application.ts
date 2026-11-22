import {
    installation,
    InstallationWait,
    Build,
    Deployment,
    deployment,
    InstallationRevision,
    installationRevision,
    type RevisionDescription,
    instance,
    Installation,
    type BuildSource,
    type ResourceNeed,
    type StackEvaluation,
    type DeploymentRuntime,
    PROCESS_RUNTIME,
} from "@destack/stack/object";
import type { RevisionOptions } from "./target.ts";
import { account, Entitlement, space } from "@destack/account/object";
import {
    ObjectDescription,
    ObjectType,
    ObjectTypePool,
    Observation,
    SYNCED,
    SYNCED_CONDITION,
} from "@destack/object";
import { KNOWN_TYPES } from "@destack/stack/stack";
import { Setting } from "@destack/setting";
import { setting, SettingValue } from "@destack/setting/object";
import { DeclarationReference } from "@destack/package/declare";
import { AccessContext, Caller, principal } from "@destack/access";
import { type BlobStore, type DatabaseConnection, DatabaseKind, Snapshot } from "@destack/db";
import {
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
import { type BuildReader, BuildImage, PackageManifest } from "@destack/package/manifest";
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
    Digest,
    type DigestCrypto,
    type Identifier,
    present,
    schema,
    Version,
    type VersionRange,
} from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import type { FamilyError } from "@destack/error";
import { StackError, type StackReason } from "@destack/stack/error";
import { ResourceError, UnreachablePlan } from "@destack/resource/error";
import type { Runtime } from "../runtime/runtime.ts";
import type { Binder } from "../resource/binding.ts";
import { Approval } from "./approval.ts";
import { HOOK_WORKLOAD } from "@destack/stack/workload";
import { BOOTSTRAP_PACKAGES } from "@destack/stack/declare";
import { InstallationTarget } from "./target.ts";
import { InstallationAccess } from "./access.ts";

/** The condition of an installation whose bound targets are each ready. */
const BOUND = { status: "true", reason: "Bound", message: "every bound target is ready" } as const;

/** The description of a declaration of a kind other than resource: its name and specification. */
const OtherDescription = schema.object({ name: DeclarationName }).catchall(schema.json());

/** What a machine deploys applications with: its binder, its builds and its server runtimes. */
export interface ApplicationOptions {
    /** The kinds of objects bindings target. */
    readonly binder: Binder;
    /** The space whose builds are read, through to its machine's builds and registries. */
    readonly space: RevisionOptions["space"];
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
    /** The images of the releases the machine runs in its process: those it ships or links. */
    readonly trusted: ReadonlySet<Digest>;
    /** The space's blobs, storing the states its revisions require. */
    readonly blobs: BlobStore;
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
    /** The application's installation. */
    readonly #target: Installation;
    /** What the machine follows releases and deploys with. */
    readonly #options: ApplicationOptions;

    /** Reconcile or deploy an application installation with the machine's options. */
    constructor(target: Installation, options: ApplicationOptions) {
        this.#target = target;
        this.#options = options;
    }

    /** Run a step of the installation's reconcile in a span of its space and installation. */
    #span<Result>(step: string, work: () => Promise<Result>): Promise<Result> {
        return this.#options.space.machine.scope.span(
            `destack.installation.${step}`,
            InstallationTarget.attributes(this.#target),
            work,
        );
    }

    /** Target the build the installation runs, establish its types and deploy its revision. */
    async reconcile(call: MethodContextOf<typeof installation, "reconcile">): Promise<void> {
        // target the revision of the requested build, else of what it follows
        const target = this.#target;
        const isMoved = await this.#span("target", () => this.target(call, call.input.build));

        // establish the types its revision declares, before a server of the space serves them
        await this.#span("establish", () =>
            ApplicationInstallation.establish(call, [target.id], this.#options),
        );

        // write the states the space's revisions require once they moved, and reconcile its own storage
        const scope = space.identifier(target.scope);
        if (isMoved) {
            await this.#span("specify", () => this.#options.binder.specify(call, scope));
        }
        await this.#span("storage", () => this.#options.binder.reconcilePure(call, scope));

        // deploy the revision it targets now
        await this.#span("deploy", () => this.deployTargeted(call));
    }

    /** Establish the policies of the types some installations' targeted revisions declare. */
    static async establish(
        call: MethodContextOf<typeof installation, "reconcile">,
        ids: readonly Identifier<"installation">[],
        options: ApplicationOptions,
    ): Promise<void> {
        // read the ones targeting a revision the space does not serve yet
        const rows = await call
            .read(installation.table)
            .findMany({ where: { id: { in: [...ids] } } });
        const unserved = rows.filter(
            (row) =>
                row.deletedAt === null &&
                row.targetRevisionId !== null &&
                row.servedRevisionId !== row.targetRevisionId,
        );
        const targeted = unserved.map((row) => present(row.targetRevisionId, "the revision"));
        const revisions = await call
            .read(installationRevision.table)
            .findMany({ where: { id: { in: targeted } } });

        // read the object types each build on this machine declares
        const { scope } = options.space.machine;
        const described: ObjectDescription[][] = [];
        for (const row of unserved) {
            const revision = present(
                revisions.find((each) => each.id === row.targetRevisionId),
                `the revision of ${row.name}`,
            );
            const built = await Build.require(
                { read: (table) => call.read(table) },
                revision.scope,
                revision.buildId,
            );
            const { image } = built;
            if ((await options.space.builds.storeOf(image)) !== undefined) {
                const reader = await options.space.root(row.packageId, image);
                described.push(
                    await scope.span("destack.installation.establish.read", {}, () =>
                        ObjectDescription.read(reader),
                    ),
                );
            }
        }

        // declare each package's types once the types they refer to are known, leaving the others to wait
        const { store } = call.execution;
        const pool = ObjectTypePool.of(ObjectType.known(store.objects, KNOWN_TYPES));
        const types: ObjectType[] = [];
        for (let pending = described; pending.length > 0;) {
            const ready = pending.filter((each) => pool.missing(each).length === 0);
            if (ready.length === 0) {
                break;
            }
            types.push(...ready.flatMap((each) => pool.declare(each)));
            pending = pending.filter((each) => !ready.includes(each));
        }
        await scope.span("destack.installation.establish.compile", {}, async () => {
            store.establish(types.map((type) => type.policy));
        });
    }

    /** Target the revision of the build it runs, answering whether its states moved. */
    async target(
        call: MethodContextOf<typeof installation, "reconcile">,
        requested: Identifier<"build"> | undefined,
    ): Promise<boolean> {
        // choose the build a live installation runs: the requested one, else what it follows
        const target = this.#target;
        const reader: Pick<Snapshot, "read"> = { read: (table) => call.read(table) };
        const chosen =
            target.deletedAt === null
                ? await this.#span("choose", () =>
                      InstallationTarget.choose(reader, target, requested, this.#options),
                  )
                : undefined;

        // refuse a requested build the installation cannot run
        if (chosen !== undefined && requested !== undefined) {
            await this.requireDeployable(call, chosen.image);
        }

        // leave a targeted revision of the chosen build whose bindings still stand
        const targeted =
            target.targetRevisionId === null
                ? undefined
                : await call
                      .read(installationRevision)
                      .findFirst({ where: { id: target.targetRevisionId } });
        const isCurrent =
            chosen !== undefined &&
            targeted !== undefined &&
            targeted.buildId === chosen.build &&
            (await this.#span("current", async () =>
                this.#options.binder.isCurrent(
                    reader,
                    target,
                    targeted,
                    await this.#options.space.root(target.packageId, chosen.image),
                ),
            ));

        // record a chosen release as the space's build, revise the installation at it and target the revision
        let isRetargeted = target.deletedAt !== null;
        if (chosen !== undefined && !isCurrent) {
            const built = chosen.build ?? (await this.record(call, chosen.source)).id;
            const revision = await this.#span("revise", () =>
                this.revise(call, {
                    build: built,
                    dependencies: chosen.dependencies,
                    states: chosen.states,
                    release: chosen.release,
                    declarations: chosen.declarations,
                }),
            );
            if (revision.id !== target.targetRevisionId) {
                isRetargeted = true;
                await call.mutate(call.self, [
                    Observation.call(installation, target, {
                        conditions: {},
                        fields: { targetRevisionId: revision.id },
                    }),
                ]);
            }
        }

        return isRetargeted;
    }

    /** Deploy the revision the installation targets now, read as the call left it. */
    async deployTargeted(call: MethodContextOf<typeof installation, "reconcile">): Promise<void> {
        const current = await call
            .read(installation.table)
            .findFirst({ where: { id: this.#target.id } });
        const applying = new ApplicationInstallation(
            present(current, `the installation ${this.#target.id}`),
            this.#options,
        );
        await applying.#deployRevision(call);
    }

    /** Refuse a build of another package, or one whose required space settings are unset. */
    async requireDeployable(
        call: MethodContextOf<typeof installation, "reconcile">,
        digest: Digest,
    ): Promise<void> {
        // require a build of the installation's package
        const target = this.#target;
        const image = await this.#options.space.builds.image(digest);
        const reader = await this.#options.space.builds.contents(image.root);
        const built = PackageManifest.package(reader.manifest);
        if (built.id !== target.packageId) {
            throw StackError.of(
                "PACKAGE_INVALID",
                `build is invalid: it is of '${built.name}', which '${target.name}' does not install`,
            );
        }

        // require each space setting without a default it declares
        const placed = { scope: target.scope, installation: target.id };
        for (const declared of await Setting.list(reader)) {
            const { definition } = declared;
            if (definition.scope === "space" && "required" in definition) {
                await SettingValue.resolve(call, declared, placed);
            }
        }
    }

    /** Record a chosen release as the space's build. */
    async record(
        call: MethodContextOf<typeof installation, "reconcile">,
        source: BuildSource,
    ): Promise<{ readonly id: Identifier<"build"> }> {
        // refuse a source the space builds itself, which a chosen build always names
        if (source.kind !== "release") {
            throw new TypeError(`a ${source.kind} source has a build of the space`);
        }
        const reader: Pick<Snapshot, "read"> = { read: (table) => call.read(table) };

        return Build.release(
            reader,
            (calls) => call.mutate(call.self, calls),
            Installation.space(this.#target),
            source,
        );
    }

    /** Relate the installation at the universe as its space, through the uplink. */
    async relate(reconciliation: ObjectReconciliation): Promise<void> {
        // read the installation as it stands, and the build of the revision it applied
        const target = this.#target;
        const snapshot = Snapshot.live(reconciliation.database);
        const deployed = await snapshot
            .read(installation.table)
            .findFirst({ where: { id: target.id } });
        const { trusted } = this.#options;
        const built =
            deployed?.appliedRevisionId === null || deployed === undefined
                ? undefined
                : await InstallationRevision.build(snapshot, deployed.appliedRevisionId);

        // keep the relations of a superseded host installation until its stack revises it
        if (built !== undefined && Installation.isSuperseded(target, built.image, trusted)) {
            return;
        }
        const { packageId } = target;
        const build =
            built === undefined
                ? undefined
                : await this.#options.space.root(packageId, built.image);
        const description = build === undefined ? undefined : await PackageDescription.of(build);
        const set = Installation.permissions(deployed ?? target, description, "space");

        // relate it as its space, once its relations moved
        const version =
            build === undefined ? undefined : PackageManifest.package(build.manifest).version;
        const calls = await InstallationAccess.relate(
            snapshot,
            target,
            deployed ?? target,
            set,
            version,
        );
        if (calls.length > 0) {
            const delegator = principal.space.reference(Scope.universe.id, target.scope);
            const lending = Caller.acting(delegator, reconciliation.caller.subject);
            await reconciliation.store.mutate(AccessContext.of(lending, reconciliation.now), {
                id: RequestId.create(reconciliation.store.environment),
                calls,
            });
        }
    }

    /** Revise the installation: a build at its bindings, retaining each revision once. */
    async revise(
        call: MethodContext,
        input: RevisionDescription & {
            readonly build: Identifier<"build">;
            readonly evaluation?: StackEvaluation | undefined;
        },
    ): Promise<InstallationRevision> {
        // bind an application's needs, creating and following the installation's own resources
        const target = this.#target;
        const built = await Build.require(
            { read: (table) => call.read(table) },
            Installation.space(target),
            input.build,
        );
        const reader = await this.#options.space.root(target.packageId, built.image);
        const description = await PackageDescription.of(reader);
        const { stranded } = this.#selectWorkloads(built.image, reader, description);
        const isRefused =
            this.#describeRefusal(built.image, reader, stranded, description) !== undefined ||
            (await this.#describeDatabases(reader)) !== undefined;

        // bind a stack's nothing, and a refused build's selected targets alone, as it creates nothing
        const { bindings: selected } = target;
        const declared = await this.#readNeeds(reader, []);
        const unprovided = this.#options.binder.unprovided(declared);
        const needs = declared.filter(
            (need) =>
                !this.#isUnaddressed(need) &&
                !unprovided.includes(need.kind) &&
                (!isRefused || selected[need.package.id]?.[need.name] !== undefined),
        );
        const bindings =
            target.export !== null
                ? []
                : await this.#options.binder.bind(call, target, needs, await this.#running(call));

        // retain the revision once per digest
        const [revision] = await call.mutate(call.self, [
            await InstallationTarget.create(
                target,
                { id: built.id, image: built.image },
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
            Installation.space(this.#target),
            revision.buildId,
        );

        const { packageId } = this.#target;

        return this.#readNeeds(await this.#options.space.root(packageId, built.image), []);
    }

    /** Read the needs of the revisions the installation's live deployments still run. */
    async #running(call: MethodContext): Promise<ResourceNeed[]> {
        // read the revisions the live deployments run
        const target = this.#target;
        const live = await call.read(deployment).findMany({
            where: { ...Deployment.live(), installationId: target.id },
        });
        const running = await call.read(installationRevision).findMany({
            where: { id: { in: [...new Set(live.map((entry) => entry.revisionId))] } },
        });

        // read each one's needs from its build
        const needs = await Promise.all(running.map((revision) => this.#needs(call, revision)));

        return needs.flat();
    }

    /** Deploy the revision of an enabled application, and drain a suspended one. */
    async #deployRevision(call: MethodContextOf<typeof installation, "reconcile">): Promise<void> {
        // read the revision it targets and its live deployments
        const target = this.#target;
        const revision =
            target.targetRevisionId === null
                ? undefined
                : await call
                      .read(installationRevision)
                      .findFirst({ where: { id: target.targetRevisionId } });
        const live = await call.read(deployment).findMany({
            where: {
                ...Deployment.live(),
                installationId: target.id,
            },
        });

        // retire a deleted application's declarations, keeping it running until its own code finalized its records
        const isDeleted = target.deletedAt !== null;
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
        if (isDeleted || target.status !== "enabled" || revision === undefined) {
            const active = live.filter((entry) => entry.status === "active");
            await call.mutate(
                call.self,
                active.map((entry) => Deployment.drain(entry)),
            );

            // report an unpublished followed release
            const kind = InstallationTarget.follows(target);
            const isEnabled = !isDeleted && target.status === "enabled";
            if (isEnabled && revision === undefined && (kind === "release" || kind === "tag")) {
                const message = `${target.name} has no published release for what it follows`;
                await call.observe(installation, target, {
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
                    installation.call("finalize", installation.target(target)),
                ]);

                return;
            }
        }
        // deploy the revision until each workload runs from the output the machine chooses
        else {
            await this.#deploy(call, revision, live);
        }

        // relate the application as consumer of exactly what its live deployments' revisions bind
        await this.#span("bind", () => this.#relate(call));
    }

    /** Release a drained application's resources, delete its deployments and drop its grants. */
    async #finalize(call: MethodContextOf<typeof installation, "reconcile">): Promise<void> {
        // retire every record it declares, and let go of the ones their types keep, such as retained resources
        const target = this.#target;
        await this.#options.binder.retire(call, target, [], []);
        await this.#options.binder.disown(call, target);

        // delete its retired deployments with their instances
        const retired = await call
            .read(deployment)
            .findMany({ where: { installationId: target.id } });
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
    async #relate(call: MethodContextOf<typeof installation, "reconcile">): Promise<void> {
        // relate it as consumer of what its live deployments' revisions bind
        const target = this.#target;
        const digest = await this.#options.binder.relate(call, target, target.bound);

        // record what it related once that changed
        if (digest !== target.bound) {
            await call.mutate(call.self, [
                Observation.call(installation, target, {
                    conditions: {},
                    fields: { bound: digest },
                }),
            ]);
        }
    }

    /** Deploy each new workload output of a revision's build and drain the replaced deployments. */
    async #deploy(
        call: MethodContextOf<typeof installation, "reconcile">,
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
        if (Installation.isSuperseded(target, built.image, this.#options.trusted)) {
            const message = `waiting for the space's stack to move it to a release this machine runs`;
            await call.observe(installation, target, {
                conditions: {
                    Ready: Installation.waiting("WaitingForStack", message),
                },
                fields: InstallationWait.none,
            });

            return;
        }

        // wait for a build this machine lacks, which a pull or its stack brings, as an image pull waits
        if ((await this.#options.space.builds.storeOf(built.image)) === undefined) {
            await call.observe(installation, target, {
                conditions: { Ready: Installation.waitingForBuild(built.image) },
                fields: InstallationWait.none,
            });

            return;
        }

        // read the workloads the machine runs
        const build = await this.#span("open", () => this.#open(call, revision));
        const description = await this.#span("describe", () => PackageDescription.of(build));

        // select the workloads the machine runs
        const { workloads, stranded } = this.#selectWorkloads(built.image, build, description);

        // refuse an unrunnable workload, an undeclared or unentitled capability or directory, or a database
        const refusal = await this.#span(
            "refuse",
            async () =>
                this.#describeRefusal(built.image, build, stranded, description) ??
                (await this.#describeOutdated(built.image)) ??
                (await this.#describeUnentitled(call, description)) ??
                (await this.#describeUninstalled(call, revision, description)) ??
                (await this.#describeDatabases(build)),
        );
        if (refusal !== undefined) {
            throw refusal;
        }

        // stop for a broken upgrade chain or an approval of the upgrade from the applied release
        if (
            await this.#span("upgrade", () =>
                this.#holdForUpgrade(call, revision, build, built.image),
            )
        ) {
            return;
        }

        // split the workloads into fresh and replaced ones, and upsert only the access when nothing changes
        const { fresh, replaced } = await this.#span("partition", () =>
            this.#partition(call, revision, workloads, live),
        );
        if (
            target.appliedRevisionId === revision.id &&
            fresh.length === 0 &&
            replaced.length === 0
        ) {
            // leave a revision the installation observed deployed
            const ready = target.conditions["Ready"];
            if (Observation.isCurrent(target, "Ready") && ready?.reason === "Deployed") {
                return;
            }

            // grant again once the space serves what its grant names
            if (await this.#waitForSpace(call, description, revision)) {
                return;
            }
            await InstallationAccess.upsert(call, target, description);
            await this.#redeclare(call, revision);
            await this.#observeApplied(call, workloads.length);

            return;
        }

        // refuse installation needs without a bound address
        const needs = await this.#span("needs", () => this.#readNeeds(build, workloads));
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
        call: MethodContextOf<typeof installation, "reconcile">,
        revision: InstallationRevision,
        plan: DeploymentPlan,
    ): Promise<void> {
        // wait for a provider of each kind no provider here serves yet
        const target = this.#target;
        const { build, workloads, fresh, replaced, needs, description } = plan;
        const unprovided = this.#options.binder.unprovided(needs);
        if (unprovided.length > 0) {
            // wait for the space to serve another installation, as one brings the provider
            const message = `waiting for a provider of ${unprovided.join(", ")} resources`;
            await call.observe(installation, target, {
                conditions: {
                    Bound: Installation.waiting("WaitingForResources", message),
                    Ready: Installation.waiting("WaitingForResources", message),
                },
                fields: InstallationWait.of(installation, { scope: target.scope }),
            });

            return;
        }

        // wait for the resources its revision binds
        const bindings = revision.bindings;
        const unready = await this.#span("ready", () =>
            this.#options.binder.ready(call, target, bindings, needs),
        );
        if (unready !== undefined) {
            const message = `waiting for ${unready.object.name} ${unready.id} to be ready`;
            await call.observe(installation, target, {
                conditions: {
                    Bound: Installation.waiting("WaitingForResources", message),
                    Ready: Installation.waiting("WaitingForResources", message),
                },
                fields: InstallationWait.of(unready.object, { id: unready.id }),
            });

            return;
        }

        // wait for the space settings its servers read at start, which its stack may set after installing it
        const placed = (await Setting.list(build)).filter(
            (declared) => declared.definition.scope === "space",
        );
        const unset = await SettingValue.missing({ read: (table) => call.read(table) }, placed, {
            scope: target.scope,
            installation: target.id,
        });
        if (unset.length > 0) {
            const keys = unset.map((declared) => DeclarationReference.key(declared.reference));
            const message = `waiting for settings ${keys.join(", ")}`;
            await call.observe(installation, target, {
                conditions: {
                    Bound: Installation.waiting("WaitingForSettings", message),
                    Ready: Installation.waiting("WaitingForSettings", message),
                },
                fields: InstallationWait.of(setting, { scope: target.scope }),
            });

            return;
        }

        // wait for the space to serve the packages whose types its grant has
        if (await this.#waitForSpace(call, description, revision)) {
            return;
        }

        // grant the installation exactly what its package and its enabled capabilities request
        await this.#span("grant", () => InstallationAccess.upsert(call, target, description));

        // drain the replaced deployments down to one active deployment per workload
        await call.mutate(
            call.self,
            replaced.map((entry) => Deployment.drain(entry)),
        );

        // activate each fresh workload's deployment
        const releases = {
            served: await servedRelease(build),
        };
        await this.#span("activate", async () => {
            for (const workload of fresh) {
                await this.#activate(call, revision, workload, releases);
            }
        });

        // declare again what the revision and the live deployments need, retiring what only drained ones did
        await this.#span("declare", () => this.#redeclare(call, revision));

        // observe the revision applied
        await this.#observeApplied(call, workloads.length);
        await call.mutate(call.self, [
            Observation.call(installation, target, {
                conditions: {},
                fields: { appliedRevisionId: revision.id },
            }),
        ]);
    }

    /** Observe the applied revision deployed at the installation's generation. */
    async #observeApplied(
        call: MethodContextOf<typeof installation, "reconcile">,
        workloads: number,
    ): Promise<void> {
        // record it synced once it runs what it follows, unless its residency relates it after the call
        const reader: Pick<Snapshot, "read"> = { read: (table) => call.read(table) };
        const homed = await InstallationAccess.home(reader, this.#target);
        const isSynced =
            homed === undefined &&
            (await InstallationTarget.isFollowed(reader, this.#target, this.#options));

        // observe it deployed, bound, and waiting for nothing
        await call.observe(installation, this.#target, {
            conditions: {
                Bound: BOUND,
                Ready: {
                    status: "true",
                    reason: "Deployed",
                    message: `${workloads} workloads deployed`,
                },
                ...(isSynced ? { [SYNCED]: SYNCED_CONDITION } : {}),
            },
            fields: { plan: null, approvedPlan: null, ...InstallationWait.none },
        });
    }

    /** Describe an image's build requiring a newer release of a builtin than the machine links. */
    async #describeOutdated(image: Digest): Promise<Refusal | undefined> {
        // index the version of each builtin the machine links by package name
        const { linked } = this.#options;
        const builtins = new Map(
            [...linked.values()].map((release) => [release.package.name, release.package.version]),
        );

        // refuse the first build requiring a release of a builtin newer than the linked one
        for (const {
            package: requiring,
            requires,
        } of await this.#options.space.builds.requirements(image)) {
            for (const [name, required] of Object.entries(requires)) {
                const running = builtins.get(name);
                const isVersion = Version.safeParse(required).success;
                if (
                    running !== undefined &&
                    isVersion &&
                    Version.compare(Version.release(running), Version.release(required)) < 0
                ) {
                    const { name: owner, version } = requiring;
                    const message = `cannot deploy '${owner}@${version}': it needs ${name} ${Version.release(required)} or newer, and this machine runs ${running}`;

                    return StackError.of("RELEASE_BLOCKED", message);
                }
            }
        }

        return undefined;
    }

    /** Describe why a build cannot deploy. */
    #describeRefusal(
        image: Digest,
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
        else if (this.#target.isolation === "host" && !this.#options.trusted.has(image)) {
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
        const isHosted = this.#isStored(image);
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
        call: MethodContextOf<typeof installation, "reconcile">,
        description: PackageDescription | undefined,
    ): Promise<Refusal | undefined> {
        // check nothing on a machine enforcing no entitlements
        if (!this.#options.isEnforcingEntitlements) {
            return undefined;
        }

        // list the features the enabled capabilities need
        const enabled = Installation.enabled(this.#target, description);
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
        call: MethodContextOf<typeof installation, "reconcile">,
        revision: InstallationRevision,
        description: PackageDescription | undefined,
    ): Promise<Refusal | undefined> {
        // read the space's own packages and the installations of the packages it needs, no others
        const target = this.#target;
        const needed = Installation.needed(target, revision, description);
        const rows =
            needed.length === 0
                ? []
                : await call.read(installation).findMany({
                      where: {
                          scope: target.scope,
                          packageId: { in: needed.map((entry) => entry.package.id) },
                      },
                  });
        const installed = new Set([
            ...Object.values(BOOTSTRAP_PACKAGES).map((handle) => handle.id),
            ...rows.map((row) => row.packageId),
        ]);

        // refuse the first needed package missing, naming the capability needing it
        const missing = needed.find((entry) => !installed.has(entry.package.id));
        if (missing === undefined) {
            return undefined;
        }
        const { name } = missing.package;

        return StackError.of(
            "DEPENDENCY_BLOCKED",
            missing.capability === undefined
                ? `${target.name} needs ${name} installed`
                : `capability ${missing.capability} needs ${name} installed`,
        );
    }

    /** Wait while the space does not serve a package it needs or a type it is granted. */
    async #waitForSpace(
        call: MethodContextOf<typeof installation, "reconcile">,
        description: PackageDescription | undefined,
        revision: InstallationRevision,
    ): Promise<boolean> {
        // report the wait on its bound targets' readiness
        const unserved = await describeUngranted(call, this.#target, revision, description);
        if (unserved !== undefined) {
            await call.observe(installation, this.#target, {
                conditions: {
                    Bound: BOUND,
                    Ready: Installation.waiting("WaitingForSpace", unserved),
                },
                fields: InstallationWait.of(installation, { scope: this.#target.scope }),
            });
        }

        return unserved !== undefined;
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
        const message = `${this.#target.packageId} declares the databases ${names}: the space's main database keeps every installation's tables`;

        return StackError.of("DATABASE_BLOCKED", message);
    }

    /** Stop the deploy for an unapproved upgrade or access growth, and report the stop. */
    async #holdForUpgrade(
        call: MethodContextOf<typeof installation, "reconcile">,
        revision: InstallationRevision,
        build: BuildReader,
        image: Digest,
    ): Promise<boolean> {
        // pass a first deploy and a redeploy of the applied revision
        const target = this.#target;
        if (target.appliedRevisionId === null || target.appliedRevisionId === revision.id) {
            return false;
        }

        // plan the upgrade from the applied revision, with the access the build asks beyond it
        const applied = await call
            .read(installationRevision)
            .findFirst({ where: { id: target.appliedRevisionId } });
        const plan = await this.#planUpgrade(
            present(applied, `revision ${target.appliedRevisionId}`),
            build,
            image,
            call.environment.crypto,
        );

        // observe the upgrade waiting for approval
        const waiting = await Approval.waiting(call, target, plan, call.environment.crypto);
        if (waiting !== undefined) {
            await call.observe(installation, target, waiting);
        }

        return waiting !== undefined;
    }

    /** Split a revision's workloads into fresh ones and the active deployments they replace. */
    async #partition(
        call: MethodContextOf<typeof installation, "reconcile">,
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
            this.#target.bindings[need.package.id]?.[need.name] === undefined
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
        call: MethodContextOf<typeof installation, "reconcile">,
        revision: InstallationRevision,
        workload: BuildWorkload,
        releases: Releases,
    ): Promise<void> {
        await call.mutate(call.self, [
            deployment.call("create", {
                installationId: this.#target.id,
                packageId: this.#target.packageId,
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
        call: MethodContextOf<typeof installation, "reconcile">,
        revision: Pick<InstallationRevision, "id">,
    ): Promise<BuildReader> {
        const { packageId } = this.#target;

        return InstallationRevision.reader(call, revision.id, packageId, this.#options.space);
    }

    /** Plan the upgrade steps from an applied revision to the next build, kind by kind. */
    async #planUpgrade(
        previous: Pick<InstallationRevision, "release" | "declarations">,
        next: BuildReader,
        image: Digest,
        crypto: DigestCrypto,
    ): Promise<Plan> {
        // skip rollbacks, and rebuilds of the applied release declaring what it declared
        const from = previous.release;
        const release = next.manifest.package.version;
        const order = Version.compare(release, from);
        const isRebuilt =
            order === 0 &&
            (await Digest.json(await next.declarations(), crypto)) === previous.declarations;
        if (order < 0 || isRebuilt) {
            return { steps: [] };
        }

        // compare each kept declaration its kind compares, then plan the rest from the applied release
        const declarations = await this.#options.blobs.readJson(
            previous.declarations,
            schema.array(graph.Declaration),
        );
        const history = {
            release: from,
            declarations,
            vocabulary: Vocabulary.advance({}, declarations, from),
        };
        const { name } = this.#target;
        const owner = PackageManifest.package(next.manifest);
        const opened = await this.#options.space.builds.image(image);
        let upgrade: Upgrade;
        try {
            const compared = await this.#compare(declarations, from, next, opened);
            upgrade = Upgrade.plan(history, owner, await next.declarations(), compared);
        } catch (cause) {
            // tell the caller why no plan reaches the release, whose author changes its declarations
            if (!ResourceError.is(cause, "PLAN_BLOCKED")) {
                throw cause;
            }
            const detail = UnreachablePlan.parse(cause.data)
                .problems.map((problem) => `${problem.target}: ${problem.detail}`)
                .join("; ");
            throw StackError.of(
                "PLAN_BLOCKED",
                `cannot upgrade installation '${name}' to '${release}': ${detail}`,
                { cause },
            );
        }

        // address each step below the installation's release
        const prefix = Address.join("installation", name, "release", release);
        const steps: Step[] = upgrade.steps.map((step) => ({
            ...step,
            target: Address.join(prefix, step.target),
        }));

        return { steps };
    }

    /** Compare the declarations a next build keeps by their kinds' compare, by address. */
    async #compare(
        declarations: readonly graph.Declaration[],
        from: string,
        next: BuildReader,
        image: BuildImage,
    ): Promise<ReadonlyMap<string, Plan>> {
        // pair the applied declarations with the next build's by kind and name
        const kept = new Map(
            declarations.map((each) => [Address.join(each.kind, each.name), each] as const),
        );
        const nextDeclarations = await next.declarations();

        // read the capabilities the installation enables at each release
        const installed = this.#target;
        const enabledFrom = Installation.enabled(installed, PackageDescription.find(declarations));
        const enabledTo = Installation.enabled(
            installed,
            PackageDescription.find(nextDeclarations),
        );
        // find each kind's package in the next build's image
        const entries = await BuildImage.entries(image, (digest) =>
            this.#options.space.builds.get(digest),
        );
        const owner = PackageManifest.package(next.manifest);
        const [runtime] = this.#options.runtimes;
        const compared = new Map<string, Plan>();
        const isEnabledSame = canonicalize(enabledFrom) === canonicalize(enabledTo);
        for (const after of nextDeclarations) {
            // compare a kept declaration whose description or enabled capabilities changed
            const address = Address.join(after.kind, after.name);
            const before = kept.get(address);
            const isSame =
                before !== undefined &&
                isEnabledSame &&
                canonicalize(before.description) === canonicalize(after.description);
            if (before === undefined || isSame) {
                continue;
            }

            // find the kind's compare function in its package's build
            const opened =
                after.kindPackageId === owner.id
                    ? await this.#options.space.machine.builds.putImage(image)
                    : await this.#kindImage(image, entries, after.kindPackageId);
            const kind = await this.#options.space.root(after.kindPackageId, opened);
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
            const entry = (
                declaration: graph.Declaration,
                version: string,
                capabilities: readonly string[],
            ) => ({
                description: declaration.description,
                symbol: { package: { ...owner, version } },
                capabilities,
            });
            const plan = await machine.call(opened, exported.path, reference.name, [
                entry(before, from, enabledFrom),
                entry(after, owner.version, enabledTo),
            ]);
            compared.set(address, Plan.parse(plan));
        }

        return compared;
    }

    /** Read the image of a package defining a kind: in the next build's, else the linked one. */
    async #kindImage(
        image: BuildImage,
        entries: Readonly<Record<string, DependencyRelease>>,
        packageId: PackageId,
    ): Promise<Digest> {
        // take the builtin the machine links, as every build resolves a linked package to it
        const { linked } = this.#options;
        const builtin = linked.get(packageId);
        if (builtin !== undefined) {
            await this.#options.space.pull(packageId, builtin.image);

            return builtin.image;
        }

        // derive the package's image within the next build's, stored where the machine reads it
        const entry = present(
            Object.values(entries).find(
                (each) => each.kind === "destack" && each.package.id === packageId,
            ),
            `the build of the kind package ${packageId}`,
        );

        return await this.#options.space.machine.builds.putImage(
            BuildImage.at(image, entry.manifest),
        );
    }

    /** Decide whether a build runs in the store's process: linked, or a host installation's. */
    #isStored(image: Digest): boolean {
        const installed = this.#target;

        return (
            this.#options.linked.get(installed.packageId)?.image === image ||
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
            enabled: Installation.enabled(this.#target, description),
        };
    }

    /** Select a build's workloads: one in the store for a linked or process build, else its own. */
    #selectWorkloads(
        image: Digest,
        build: BuildReader,
        description: PackageDescription | undefined,
    ): {
        readonly workloads: BuildWorkload[];
        readonly stranded: string[];
    } {
        // serve a linked release or a host installation's build in the store
        if (this.#isStored(image)) {
            return { workloads: [this.#processWorkload(description)], stranded: [] };
        }

        // list the workloads of the emitted server outputs, each sandbox joined with what the enabled capabilities unlock
        const target = this.#target;
        const enabled = Installation.enabled(target, description);
        const unlocked = Installation.sandbox(target, description);
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
                          directories: sandbox.files === undefined ? [] : target.directories,
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
        const release = `${target.name} ${build.manifest.package.version}`;
        const declared = Object.keys(description?.capabilities ?? {});
        const undeclared = Installation.capabilities(target).filter(
            (name) => !declared.includes(name),
        );
        if (undeclared.length > 0) {
            return `${release} declares no capabilities ${undeclared.join(", ")} to allow`;
        }

        // read the directory access of the workloads' sandboxes joined with what the enabled capabilities unlock
        const unlocked = Installation.sandbox(target, description);
        const access = Object.values(build.manifest.outputs).flatMap((output) =>
            Object.values(output.workloads).flatMap((workload) => {
                const { files } = Sandbox.merge(workload.sandbox, unlocked);

                return files === undefined ? [] : [files.access];
            }),
        );
        const written = target.directories.filter((grant) => grant.access === "write");
        const reaching = target.directories.filter((grant) =>
            DirectoryGrant.isReachingSockets(grant),
        );
        // refuse granting directories without fs
        if (target.directories.length > 0 && access.length === 0) {
            return `${release} enables no fs ability to grant directories ${joinPaths(target.directories)}`;
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
        const packageId = this.#target.packageId;
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
    call: Pick<MethodContextOf<typeof installation, "reconcile">, "read" | "authorizer">,
    target: Installation,
    revision: InstallationRevision,
    description: PackageDescription | undefined,
): Promise<string | undefined> {
    // wait for each package it needs that is neither the space's own nor installed at a build
    const own = new Set(Object.values(BOOTSTRAP_PACKAGES).map((handle) => handle.id));
    const needed = Installation.needed(target, revision, description);
    const rows =
        needed.length === 0
            ? []
            : await call.read(installation).findMany({
                  where: {
                      scope: target.scope,
                      packageId: { in: needed.map((entry) => entry.package.id) },
                  },
              });
    const isResolved = (row: (typeof rows)[number]) =>
        row.targetRevisionId !== null || InstallationTarget.isPinned(row);
    const waiting = needed
        .filter(
            (entry) =>
                !own.has(entry.package.id) &&
                !rows.some((row) => row.packageId === entry.package.id && isResolved(row)),
        )
        .map((entry) => entry.package.name);

    // and for each type of another package in its space grant whose policy the space has not established
    const granted = Installation.permissions(target, description, "space").space ?? [];
    const unknown = granted
        .filter((permission) => permission.packageId !== target.packageId)
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
    // read the comparison the package defines for the kind, from its constructors or its kinds
    const definition = PackageDefinition.parse(
        JSON.parse(new TextDecoder().decode(await build.load("destack.json"))),
    );
    const reference = PackageDefinition.comparisons(definition).find(
        (each) => each.kind === kind,
    )?.compare;
    if (reference === undefined) {
        return undefined;
    }

    // split the reference into its export subpath and the function's name
    const [subpath = "", name = ""] = reference.split("#");

    return { subpath, name };
}
