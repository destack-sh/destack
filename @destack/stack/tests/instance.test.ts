import { RESOURCE_KIND } from "@destack/resource";
import { defineSetting } from "@destack/setting/declare";
import { describeSetting } from "@destack/setting/inspect";
import settingPackage from "@destack/setting/package";
import { testHost } from "@destack/test/host";
import type { Crypto } from "@destack/environment";
import objectHandle from "@destack/object/package";
import {
    approval,
    type BindingTarget,
    changeRequest,
    defineObject,
    field,
    type Field,
    type ObjectDefinition,
    ObjectDescription,
    InstallationBindingSpec,
    installation,
    Observation,
} from "@destack/object";
import { MemoryBuild } from "@destack/package/test";
import { Plan } from "@destack/resource";
import { ObjectReference, Scope, RequestId } from "@destack/sync";
import { expect, refusal, single, test } from "@destack/test";
import { pathToFileURL } from "node:url";
import {
    type DatabaseConnection,
    databaseProvider,
    DatabaseState,
    eq,
    Expression,
    isNotNull,
    Snapshot,
} from "@destack/db";
import { Digest, nth, Identifier, present, schema, type VersionRange } from "@destack/schema";
import { BunSqliteDatabaseHost } from "@destack/db/bun";
import {
    BROWSER_SANDBOX,
    HOST_SANDBOX,
    Sandbox,
    type DirectoryGrant,
    Package,
    PackageId,
} from "@destack/package";
import { TestEntitlement } from "@destack/account/test";
import {
    AccessContext,
    accessGrant,
    accessRole,
    accessRolePermission,
    Caller,
    type GrantRequest,
    principal,
} from "@destack/access";
import accountPackage from "@destack/account/package";
import { space } from "@destack/account/object";
import type { ServerRuntime } from "@destack/package/runtime";
import type { BuildReader } from "@destack/package/manifest";
import * as object from "../src/object/index.ts";
import * as accountObject from "@destack/account/object";
import { Approval, SpaceStore, type SpaceOptions } from "../src/server/index.ts";
import { Rollout } from "../src/object/index.ts";
import {
    readInstalled,
    refusedCondition,
    fixtureBuild,
    fixtureOutputs,
    ids,
    insertRollout,
    reconcile,
    reconcileResources,
    settle,
    TestRuntime,
    TestSpace,
    TestSpaceDatabaseHost,
    workload,
} from "../src/test/index.ts";

/** The notes package the scenarios install. */
const notesPackage = Package.parse({
    id: ids.package,
    name: "@example/notes",
    version: "2026.9.0",
});

/** The package defining object types, resource kinds and triggers. */
const objectPackage = Package.parse({
    id: objectHandle.id,
    name: "@destack/object",
    version: "2026.9.0",
});

/** The main workload of the notes build. */
const mainWorkload = { ...workload, resources: [] };

/** A package an application depends on, which its space may lack. */
const VAULT = Package.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000f2",
    name: "@acme/vault",
    version: "2026.9.0",
});

/** A declaration of the notes build, with the package defining its kind. */
interface Declared {
    /** The declaration kind, a resource when absent. */
    readonly kind?: "resource" | "package" | "trigger" | "object" | "setting";
    /** The package defining the declaration's kind. */
    readonly definer: Package;
    /** The declaration's name. */
    readonly name: string;
    /** The declaration's description, as the definer's inspector records it. */
    readonly description: Record<string, unknown>;
}

/** Declare the notes package's note type with some fields, as the object package describes it. */
function noteType(
    fields: Readonly<Record<string, Field>>,
    release: { readonly version?: string; readonly convert?: ObjectDefinition["convert"] } = {},
): Declared {
    // describe the type as the notes package's release, with the release's conversions
    const { version = notesPackage.version, convert } = release;
    const note = defineObject({
        name: "note",
        plural: "notes",
        scope: space,
        fields,
        shareable: {},
    });
    const { name, definition } = ObjectDescription.of(note);
    const description = {
        name,
        package: { ...notesPackage, version },
        definition: convert === undefined ? definition : { ...definition, convert },
    };

    return { kind: "object", definer: objectPackage, name: "note", description };
}

/** The notes package's note type, whose notes have a body. */
const main = noteType({ body: field.string().optional() });

/** The tasks binding, calling an installation of the notes package. */
const tasks: Declared = {
    definer: objectPackage,
    name: "tasks",
    description: {
        name: "tasks",
        kind: "installation",
        spec: { package: ids.package },
    },
};

/** The notes package's declaration, requesting permissions by scope. */
function requesting(permissions: GrantRequest): Declared {
    return {
        kind: "package",
        definer: objectPackage,
        name: "notes",
        description: {
            api: { name: "notes", procedures: [] },
            objects: {},
            adapters: {},
            views: [],
            permissions,
            capabilities: {},
            errors: [],
        },
    };
}

/** Write the notes build: its workloads in bun and workerd outputs, and a home view. */
async function notesBuild(
    declared: readonly Declared[],
    crypto: Crypto,
    sandbox: Sandbox = {},
    workloads: readonly string[] = ["main"],
): Promise<BuildReader> {
    // make the workload use every declared resource in both server outputs
    const used = {
        ...workload,
        sandbox: Sandbox.select(
            sandbox,
            HOST_SANDBOX.filter((name) => sandbox[name] !== undefined),
        ),
        resources: declared
            .filter((entry) => (entry.kind ?? "resource") === "resource")
            .map((entry) => ({
                packageId: ids.package,
                name: entry.name,
            })),
    };
    const browser = BROWSER_SANDBOX.filter((name) => sandbox[name] !== undefined);
    const output = (runtime: string) => ({
        runtime,
        emit: true,
        workloads: Object.fromEntries(workloads.map((name) => [name, used])),
        views:
            browser.length === 0
                ? {}
                : {
                      home: {
                          entrypoint: "view.js",
                          stylesheets: [],
                          sandbox: Sandbox.select(sandbox, browser),
                          presents: [],
                          categories: [],
                          audience: "person",
                          packages: [],
                          render: { mode: "client" },
                      },
                  },
    });

    // declare each resource as resource's kind, and each other declaration as its definer's
    const built = await MemoryBuild.of(
        notesPackage,
        declared.map((entry) => ({
            ...(entry.kind === undefined
                ? RESOURCE_KIND.identity
                : { kind: entry.kind, kindPackageId: entry.definer.id }),
            module: "src/resource.ts",
            name: entry.name,
            description: entry.description,
        })),
        crypto,
        { outputs: fixtureOutputs({ server: output("bun"), worker: output("workerd") }) },
    );

    return built;
}

/** A runtime that counts attempts of a runner that exits before serving. */
class FailingRuntime extends TestRuntime {
    /** The starts attempted. */
    attempts = 0;

    /** Fail to start. */
    override async start(): Promise<void> {
        this.attempts += 1;
        throw new Error("runner exited before serving");
    }
}

/** The checkout the notes installations follow. */
const selection = {
    kind: "checkout" as const,
    machine: ids.machine,
    checkout: schema.identifier("checkout").parse("checkout-01996ab0-0000-7000-8000-000000000008"),
    directory: "." as const,
};

/** Install notes from a checkout at one revision of its build and return the installation. */
async function installNotes(
    fixture: TestSpace,
    id: Identifier<"installation"> = ids.notes,
    build?: BuildReader,
) {
    const { database } = fixture;
    const now = fixture.machine.clock.now();
    const notes = await insertRollout(
        database,
        {
            id,
            scope: ids.space,
            packageId: ids.package,
            name: "notes",
            selection,
            installer: fixture.owner.subject,
        },
        now,
    );
    await submitRevision(fixture, id, "e", build);

    return notes;
}

/** Follow a checkout build digested from one repeated character, for its controller to revise. */
async function submitRevision(
    fixture: TestSpace,
    installationId: Identifier<"installation">,
    character: string,
    build?: BuildReader,
) {
    const { database } = fixture;
    const now = fixture.machine.clock.now();
    const reader = build ?? (await notesBuild([main], fixture.environment.crypto));
    const built = await fixture.recordBuild(reader);
    const [installed] = await database
        .select()
        .from(installation.table)
        .where(eq(installation.table.id, installationId));
    const described = await fixture.describeRevision(
        present(installed, `installation ${installationId}`),
        reader,
    );
    const { crypto } = fixture.machine.environment;
    const digest = await object.InstallationRevision.digest(
        await reader.digest(),
        undefined,
        [],
        crypto,
    );
    const revision = nth(
        await database.transaction(
            async (transaction) =>
                await transaction
                    .insert(object.installationRevision.table)
                    .values({
                        scope: ids.space,
                        createdAt: now,
                        updatedAt: now,
                        id: await object.InstallationRevision.id(installationId, digest, crypto),
                        installationId,
                        buildId: built.build,
                        ...described,
                        generation: 0,
                        bindings: [],
                        digest,
                    })
                    .returning(),
        ),
        0,
    );
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(installation.table)
                .set({ revisionId: revision.id })
                .where(eq(installation.table.id, installationId)),
    );

    return revision;
}

/** Serve a database's deployments, instances and resources through the controllers of a machine. */
async function serveControllers(
    fixture: TestSpace,
    options: {
        readonly runtimes?: readonly ServerRuntime[];
        readonly runtime?: TestRuntime;
        readonly accounts?: SpaceOptions["accounts"];
        readonly isEnforcingEntitlements?: boolean;
        readonly destack?: VersionRange;
    } = {},
) {
    // provision the machine's databases as files in a temporary directory
    const { database } = fixture;
    const directory = await fixture.environment.files.makeTemporaryDirectory("destack-instance-");
    const root = pathToFileURL(`${directory}/`);
    const runtime =
        options.runtime ?? new TestRuntime({ scheduler: fixture.environment.scheduler });
    const runtimes = options.runtimes ?? ["bun"];

    // run the chosen runtimes on the machine serving the space and record instances on the one test runtime
    const served = await fixture.spaceStore({
        providers: [
            databaseProvider(new BunSqliteDatabaseHost(root, fixture.environment), object.database),
            databaseProvider(new TestSpaceDatabaseHost(root, fixture.environment), object.database),
        ],
        ...(options.accounts === undefined ? {} : { accounts: options.accounts }),
        ...(options.destack === undefined ? {} : { destack: options.destack }),
        ...(options.isEnforcingEntitlements === true
            ? { isEnforcingEntitlements: true, publisherAt: () => TestEntitlement.publisher }
            : {}),
        runtimes: runtimes.map((name) =>
            name === runtime.name
                ? runtime
                : new TestRuntime({ scheduler: fixture.environment.scheduler, name }),
        ),
    });
    const { store } = served;

    return {
        root,
        runtime,
        store,
        authorizer: store.authorizer,
        deployments: {
            reconcile: async (id: string) => {
                // attach, provision the space's resources, record the targets bound, and deploy once the space serves the bound revision
                await fixture.applyInstalled(store);
                await reconcile(store, "rollout", { installationId: id });
                await reconcileResources(store, ids.space);
                await fixture.applyInstalled(store);
                await reconcile(store, "rollout", { installationId: id });
                await fixture.applyInstalled(store);

                return reconcile(store, "rollout", { installationId: id });
            },
            deploy: async (id: string) => {
                // deploy once the space serves the bound revision
                await fixture.applyInstalled(store);

                return reconcile(store, "rollout", { installationId: id });
            },
        },
        instances: {
            reconcile: (id: string) => reconcile(store, "deployment", { id }),
            restart: (id: string) => reconcile(store, "instance:restart", { id }),
            settle: () => {
                // run the store's and the stack package's control, each controller once, serving the installed revisions as the space's machine does
                const own = store.control();
                const stack = nth(store.packages, 0).control;
                const names = new Set(own.controllers.map((each) => each.name));
                const added = (stack?.controllers ?? []).filter((each) => !names.has(each.name));

                return settle(database, {
                    controllers: [
                        ...own.controllers,
                        ...added,
                        fixture.host.controller(() => fixture.applyInstalled(store)),
                    ],
                    cursors: [...own.cursors, ...(stack?.cursors ?? [])],
                });
            },
        },
        resources: {
            reconcile: (scope: string) => reconcileResources(store, scope),
        },
        approve: (id: Identifier<"installation">) => approvePlan(database, store, id),
        uninstall: (id: Identifier<"installation">) =>
            store.mutate(store.as(object.rollout), {
                id: RequestId.create(fixture.machine.environment),
                calls: [installation.call("delete", installation.target({ scope: ids.space, id }))],
            }),
    };
}

/** Select a target for a notes declaration of an installation, as its stack does. */
async function selectBinding(
    database: DatabaseConnection,
    id: Identifier<"installation">,
    name: string,
    target: BindingTarget,
) {
    // change the installation's desired bindings, moving its generation as its update does
    const [row] = await database
        .select({
            bindings: installation.table.bindings,
            generation: installation.table.generation,
        })
        .from(installation.table)
        .where(eq(installation.table.id, id));
    const { bindings, generation } = present(row, `installation ${id}`);
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(installation.table)
                .set({
                    bindings: {
                        ...bindings,
                        [ids.package]: { ...bindings[ids.package], [name]: { target } },
                    },
                    generation: generation + 1,
                })
                .where(eq(installation.table.id, id)),
    );
}

/** Read the revision a deployment runs. */
async function revisionOf(database: DatabaseConnection, id: Identifier<"installation-revision">) {
    const [revision] = await database
        .select()
        .from(object.installationRevision.table)
        .where(eq(object.installationRevision.table.id, id));

    return present(revision, `revision ${id}`);
}

/** Read how far an installation is, with its failure or what its ready condition says. */
async function readiness(database: DatabaseConnection, id: Identifier<"installation">) {
    const [row] = await database
        .select({
            generation: installation.table.generation,
            conditions: installation.table.conditions,
        })
        .from(installation.table)
        .where(eq(installation.table.id, id));
    const installed = present(row, "the installation");

    return [Observation.progress(installed), Observation.message(installed)];
}

/** Approve the plan an installation waits on as it stands. */
async function approvePlan(
    database: DatabaseConnection,
    store: SpaceStore["store"],
    id: Identifier<"installation">,
) {
    // read the plan the installation waits on
    const [target] = await Rollout.read(
        Snapshot.live(database),
        await database
            .select()
            .from(object.rollout.table)
            .where(eq(object.rollout.table.installationId, id)),
    );
    const resources = [object.database, object.dependency];
    const pending = await Approval.of(
        Snapshot.live(database),
        resources,
        present(target, "the installation's rollout"),
    );
    const digest = await Plan.digest(pending.plan(), database.state.environment.crypto);

    // approve it as the system
    const [granted] = await store.mutate(store.as(object.rollout), {
        id: RequestId.create(store.environment),
        calls: [
            object.rollout.call("approve", {
                ...object.rollout.target({
                    scope: present(target, "the installation's rollout").scope,
                    id: present(target, "the installation's rollout").installationId,
                }),
                plan: digest,
            }),
        ],
    });

    return granted;
}

test("deploy each workload with its bound resources, and drain and retire it once suspended", async () => {
    // install notes, whose notes the space's main database stores
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const notes = await installNotes(fixture);

    // deploy it on a bun machine, apply main, and run its instance on this machine
    const { runtime, deployments, instances, resources } = await serveControllers(fixture);
    await deployments.reconcile(notes.installationId);
    await resources.reconcile(ids.space);
    const deployed = await database.select().from(object.deployment.table);
    await instances.reconcile(nth(deployed, 0).id);
    const running = nth(await database.select().from(object.instance.table), 0);
    const installed = await readInstalled(database, notes.installationId);
    const revision = await revisionOf(database, nth(deployed, 0).revisionId);
    const run = nth(runtime.started, 0);

    // deploy only the bun output and run it without its main database, which its revision leaves out
    expect({
        deployments: deployed.map((entry) => [
            entry.status,
            entry.output,
            entry.runtime,
            entry.workload,
            entry.description,
        ]),
        instance: [running.status, running.machineId],
        run: [run.instanceId, run.deploymentId, run.scope, run.output, run.workload],
        resources: run.resources,
        bindings: revision.bindings,
        applied: installed.appliedRevisionId === installed.revisionId,
    }).toEqual({
        deployments: [["active", "server", "bun", "main", mainWorkload]],
        instance: ["running", fixture.machine.id],
        run: [running.id, nth(deployed, 0).id, ids.space, "server", "main"],
        resources: [],
        bindings: [],
        applied: true,
    });

    // suspend the application
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(installation.table)
                .set({ status: "suspended" })
                .where(eq(installation.table.id, notes.installationId)),
    );
    await deployments.reconcile(notes.installationId);
    await instances.reconcile(nth(deployed, 0).id);
    await instances.reconcile(nth(deployed, 0).id);
    const retired = nth(await database.select().from(object.deployment.table), 0);
    const stopped = nth(await database.select().from(object.instance.table), 0);
    expect([retired.status, stopped.status, runtime.stopped]).toEqual([
        "retired",
        "stopped",
        [running.id],
    ]);
});

test("apply an application's tables from its revision to the space's main database, start its instance once main applies them, and retain them for approval once it uninstalls", async () => {
    // install notes without a stack and deploy it
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const notes = await installNotes(fixture);
    const { runtime, deployments, instances, resources } = await serveControllers(fixture);
    await deployments.deploy(notes.installationId);

    // activate nothing while the space does not serve the revision, main not applying its tables yet
    const waiting = await readInstalled(database, notes.installationId);
    expect({
        started: runtime.started.length,
        deployments: await database.select().from(object.deployment.table),
        ready: [
            waiting.installation.conditions["Ready"]?.reason,
            waiting.installation.conditions["Ready"]?.message,
        ],
    }).toEqual({
        started: 0,
        deployments: [],
        ready: ["WaitingForSpace", `waiting for the space to serve revision ${waiting.revisionId}`],
    });

    // provision and apply main, deploy once the space serves the bound revision, and start the instance
    await resources.reconcile(ids.space);
    await deployments.deploy(notes.installationId);
    await instances.settle();
    const deployed = nth(await database.select().from(object.deployment.table), 0);
    const applied = nth(await database.select().from(object.database.table), 0);
    const running = nth(await database.select().from(object.instance.table), 0);
    const [revision] = await database
        .select({ states: object.installationRevision.table.states })
        .from(object.installationRevision.table)
        .where(eq(object.installationRevision.table.id, deployed.revisionId));
    const required = present(
        present(revision, "the deployed revision").states["main"],
        "main's tables",
    );
    const bound = await fixture.blobs.readJson(required, DatabaseState);
    expect({
        main: [applied.name, applied.ownerId, applied.conditions["Ready"]?.reason],
        bindings: (await revisionOf(database, deployed.revisionId)).bindings,
        tables: bound.tables.sqlite.map((state) => state.table.name),
        instance: [running.deploymentId, running.status],
        resources: present(runtime.started.at(-1), "the started run").resources,
    }).toEqual({
        main: ["main", ids.space, "Applied"],
        bindings: [],
        tables: ["example__notes__note"],
        instance: [deployed.id, "running"],
        resources: [],
    });

    // uninstall notes: drain its deployment, remove it, and wait for approval to drop its tables
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(installation.table)
                .set({ deletedAt: fixture.machine.clock.now() })
                .where(eq(installation.table.id, notes.installationId)),
    );
    await deployments.reconcile(notes.installationId);
    await instances.reconcile(deployed.id);
    await instances.reconcile(deployed.id);
    await deployments.reconcile(notes.installationId);
    await resources.reconcile(ids.space);
    const retained = nth(await database.select().from(object.database.table), 0);
    const remaining = await database
        .select({ name: installation.table.name })
        .from(installation.table)
        .where(isNotNull(installation.table.name))
        .orderBy(installation.table.name);
    expect({
        installations: remaining,
        revisions: await database
            .select({ id: object.installationRevision.table.id })
            .from(object.installationRevision.table)
            .where(eq(object.installationRevision.table.installationId, notes.installationId)),
        main: [
            retained.conditions["Ready"]?.reason,
            retained.plan?.steps.map((step) => [step.action, step.risk, step.target]) ?? null,
        ],
    }).toEqual({
        installations: [{ name: "stack" }],
        revisions: [],
        main: [
            "AwaitingApproval",
            [
                [
                    "delete",
                    "safe",
                    "table/example__notes__note/dependent/destack__access__relationship",
                ],
                ["delete", "destructive", "table/example__notes__note"],
            ],
        ],
    });
});

test("restart a failing instance in place after a doubling backoff, with one record of its last failure", async () => {
    // deploy notes with an applied database on a simulated machine with a failing runner
    const fixture = await TestSpace.open();
    const { simulation } = fixture;
    const { database } = fixture;
    const failing = new FailingRuntime({ scheduler: simulation.scheduler });
    const { first, early, second, recorded } = await (async () => {
        const notes = await installNotes(fixture);
        const { deployments, instances, resources } = await serveControllers(fixture, {
            runtime: failing,
        });
        await deployments.reconcile(notes.installationId);
        await resources.reconcile(ids.space);
        const deployed = nth(await database.select().from(object.deployment.table), 0);

        // fail, keep the instance through its backoff, then restart it once the backoff passed
        const backoff = async () => {
            const [row] = await database
                .select()
                .from(object.instance.table)
                .where(eq(object.instance.table.deploymentId, deployed.id));
            const { id, restartAt, stoppedAt } = present(row, "the deployment's instance");

            return { id, backoff: (restartAt ?? 0) - (stoppedAt ?? 0) };
        };
        await instances.reconcile(deployed.id);
        const failed = await backoff();
        await instances.reconcile(deployed.id);
        const retried = await backoff();
        await instances.restart(failed.id);
        await instances.reconcile(deployed.id);
        const restarted = await backoff();

        return {
            first: failed.backoff,
            early: retried.backoff,
            second: restarted.backoff,
            recorded: await database.select().from(object.instance.table),
        };
    })();
    expect({
        backoffs: [first, early, second],
        attempts: failing.attempts,
        instances: recorded.map((entry) => [
            entry.status,
            entry.restarts,
            entry.conditions["Ready"]?.status,
            entry.conditions["Ready"]?.reason,
            entry.conditions["Ready"]?.message,
        ]),
    }).toEqual({
        backoffs: [1000, 1000, 2000],
        attempts: 2,
        instances: [["failed", 1, "false", "StartFailed", "runner exited before serving"]],
    });
});

test("start a running instance again once its machine restarted and no longer runs its process", async () => {
    // run notes with an applied database
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const notes = await installNotes(fixture);
    const before = new TestRuntime({ scheduler: fixture.environment.scheduler });
    const first = await serveControllers(fixture, { runtime: before });
    await first.deployments.reconcile(notes.installationId);
    await first.resources.reconcile(ids.space);
    const deployed = nth(await database.select().from(object.deployment.table), 0);
    await first.instances.reconcile(deployed.id);

    // restart the machine with an idle runtime and reconcile again
    const after = new TestRuntime({ scheduler: fixture.environment.scheduler });
    const second = await serveControllers(fixture, { runtime: after });
    await second.instances.reconcile(deployed.id);
    const recorded = await database.select().from(object.instance.table);
    expect({
        before: before.started.map((run) => run.instanceId),
        after: after.started.map((run) => run.instanceId),
        instances: recorded.map((entry) => [entry.id, entry.status]),
    }).toEqual({
        before: [nth(recorded, 0).id],
        after: [nth(recorded, 0).id],
        instances: [[nth(recorded, 0).id, "running"]],
    });
});

test("deploy a submitted revision next to the previous one, serve the previous one until the new one runs, then retire it", async () => {
    // run the first revision of notes with its applied database
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const notes = await installNotes(fixture);
    const { deployments, instances, resources } = await serveControllers(fixture);
    await deployments.reconcile(notes.installationId);
    await resources.reconcile(ids.space);
    const first = nth(await database.select().from(object.deployment.table), 0);
    await instances.reconcile(first.id);

    // submit a second revision a millisecond later that deploys next to the first and drains it
    await fixture.simulation.sleep(
        fixture.simulation.clock.now() + 1 - fixture.simulation.clock.now(),
    );
    const titled = noteType({ body: field.string().optional(), title: field.string().optional() });
    const second = await submitRevision(
        fixture,
        notes.installationId,
        "f",
        await notesBuild([titled], fixture.environment.crypto),
    );
    await deployments.reconcile(notes.installationId);
    await resources.reconcile(ids.space);
    await deployments.reconcile(notes.installationId);
    const replacement = nth(
        await database
            .select()
            .from(object.deployment.table)
            .where(eq(object.deployment.table.revisionId, second.id)),
        0,
    );
    await instances.reconcile(first.id);
    const serving = nth(
        await database
            .select({ status: object.instance.table.status })
            .from(object.instance.table)
            .where(eq(object.instance.table.deploymentId, first.id)),
        0,
    );

    // run the replacement and stop and retire the drained first revision as its instance's changes wake it
    for (const id of [replacement.id, first.id, first.id]) {
        await instances.reconcile(id);
    }
    await deployments.reconcile(notes.installationId);
    const deployed = await database
        .select()
        .from(object.deployment.table)
        .orderBy(object.deployment.table.createdAt, object.deployment.table.id);
    const recorded = await database
        .select()
        .from(object.instance.table)
        .orderBy(object.instance.table.createdAt, object.instance.table.id);
    const installed = await readInstalled(database, notes.installationId);
    // serve the first revision until its replacement runs
    expect({
        serving: serving.status,
        deployments: deployed.map((entry) => [
            entry.revisionId,
            entry.output,
            entry.workload,
            entry.status,
        ]),
        instances: recorded.map((entry) => [entry.deploymentId, entry.status, entry.restarts]),
        applied: installed.appliedRevisionId,
    }).toEqual({
        serving: "running",
        deployments: [
            [first.revisionId, "server", "main", "retired"],
            [second.id, "server", "main", "active"],
        ],
        instances: [
            [first.id, "stopped", 0],
            [replacement.id, "running", 0],
        ],
        applied: second.id,
    });
});

test("redeploy the revision an application follows once the outputs its machine runs change", async () => {
    // deploy notes on a machine running workerd
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const notes = await installNotes(fixture);
    const { deployments: workerd } = await serveControllers(fixture, { runtimes: ["workerd"] });
    await workerd.reconcile(notes.installationId);

    // serve it on a bun machine a millisecond later to replace the workerd deployment
    await fixture.simulation.sleep(
        fixture.simulation.clock.now() + 1 - fixture.simulation.clock.now(),
    );
    const { deployments: bun } = await serveControllers(fixture, { runtimes: ["bun"] });
    await bun.reconcile(notes.installationId);
    await bun.reconcile(notes.installationId);
    const deployed = await database
        .select()
        .from(object.deployment.table)
        .orderBy(object.deployment.table.createdAt, object.deployment.table.id);
    expect(deployed.map((entry) => [entry.output, entry.runtime, entry.status])).toEqual([
        ["worker", "workerd", "draining"],
        ["server", "bun", "active"],
    ]);
});

test("redeploy an application once a binding its deployment captured targets another resource", async () => {
    // deploy notes with the bucket it owns for its files
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const files: Declared = {
        definer: objectPackage,
        name: "files",
        description: { name: "files", kind: "bucket", spec: {} },
    };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, files], fixture.environment.crypto),
    );
    const { deployments, resources } = await serveControllers(fixture);
    await deployments.reconcile(notes.installationId);
    await resources.reconcile(ids.space);
    await deployments.reconcile(notes.installationId);
    const owned = nth(await database.select().from(object.bucket.table), 0);

    // rebind the files a millisecond later to a second bucket of the same declaration
    await fixture.simulation.sleep(
        fixture.simulation.clock.now() + 1 - fixture.simulation.clock.now(),
    );
    const now = fixture.machine.clock.now();
    const second = nth(
        await database.transaction(
            async (transaction) =>
                await transaction
                    .insert(object.bucket.table)
                    .values({
                        ...owned,
                        id: Identifier.create("bucket", fixture.environment),
                        name: "notes.archive",
                        managerInstallationId: null,
                        managerPackageId: null,
                        managerName: null,
                        createdAt: now,
                        updatedAt: now,
                    })
                    .returning(),
        ),
        0,
    );
    await selectBinding(database, notes.installationId, "files", { type: "bucket", id: second.id });

    // revise and deploy again with the new target, and drain the deployment whose revision bound the old one
    await deployments.reconcile(notes.installationId);
    const deployed = await database
        .select()
        .from(object.deployment.table)
        .orderBy(object.deployment.table.createdAt, object.deployment.table.id);
    const bound = await Promise.all(
        deployed.map(async (entry) => {
            const revision = await revisionOf(database, entry.revisionId);
            const taken = revision.bindings.filter((each) => each.name === "files");

            return [entry.id, nth(taken, 0).target];
        }),
    );
    expect({
        deployments: deployed.map((entry) => entry.status),
        captures: bound,
    }).toEqual({
        deployments: ["draining", "active"],
        captures: [
            [nth(deployed, 0).id, owned.id],
            [nth(deployed, 1).id, second.id],
        ],
    });
});

test("deploy nothing of a release built with a newer or an older Destack than the machine runs, and report which", async () => {
    // deploy notes on a machine running only a later Destack, then on one running only an earlier one
    const deploy = async (destack: VersionRange) => {
        const fixture = await TestSpace.open();
        const notes = await installNotes(
            fixture,
            ids.notes,
            await fixtureBuild(ids.package, "2026.9.0", fixture.environment.crypto),
        );
        const { deployments } = await serveControllers(fixture, { destack });
        const refused = await refusal(deployments.reconcile(notes.installationId));
        const installed = await readInstalled(fixture.database, notes.installationId);
        const deployed = await fixture.database.select().from(object.deployment.table);
        const synced = installed.installation.conditions["Synced"];

        return [deployed.length, refused, synced?.message];
    };

    // refuse each, the release too old for the later machine and too new for the earlier one
    const built = import.meta.destack.package.version;
    const older = `@test/package is built with Destack ${built}, older than this machine supports (2099.1.0)`;
    const newer = `@test/package is built with Destack ${built}, newer than this machine's 2001.1.0`;
    expect({
        later: await deploy({ oldest: "2099.1.0", newest: "2099.1.0" }),
        earlier: await deploy({ oldest: "2001.1.0", newest: "2001.1.0" }),
    }).toEqual({
        later: [
            0,
            ["PRECONDITION_FAILED", "RELEASE_TOO_OLD", older],
            `@destack/stack RELEASE_TOO_OLD: ${older}`,
        ],
        earlier: [
            0,
            ["PRECONDITION_FAILED", "RELEASE_TOO_NEW", newer],
            `@destack/stack RELEASE_TOO_NEW: ${newer}`,
        ],
    });
});

test("deploy nothing of a build whose workloads have no output on a runtime the machine runs, and report why", async () => {
    // install notes with its workload only in a bun output
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const notes = await installNotes(
        fixture,
        ids.notes,
        await fixtureBuild(ids.package, "2026.9.0", fixture.environment.crypto),
    );
    const { deployments } = await serveControllers(fixture, {
        runtimes: ["workerd"],
    });

    // refuse deploying it on a workerd machine
    const message = "workloads main have no output on workerd";
    const refused = await refusal(deployments.reconcile(notes.installationId));
    const installed = await readInstalled(database, notes.installationId);
    expect({
        deployments: await database.select().from(object.deployment.table),
        refused,
        synced: installed.installation.conditions["Synced"],
        applied: installed.appliedRevisionId,
    }).toEqual({
        deployments: [],
        refused: ["PRECONDITION_FAILED", "RELEASE_UNSUPPORTED", message],
        synced: refusedCondition(installed, "RELEASE_UNSUPPORTED", message),
        applied: null,
    });
});

test("observe Ready at a new generation of an unchanged revision", async () => {
    // deploy notes, then bump its generation
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const notes = await installNotes(fixture);
    const { deployments } = await serveControllers(fixture);
    await deployments.reconcile(notes.installationId);
    const first = await readInstalled(database, notes.installationId);
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(installation.table)
                .set({ generation: first.installation.generation + 1 })
                .where(eq(installation.table.id, notes.installationId)),
    );

    // deploy it again
    await deployments.reconcile(notes.installationId);
    const next = await readInstalled(database, notes.installationId);
    expect({
        revisions: [first.appliedRevisionId, next.appliedRevisionId],
        ready: [readyOf(first), readyOf(next)],
    }).toEqual({
        revisions: [first.appliedRevisionId, first.appliedRevisionId],
        ready: [
            {
                status: "true",
                reason: "Deployed",
                message: "1 workloads deployed",
                observedGeneration: first.installation.generation,
            },
            {
                status: "true",
                reason: "Deployed",
                message: "1 workloads deployed",
                observedGeneration: first.installation.generation + 1,
            },
        ],
    });
});

test("deploy nothing of a build declaring its own database, and report why", async () => {
    // install notes declaring a database in addition to the space's main database
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const archive: Declared = {
        definer: objectPackage,
        name: "archive",
        description: { name: "archive", kind: "database", spec: {} },
    };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, archive], fixture.environment.crypto),
    );
    const { deployments } = await serveControllers(fixture);

    // refuse deploying it
    const message = `${ids.package} declares the databases archive: the space's main database keeps every installation's tables`;
    const refused = await refusal(deployments.reconcile(notes.installationId));
    const installed = await readInstalled(database, notes.installationId);
    expect({
        deployments: await database.select().from(object.deployment.table),
        refused,
        synced: installed.installation.conditions["Synced"],
        applied: installed.appliedRevisionId,
    }).toEqual({
        deployments: [],
        refused: ["PRECONDITION_FAILED", "DATABASE_DECLARED", message],
        synced: refusedCondition(installed, "DATABASE_DECLARED", message),
        applied: null,
    });
});

test("deploy nothing of a build routing hooks without serving them, and report why", async () => {
    // install notes routing hooks on its note type, built without stack's hooks workload
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const hooked: Declared = {
        ...requesting({}),
        description: {
            api: { name: "notes", procedures: [] },
            objects: {
                notes: {
                    type: { packageId: ids.package, type: "note" },
                    hooks: { create: ["handle"] },
                },
            },
            adapters: {},
            views: [],
            permissions: {},
            capabilities: {},
            errors: [],
        },
    };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, hooked], fixture.environment.crypto),
    );
    const { deployments } = await serveControllers(fixture);

    // refuse deploying it
    const message = `${notesPackage.name} routes hooks, but nothing serves them: declare @destack/stack`;
    const refused = await refusal(deployments.reconcile(notes.installationId));
    const installed = await readInstalled(database, notes.installationId);
    expect({
        deployments: await database.select().from(object.deployment.table),
        refused,
        synced: installed.installation.conditions["Synced"],
    }).toEqual({
        deployments: [],
        refused: ["PRECONDITION_FAILED", "HOOK_UNSERVED", message],
        synced: refusedCondition(installed, "HOOK_UNSERVED", message),
    });
});

test("deploy nothing of a build declaring adapters without a workload answering them, and report why", async () => {
    // install notes declaring an adapter, built without any workload
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const adapted: Declared = {
        ...requesting({}),
        description: {
            api: { name: "notes", procedures: [] },
            objects: {},
            adapters: { inbox: { path: "/inbox" } },
            views: [],
            permissions: {},
            capabilities: {},
            errors: [],
        },
    };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, adapted], fixture.environment.crypto, {}, []),
    );
    const { deployments } = await serveControllers(fixture);

    // refuse deploying it
    const message = `${notesPackage.name} declares adapters, but no workload of its build answers them`;
    const refused = await refusal(deployments.reconcile(notes.installationId));
    const installed = await readInstalled(database, notes.installationId);
    expect({
        deployments: await database.select().from(object.deployment.table),
        refused,
        synced: installed.installation.conditions["Synced"],
    }).toEqual({
        deployments: [],
        refused: ["PRECONDITION_FAILED", "ADAPTER_UNSERVED", message],
        synced: refusedCondition(installed, "ADAPTER_UNSERVED", message),
    });
});

/** The region notes reads at start, which has no default. */
const region = defineSetting({
    name: "region",
    title: "Region",
    description: "The region notes serves.",
    schema: schema.string().min(1),
    scope: "space",
    overrides: ["installation"],
    apply: "restart",
    required: true,
});

test("deploy nothing of a build whose space settings without a default are unset, waiting for them", async () => {
    // install notes declaring a region that no scope sets
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const regional: Declared = {
        kind: "setting",
        definer: settingPackage,
        name: region.name,
        description: { ...describeSetting(region), package: notesPackage },
    };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, regional], fixture.environment.crypto),
    );
    const { deployments } = await serveControllers(fixture);

    // deploy it
    await deployments.reconcile(notes.installationId);
    const installed = await readInstalled(database, notes.installationId);
    const message = `waiting for settings ${ids.package}/region`;
    expect({
        deployments: await database.select().from(object.deployment.table),
        conditions: installed.installation.conditions,
    }).toEqual({
        deployments: [],
        conditions: {
            Bound: {
                status: "false",
                reason: "WaitingForSettings",
                message,
                observedGeneration: installed.installation.generation,
                lastTransitionAt: installed.installation.conditions["Bound"]?.lastTransitionAt,
            },
            Ready: {
                status: "false",
                reason: "WaitingForSettings",
                message,
                observedGeneration: installed.installation.generation,
                lastTransitionAt: installed.installation.conditions["Ready"]?.lastTransitionAt,
            },
            // record the pass synced, as waiting on settings is no failure
            Synced: {
                status: "true",
                reason: "ReconcileSuccess",
                message: "",
                observedGeneration: installed.installation.generation,
                lastTransitionAt: installed.installation.conditions["Synced"]?.lastTransitionAt,
            },
        },
    });
});

test("run each workload with its build's sandbox, what its allowed capabilities unlock, and its directories", async () => {
    // install notes whose workload runs git, and whose optional capabilities call an issue tracker, index chosen directories or scan receipts
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const base = Sandbox.parse({ run: { commands: ["git"] } });
    const declared = requesting({});
    const request: Declared = {
        ...declared,
        description: {
            ...declared.description,
            capabilities: {
                issues: {
                    reason: "Link issues",
                    optional: true,
                    sandbox: { network: { connect: ["api.github.com"] } },
                },
                index: {
                    reason: "Index your notes",
                    optional: true,
                    sandbox: { files: { access: "read" } },
                },
                scanning: { reason: "Scan receipts", optional: true, sandbox: { camera: {} } },
            },
        },
    };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, request], fixture.environment.crypto, base),
    );
    const { runtime, deployments, instances, resources } = await serveControllers(fixture);
    const notesDirectory = { path: "/Users/person/Notes", access: "read" as const };
    const grant = async (
        capabilities: readonly string[],
        directories: readonly DirectoryGrant[] = [],
    ) => {
        await database.transaction(
            async (transaction) =>
                await transaction
                    .update(installation.table)
                    .set({ capabilities: [...capabilities], directories: [...directories] })
                    .where(eq(installation.table.id, notes.installationId)),
        );

        return deployments.reconcile(notes.installationId);
    };
    const deployed = async () =>
        (
            await database
                .select()
                .from(object.deployment.table)
                .orderBy(object.deployment.table.createdAt, object.deployment.table.id)
        ).map((entry) => [entry.status, entry.description.sandbox, entry.directories]);

    // refuse allowing an undeclared capability, a directory without fs, writing a directory fs reads,
    //  and a directory reaching the host's service sockets
    await refusal(grant(["microphone"]));
    const undeclared = await readiness(database, notes.installationId);
    await refusal(grant([], [notesDirectory]));
    const unreadable = await readiness(database, notes.installationId);
    await refusal(grant(["index"], [{ ...notesDirectory, access: "write" }]));
    const overreaching = await readiness(database, notes.installationId);
    await refusal(grant(["index"], [{ path: "/var/run", access: "read" }]));
    const socketed = await readiness(database, notes.installationId);
    const refusedDeployments = await deployed();

    // deploy with the build's sandbox alone
    await grant([]);
    await resources.reconcile(ids.space);
    const first = nth(await database.select().from(object.deployment.table), 0);
    await instances.reconcile(first.id);
    const unallowed = await deployed();

    // allow the issues, indexing and scanning a millisecond later, and redeploy the workload with their host abilities and the directory
    await fixture.simulation.sleep(
        fixture.simulation.clock.now() + 1 - fixture.simulation.clock.now(),
    );
    await grant(["issues", "index", "scanning"], [notesDirectory]);
    const allowed = await deployed();
    expect({
        undeclared,
        unreadable,
        overreaching,
        socketed,
        refusedDeployments,
        started: runtime.started.map((started) => [started.sandbox, started.directories]),
        unallowed,
        allowed,
    }).toEqual({
        undeclared: [
            "failed",
            "@destack/stack CAPABILITY_UNDECLARED: notes 2026.9.0 declares no capabilities microphone to allow",
        ],
        unreadable: [
            "failed",
            "@destack/stack CAPABILITY_UNDECLARED: notes 2026.9.0 enables no fs ability to grant directories /Users/person/Notes",
        ],
        overreaching: [
            "failed",
            "@destack/stack CAPABILITY_UNDECLARED: notes 2026.9.0 enables fs for reading only, refusing to grant writing /Users/person/Notes",
        ],
        socketed: [
            "failed",
            "@destack/stack CAPABILITY_UNDECLARED: notes 2026.9.0 may not be granted directories reaching the host's service sockets in /run, /var/run or all of /tmp: /var/run",
        ],
        refusedDeployments: [],
        started: [[base, []]],
        unallowed: [["active", base, []]],
        allowed: [
            ["draining", base, []],
            [
                "active",
                { ...base, network: { connect: ["api.github.com"] }, files: { access: "read" } },
                [notesDirectory],
            ],
        ],
    });
});

test("fail an instance granted files on a runtime without a filesystem to grant", async () => {
    // deploy notes requiring files on a machine running workerd only
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const files = Sandbox.parse({ files: { access: "read" } });
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main], fixture.environment.crypto, files),
    );
    const { deployments, instances, resources } = await serveControllers(fixture, {
        runtimes: ["workerd"],
        runtime: new TestRuntime({ scheduler: fixture.environment.scheduler, name: "workerd" }),
    });
    await deployments.reconcile(notes.installationId);
    await resources.reconcile(ids.space);
    const deployed = nth(await database.select().from(object.deployment.table), 0);

    // record the refusal on the instance
    await instances.reconcile(deployed.id);
    const [recorded] = await database.select().from(object.instance.table);
    expect([
        recorded?.status,
        recorded?.conditions["Ready"]?.reason,
        recorded?.conditions["Ready"]?.message,
    ]).toEqual(["failed", "StartFailed", "the workerd runtime has no filesystem to grant"]);
});

test("refuse to deploy an installation binding without an address, then run the workload with the address its stack declares", async () => {
    // install notes calling the tasks installation besides its main database bound to a provisioned database
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, tasks], fixture.environment.crypto),
    );
    const { runtime, deployments, instances, resources } = await serveControllers(fixture);

    // refuse to deploy while no stack binds the tasks installation to an address
    await refusal(deployments.reconcile(notes.installationId));
    const refused = await readiness(database, notes.installationId);

    // declare the tasks dependency at the address of the notes installation itself and bind it
    const now = fixture.machine.clock.now();
    const record = { scope: ids.space, createdAt: now, updatedAt: now };
    const declared = {
        ...record,
        ...ObjectReference.columns(
            "owner",
            principal.space.reference(Scope.universe.id, ids.space),
        ),
        id: schema
            .identifier("dependency")
            .parse("dependency-01996ab0-0000-7000-8000-000000000021"),
        name: "tasks",
        definitionPackageId: objectPackage.id,
        definitionVersion: "2026.9.0",
        definitionName: "tasks",
        spec: { ...InstallationBindingSpec.parse(tasks.description["spec"]), address: "notes" },
        provider: "http",
        reference: "notes",
    };
    await database.transaction(
        async (transaction) => await transaction.insert(object.dependency.table).values(declared),
    );
    await selectBinding(database, notes.installationId, "tasks", {
        type: "dependency",
        id: declared.id,
    });

    // observe it ready as declared and deploy and run with the database of the installation
    await resources.reconcile(ids.space);
    const observed = nth(
        await database
            .select({ conditions: object.dependency.table.conditions })
            .from(object.dependency.table)
            .where(eq(object.dependency.table.id, declared.id)),
        0,
    );
    await deployments.reconcile(notes.installationId);
    await resources.reconcile(ids.space);
    const deployed = nth(await database.select().from(object.deployment.table), 0);
    await instances.reconcile(deployed.id);
    const dependency = runtime.started[0]?.resources.find((entry) => entry.kind === "installation");

    // report the missing address and run with the declared address and its connecting provider
    expect({
        refused,
        declared: [observed.conditions["Ready"]?.status, observed.conditions["Ready"]?.reason],
        dependency: [dependency?.name, dependency?.reference, dependency?.provider],
    }).toEqual({
        refused: [
            "failed",
            `@destack/stack BINDING_MISSING: bind installations ${ids.package} tasks to their addresses`,
        ],
        declared: ["true", "Applied"],
        dependency: ["tasks", "notes", "http"],
    });
});

test("bind a workload to an installation another space grants", async () => {
    // install notes bound to the board installation of another space, by its name there
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, tasks], fixture.environment.crypto),
    );
    const { runtime, deployments, instances, resources } = await serveControllers(fixture);
    const acme = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000031");
    const board = "board";
    await selectBinding(database, notes.installationId, "tasks", {
        type: "installation",
        name: board,
        spaceId: acme,
    });

    // deploy it and run its instance
    await deployments.reconcile(notes.installationId);
    await resources.reconcile(ids.space);
    const deployed = nth(await database.select().from(object.deployment.table), 0);
    await instances.reconcile(deployed.id);
    const captured = (await revisionOf(database, deployed.revisionId)).bindings.map((entry) => ({
        name: entry.name,
        target: entry.target,
        targetSpaceId: entry.targetSpaceId,
        version: entry.version,
    }));

    // capture the installation and its space and bind it apart from the space's own resources
    expect({
        captured: captured.filter((entry) => entry.name === "tasks"),
        installations: runtime.started[0]?.installations,
        dependencies: runtime.started[0]?.resources.filter(
            (entry) => entry.kind === "installation",
        ),
    }).toEqual({
        captured: [{ name: "tasks", target: board, targetSpaceId: acme, version: null }],
        installations: [
            {
                packageId: ids.package,
                name: "tasks",
                package: ids.package,
                installation: board,
                scope: acme,
            },
        ],
        dependencies: [],
    });
});

test("retain the columns a draining release still declares, and drop them for approval once it retires", async () => {
    // run the first revision of notes whose notes have a color
    const fixture = await TestSpace.open();
    const { database } = fixture;
    let declared: readonly Declared[] = [
        noteType({ body: field.string().optional(), color: field.string().optional() }),
    ];
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild(declared, fixture.environment.crypto),
    );
    const { deployments, instances, resources } = await serveControllers(fixture);
    await deployments.reconcile(notes.installationId);
    await resources.reconcile(ids.space);
    const first = nth(await database.select().from(object.deployment.table), 0);
    await instances.reconcile(first.id);

    // deploy a second revision that drops the color next to the first, and plan while the first drains
    declared = [main];
    const second = await submitRevision(
        fixture,
        notes.installationId,
        "f",
        await notesBuild(declared, fixture.environment.crypto),
    );
    await deployments.reconcile(notes.installationId);
    await resources.reconcile(ids.space);
    const plan = async () => {
        const row = nth(await database.select().from(object.database.table), 0);
        return [
            row.conditions["Ready"]?.reason,
            row.plan?.steps.map((step) => [step.action, step.risk, step.detail]) ?? null,
        ];
    };
    const whileDraining = await plan();

    // run the second revision and retire the first
    const replacement = nth(
        await database
            .select()
            .from(object.deployment.table)
            .where(eq(object.deployment.table.revisionId, second.id)),
        0,
    );
    await instances.reconcile(replacement.id);
    await instances.reconcile(first.id);
    await instances.reconcile(first.id);
    await resources.reconcile(ids.space);

    // leave the color while the first release serves, and wait for approval to drop it
    expect({ whileDraining, retired: await plan() }).toEqual({
        whileDraining: ["Applied", null],
        retired: ["AwaitingApproval", [["replace", "destructive", "rebuild table: drop color"]]],
    });
});

test("recreate over a draining release whose values a conversion narrows: stop it once approved, convert, then run the replacement", async () => {
    // run the first revision of notes whose modes include emacs
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const first = noteType({ mode: field.enum(["standard", "vim", "emacs"]) });
    const narrowed = noteType(
        { mode: field.enum(["standard", "vim"]) },
        {
            version: "2026.10.0",
            convert: {
                "2026.10.0": {
                    mode: Expression.case(
                        Expression.column("mode"),
                        [{ when: "emacs", then: Expression.literal("standard") }],
                        Expression.column("mode"),
                    ),
                },
            },
        },
    );
    let declared: readonly Declared[] = [first];
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild(declared, fixture.environment.crypto),
    );
    const { deployments, instances, resources, approve } = await serveControllers(fixture);
    await deployments.reconcile(notes.installationId);
    await resources.reconcile(ids.space);
    const original = nth(await database.select().from(object.deployment.table), 0);
    await instances.reconcile(original.id);

    // follow the narrowing revision a millisecond later, and plan a recreation that waits for approval
    await fixture.simulation.sleep(
        fixture.simulation.clock.now() + 1 - fixture.simulation.clock.now(),
    );
    declared = [narrowed];
    const second = await submitRevision(
        fixture,
        notes.installationId,
        "f",
        await notesBuild(declared, fixture.environment.crypto),
    );
    await deployments.reconcile(notes.installationId);
    await resources.reconcile(ids.space);
    const read = async () => {
        const row = nth(await database.select().from(object.database.table), 0);
        return {
            reason: row.conditions["Ready"]?.reason,
            plan: row.plan ?? undefined,
            id: row.id,
        };
    };
    const waiting = await read();

    // approve it through notes and replace the first release
    const approved = await approve(notes.installationId);
    await resources.reconcile(ids.space);
    const recreating = await read();
    const stopping = nth(
        await database
            .select({ status: object.deployment.table.status })
            .from(object.deployment.table)
            .where(eq(object.deployment.table.id, original.id)),
        0,
    );
    await instances.reconcile(original.id);
    await instances.reconcile(original.id);
    await resources.reconcile(ids.space);
    const converted = await read();

    // deploy the replacement once main applied its converted tables
    await deployments.reconcile(notes.installationId);
    const replacement = nth(
        await database
            .select()
            .from(object.deployment.table)
            .where(eq(object.deployment.table.revisionId, second.id)),
        0,
    );
    await instances.reconcile(replacement.id);
    const statuses = await database
        .select({ id: object.deployment.table.id, status: object.deployment.table.status })
        .from(object.deployment.table)
        .orderBy(object.deployment.table.createdAt, object.deployment.table.id);

    // recreate only once approved with the first release stopped before converting
    expect({
        waiting: [waiting.reason, waiting.plan?.steps.map((step) => [step.action, step.risk])],
        approved,
        recreating: [recreating.reason, stopping.status],
        converted: converted.reason,
        statuses: statuses.map((entry) => entry.status),
    }).toEqual({
        waiting: [
            "AwaitingApproval",
            [
                ["delete", "backward-incompatible"],
                ["replace", "fallible"],
                ["convert", "fallible"],
            ],
        ],
        approved: {
            steps: present(waiting.plan, "the waiting plan").steps.map((step) => ({
                ...step,
                target: `database/main/${step.target}`,
            })),
        },
        recreating: ["Recreating", "stopping"],
        converted: "Applied",
        statuses: ["retired", "active"],
    });
});

test("serve a release while a caller is pinned to it, and retire it once the caller retires", async () => {
    // run the first revision of notes
    const fixture = await TestSpace.open();
    const { database } = fixture;
    let declared: readonly Declared[] = [main];
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild(declared, fixture.environment.crypto),
    );
    const { deployments, instances, resources } = await serveControllers(fixture);
    await deployments.reconcile(notes.installationId);
    await resources.reconcile(ids.space);
    const first = nth(await database.select().from(object.deployment.table), 0);
    await instances.reconcile(first.id);

    // bind a caller's active deployment to notes at the first release
    const now = fixture.machine.clock.now();
    const record = { scope: ids.space, createdAt: now, updatedAt: now };
    const dependency = {
        ...record,
        ...ObjectReference.columns(
            "owner",
            principal.space.reference(Scope.universe.id, ids.space),
        ),
        id: schema
            .identifier("dependency")
            .parse("dependency-01996ab0-0000-7000-8000-000000000031"),
        name: "calls.notes",
        definitionPackageId: objectPackage.id,
        definitionVersion: "2026.9.0",
        definitionName: "notes",
        spec: { package: ids.package, address: "notes" },
        provider: "http",
        reference: "notes",
    };
    const [built] = await database
        .select({ buildId: object.installationRevision.table.buildId })
        .from(object.installationRevision.table)
        .where(eq(object.installationRevision.table.id, first.revisionId));
    const pinned = {
        ...record,
        id: schema
            .identifier("installation-revision")
            .parse("installation-revision-01996ab0-0000-7000-8000-000000000034"),
        installationId: first.installationId,
        buildId: present(built, "the first revision").buildId,
        dependencies: [],
        states: { notes: await fixture.blobs.writeJson({ release: "2026.9.0" }) },
        bindings: [
            {
                packageId: ids.package,
                name: "notes",
                target: dependency.id,
                targetSpaceId: null,
                version: 1,
            },
        ],
        digest: Digest.parse("c".repeat(64)),
        generation: 1,
    };
    const caller = {
        ...record,
        id: schema
            .identifier("deployment")
            .parse("deployment-01996ab0-0000-7000-8000-000000000032"),
        installationId: first.installationId,
        packageId: ids.package,
        revisionId: pinned.id,
        output: "server",
        workload: "caller",
        runtime: "bun" as const,
        release: "2026.9.0",
        description: first.description,
        status: "active" as const,
        activatedAt: first.activatedAt,
    };
    await database.transaction(
        async (transaction) => await transaction.insert(object.dependency.table).values(dependency),
    );
    await database.transaction(async (transaction) => {
        await transaction.insert(object.installationRevision.table).values(pinned);
        await transaction.insert(object.deployment.table).values(caller);
    });

    // deploy a second revision whose package serves only callers since 2026.10.0 and run it
    declared = [
        main,
        {
            kind: "package",
            definer: objectPackage,
            name: "notes",
            description: {
                since: "2026.10.0",
                api: { name: "notes", procedures: [] },
                objects: {},
                adapters: {},
                views: [],
                permissions: {},
                capabilities: {},
                errors: [],
            },
        },
    ];
    const second = await submitRevision(
        fixture,
        notes.installationId,
        "f",
        await notesBuild(declared, fixture.environment.crypto),
    );
    await deployments.reconcile(notes.installationId);
    await resources.reconcile(ids.space);
    const replacement = nth(
        await database
            .select()
            .from(object.deployment.table)
            .where(eq(object.deployment.table.revisionId, second.id)),
        0,
    );
    await instances.reconcile(replacement.id);
    await instances.reconcile(first.id);
    const status = async () =>
        nth(
            await database
                .select({ status: object.deployment.table.status })
                .from(object.deployment.table)
                .where(eq(object.deployment.table.id, first.id)),
            0,
        ).status;
    const whilePinned = await status();

    // retire the caller with its pinned release, then stop and retire the first release as its retirement and instance wake it
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(object.deployment.table)
                .set({ status: "draining" })
                .where(eq(object.deployment.table.id, caller.id)),
    );
    await instances.reconcile(caller.id);
    await instances.reconcile(first.id);
    await instances.reconcile(first.id);

    // serve the pinned release until its caller retires
    expect({ since: replacement.since, whilePinned, released: await status() }).toEqual({
        since: "2026.10.0",
        whilePinned: "draining",
        released: "retired",
    });
});

test("grant an applied application exactly the space permissions its package requests in its role, lend its universe-scoped request in every scope, and drop both once it is uninstalled", async () => {
    // install notes requesting reading the space and reading where people receive notifications
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const recipient = accountObject.user.permission("recipient");
    const request = requesting({
        space: { space: [space.permission("read")], universe: [recipient] },
    });
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, request], fixture.environment.crypto),
    );

    // apply it on a bun machine
    const { deployments, instances } = await serveControllers(fixture);
    await deployments.reconcile(notes.installationId);
    const deployed = nth(await database.select().from(object.deployment.table), 0);

    // read the installation's role in its space with its permissions, and the space's grants
    const installed = principal.installation.reference(ids.space, notes.installationId);
    const roles = async () =>
        (
            await database
                .select({
                    scope: accessRole.scope,
                    name: accessRole.name,
                    type: accessRolePermission.type,
                    permission: accessRolePermission.name,
                })
                .from(accessRole)
                .innerJoin(accessRolePermission, eq(accessRolePermission.roleId, accessRole.id))
                .where(eq(accessRole.name, `installation/${notes.installationId}`))
        ).map((row) => Object.values(row));
    const grants = async () =>
        (await database.select().from(accessGrant)).map((row) => ({
            delegator: ObjectReference.at("delegator", row),
            delegate: ObjectReference.at("delegate", row),
            permissions: row.permissions,
        }));
    const granted = { roles: await roles(), grants: await grants() };

    // uninstall it
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(installation.table)
                .set({ deletedAt: fixture.machine.clock.now() })
                .where(eq(installation.table.id, notes.installationId)),
    );
    await deployments.reconcile(notes.installationId);
    await instances.reconcile(deployed.id);
    await deployments.reconcile(notes.installationId);
    expect({
        granted,
        dropped: { roles: await roles(), grants: await grants() },
    }).toEqual({
        granted: {
            roles: [[ids.space, `installation/${notes.installationId}`, "space", "read"]],
            // lend the universe-scoped request on the space's authority, in every scope
            grants: [
                {
                    delegator: principal.space.reference(Scope.universe.id, ids.space),
                    delegate: installed,
                    permissions: [recipient],
                },
            ],
        },
        dropped: { roles: [], grants: [] },
    });
});

test("hold a revision asking its space for more access until approved, whatever the space's threshold, then grant it", async () => {
    // run notes asking nothing of its space
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const { crypto } = fixture.environment;
    const notes = await installNotes(fixture, ids.notes, await notesBuild([main], crypto));
    const { deployments, approve } = await serveControllers(fixture);
    await deployments.reconcile(notes.installationId);

    // submit a revision asking to read the space, and deploy it
    const request = requesting({ space: { space: [space.permission("read")] } });
    await submitRevision(
        fixture,
        notes.installationId,
        "f",
        await notesBuild([main, request], crypto),
    );
    await deployments.reconcile(notes.installationId);
    const held = await readInstalled(database, notes.installationId);
    const roles = async () =>
        (
            await database
                .select({ permission: accessRolePermission.name })
                .from(accessRole)
                .innerJoin(accessRolePermission, eq(accessRolePermission.roleId, accessRole.id))
                .where(eq(accessRole.name, `installation/${notes.installationId}`))
        ).map((row) => row.permission);
    const before = await roles();

    // approve the grant, then deploy it
    await approve(notes.installationId);
    await deployments.reconcile(notes.installationId);
    const applied = await readInstalled(database, notes.installationId);

    // wait on the grant as a privileged step above the space's default threshold, and grant it once approved
    expect({
        held: [held.installation.conditions["Ready"]?.reason, held.installation.plan],
        before,
        applied: [applied.appliedRevisionId === applied.revisionId, await roles()],
    }).toEqual({
        held: [
            "AwaitingApproval",
            {
                steps: [
                    {
                        action: "grant",
                        target: `installation/notes/access/space/${space.permission("read").packageId}/space/read`,
                        risk: "privileged",
                        detail: "request read on space",
                    },
                ],
            },
        ],
        before: [],
        applied: [true, ["read"]],
    });
});

/** Run notes, hold a revision asking to read the space, and read the change request it opens. */
async function holdForAccess() {
    // run notes, then hold a revision asking to read the space
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const { crypto } = fixture.environment;
    const notes = await installNotes(fixture, ids.notes, await notesBuild([main], crypto));
    const { deployments, store } = await serveControllers(fixture);
    await deployments.reconcile(notes.installationId);
    const request = requesting({ space: { space: [space.permission("read")] } });
    await submitRevision(
        fixture,
        notes.installationId,
        "f",
        await notesBuild([main, request], crypto),
    );
    await deployments.reconcile(notes.installationId);

    // read the change request its rollout opened and the one approval it asks
    const [rolled] = await database
        .select()
        .from(object.rollout.table)
        .where(eq(object.rollout.table.installationId, notes.installationId));
    const opened = single(
        await database
            .select()
            .from(changeRequest.table)
            .where(eq(changeRequest.table.parentId, present(rolled, "the rollout").id)),
    );
    const asked = single(
        await database.select().from(approval.table).where(eq(approval.table.parentId, opened.id)),
    );

    // decide it as the space's owner, a manager, and let the change request act on the decision
    const decide = async (decision: "approve" | "reject") => {
        const owner = fixture.owner.subject;
        await store.mutate(AccessContext.of(Caller.of(owner), store.clock.now()), {
            id: RequestId.create(store.environment),
            calls: [approval.call(decision, approval.target(asked))],
        });
        await reconcile(store, "change-request", { id: opened.id });
    };

    return { fixture, database, deployments, notes, opened, asked, decide };
}

test("ask the space's managers to approve a held plan through a change request on its rollout, applying it once one approves", async () => {
    // hold a revision asking to read the space, and read the plan it waits on
    const { fixture, database, deployments, notes, opened, asked, decide } = await holdForAccess();
    const plan = present(
        (await readInstalled(database, notes.installationId)).installation.plan,
        "the waiting plan",
    );

    // approve it as a manager, then deploy the approved plan
    await decide("approve");
    await deployments.reconcile(notes.installationId);
    const [decided] = await database
        .select()
        .from(changeRequest.table)
        .where(eq(changeRequest.table.id, opened.id));
    const applied = await readInstalled(database, notes.installationId);

    // ask the managers as one set to approve exactly the held plan, and apply it once the owner approved
    expect({
        asked: { ...ObjectReference.at("approver", asked), relation: asked.approverRelation },
        call: [opened.call.method, opened.call.input["plan"], opened.as],
        state: decided?.state,
        applied: applied.appliedRevisionId === applied.revisionId,
    }).toEqual({
        asked: { ...principal.space.reference(Scope.universe.id, ids.space), relation: "manage" },
        call: ["approve", await Plan.digest(plan, fixture.environment.crypto), "approver"],
        state: "applied",
        applied: true,
    });
});

test("fail a held plan once a manager rejects it, keeping the applied revision", async () => {
    // hold a revision asking to read the space, then reject its plan as a manager
    const { database, deployments, notes, opened, decide } = await holdForAccess();
    await decide("reject");
    const refused = await refusal(deployments.reconcile(notes.installationId));
    const held = await readInstalled(database, notes.installationId);
    const digest = Digest.parse(opened.call.input["plan"]);

    // fail the installation with the rejection, still running the revision applied before
    expect({
        refused,
        progress: [Observation.progress(held.installation), Observation.message(held.installation)],
        upgraded: held.appliedRevisionId === held.revisionId,
    }).toEqual({
        refused: [
            "CONFLICT",
            "PLAN_REJECTED",
            `plan ${digest} was rejected: change the installation for another plan`,
        ],
        progress: [
            "failed",
            `@destack/stack PLAN_REJECTED: plan ${digest} was rejected: change the installation for another plan`,
        ],
        upgraded: false,
    });
});

test("let an application ask for its optional capability in approval mode, and run with it once allowed, lending its universe-scoped permissions in every scope", async () => {
    // install notes declaring an optional push capability needing to unsubscribe people's browsers
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const unsubscribe = accountObject.client.permission("unsubscribe");
    const declared = requesting({});
    const request: Declared = {
        ...declared,
        description: {
            ...declared.description,
            capabilities: {
                push: {
                    reason: "Push notifications",
                    optional: true,
                    permissions: { space: { universe: [unsubscribe] } },
                },
            },
        },
    };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, request], fixture.environment.crypto),
    );
    const { deployments } = await serveControllers(fixture);
    const installed = principal.installation.reference(ids.space, notes.installationId);
    const grants = async () =>
        (await database.select().from(accessGrant)).map((row) => row.permissions);

    // deploy it, then allow push as its space would once it approves the request
    await deployments.reconcile(notes.installationId);
    const asking = await grants();
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(installation.table)
                .set({ allowed: ["push"] })
                .where(eq(installation.table.id, notes.installationId)),
    );
    await deployments.reconcile(notes.installationId);
    const enabled = (
        await database
            .select({
                enabled: object.deployment.table.enabled,
                status: object.deployment.table.status,
            })
            .from(object.deployment.table)
    ).toSorted((left, right) => left.status.localeCompare(right.status));

    // ask for push in approval mode on its own row, then run with it
    const allow = {
        ...installation.permission("allow"),
        scope: ids.space,
        objectId: notes.installationId,
        mode: "approval",
    };
    expect({ installed: installed.id, asking, allowed: await grants(), enabled }).toEqual({
        installed: notes.installationId,
        asking: [[allow]],
        // lend push's universe-scoped permission once allowed
        allowed: [[unsubscribe, allow]],
        enabled: [
            { enabled: ["push"], status: "active" },
            { enabled: [], status: "draining" },
        ],
    });
});

test("refuse deploying a capability whose feature the account lacks, and deploy it once the account is entitled", async () => {
    // install notes requiring a sync capability that needs the plan's sync feature
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const feature = {
        packageId: PackageId.parse("package-01996ab0-0000-7000-8000-0000000000f1"),
        name: "sync",
    };
    const declared = requesting({});
    const request: Declared = {
        ...declared,
        description: {
            ...declared.description,
            capabilities: { sync: { reason: "Sync across devices", features: [feature] } },
        },
    };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, request], fixture.environment.crypto),
    );
    const { deployments } = await serveControllers(fixture, {
        isEnforcingEntitlements: true,
    });
    const deployed = async () =>
        (
            await database
                .select({ enabled: object.deployment.table.enabled })
                .from(object.deployment.table)
        ).map((row) => row.enabled);

    // deploy with the account replicated without the feature, then once it is entitled to it
    await TestEntitlement.set(
        database,
        ids.account,
        "database",
        { kind: "boolean" },
        fixture.machine.environment,
    );
    await refusal(deployments.reconcile(notes.installationId));
    const refused = {
        ready: await readiness(database, notes.installationId),
        deployed: await deployed(),
    };
    await TestEntitlement.set(
        database,
        ids.account,
        { feature },
        { kind: "boolean" },
        fixture.machine.environment,
    );
    await deployments.reconcile(notes.installationId);

    // refuse naming the missing feature, then run with the capability
    expect({ refused, entitled: await deployed() }).toEqual({
        refused: {
            ready: [
                "failed",
                "@destack/stack CAPABILITY_UNENTITLED: capability sync needs sync: no plan of the account grants feature sync",
            ],
            deployed: [],
        },
        entitled: [["sync"]],
    });
});

test("deploy an application depending on a package the universe installs, which every space's chain includes, and refuse one depending on a package installed nowhere on its chain", async () => {
    // install notes depending on the account package, installed at the universe alone
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const notes = await installNotes(fixture);
    const depend = async (dependency: Package) =>
        await database.transaction(
            async (transaction) =>
                await transaction
                    .update(object.installationRevision.table)
                    .set({
                        dependencies: [
                            { package: dependency, manifest: Digest.parse("0".repeat(64)) },
                        ],
                    })
                    .where(
                        eq(object.installationRevision.table.installationId, notes.installationId),
                    ),
        );
    await depend(Package.of(accountPackage));
    const { deployments } = await serveControllers(fixture, {});

    // deploy it, then depend on a vault package no scope on the chain installs and deploy again
    await deployments.reconcile(notes.installationId);
    const deployed = await readiness(database, notes.installationId);
    await depend(VAULT);
    await refusal(deployments.reconcile(notes.installationId));
    const refused = await readiness(database, notes.installationId);

    // satisfy the dependency installed at the universe, and refuse the one installed nowhere
    expect({ deployed, refused }).toEqual({
        deployed: ["current", "1 workloads deployed"],
        refused: [
            "failed",
            "@destack/stack DEPENDENCY_UNINSTALLED: notes needs @acme/vault installed",
        ],
    });
});

test("refuse deploying an application until its space installs its dependency, and refuse uninstalling the dependency while the application needs it", async () => {
    // install notes depending on a vault package its space lacks, published at the registry
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const vault = await fixture.registry.release(
        await MemoryBuild.of(VAULT, [], testHost().crypto),
    );
    const notes = await installNotes(fixture);
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(object.installationRevision.table)
                .set({ dependencies: [{ package: VAULT, manifest: Digest.parse(vault) }] })
                .where(eq(object.installationRevision.table.installationId, notes.installationId)),
    );
    const { deployments, instances, uninstall } = await serveControllers(fixture, {});

    // deploy without the vault, then once installing it wakes notes through the controllers' watches
    await refusal(deployments.reconcile(notes.installationId));
    const refused = await readiness(database, notes.installationId);
    const now = fixture.machine.clock.now();
    const installed = await insertRollout(
        database,
        {
            id: Identifier.create("installation", fixture.environment),
            scope: ids.space,
            packageId: VAULT.id,
            name: "vault",
            selection: { kind: "release", version: VAULT.version },
            installer: fixture.owner.subject,
        },
        now,
    );
    await submitRevision(
        fixture,
        installed.installationId,
        "f",
        await MemoryBuild.of(VAULT, [], testHost().crypto),
    );
    await instances.settle();
    const deployed = await readiness(database, notes.installationId);

    // uninstall the vault while notes needs it
    const uninstalled = await refusal(uninstall(installed.installationId));

    // refuse with the missing package's name, deploy once installed, and leave the vault installed
    expect({ refused, deployed, uninstalled }).toEqual({
        refused: [
            "failed",
            "@destack/stack DEPENDENCY_UNINSTALLED: notes needs @acme/vault installed",
        ],
        deployed: ["current", "1 workloads deployed"],
        uninstalled: [
            "PRECONDITION_FAILED",
            "NEEDED_BY_DEPENDENT",
            "cannot uninstall vault: notes needs it",
        ],
    });
});

test("deploy an application without a package only its optional capability needs, and refuse the capability once allowed until the space installs it", async () => {
    // install notes with an optional vault sync capability needing a vault package its space lacks
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const declared = requesting({});
    const request: Declared = {
        ...declared,
        description: {
            ...declared.description,
            capabilities: {
                "vault-sync": {
                    reason: "Keep notes in the vault",
                    optional: true,
                    packages: [VAULT.id],
                },
            },
        },
    };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, request], fixture.environment.crypto),
    );
    const vault = await fixture.registry.release(
        await MemoryBuild.of(VAULT, [], testHost().crypto),
    );
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(object.installationRevision.table)
                .set({ dependencies: [{ package: VAULT, manifest: Digest.parse(vault) }] })
                .where(eq(object.installationRevision.table.installationId, notes.installationId)),
    );
    const { deployments } = await serveControllers(fixture);

    // deploy without the capability, then with it allowed
    await deployments.reconcile(notes.installationId);
    const without = await readiness(database, notes.installationId);
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(installation.table)
                .set({ allowed: ["vault-sync"] })
                .where(eq(installation.table.id, notes.installationId)),
    );
    await refusal(deployments.reconcile(notes.installationId));
    const allowed = await readiness(database, notes.installationId);

    // deploy without the vault until the capability needs it
    expect({ without, allowed }).toEqual({
        without: ["current", "1 workloads deployed"],
        allowed: [
            "failed",
            "@destack/stack DEPENDENCY_UNINSTALLED: capability vault-sync needs @acme/vault installed",
        ],
    });
});

test("wait for a provider of a kind no provider here serves yet, naming the kind", async () => {
    // install notes binding a vault, whose provider no installation here serves yet
    const fixture = await TestSpace.open();
    const credentials: Declared = {
        definer: objectPackage,
        name: "credentials",
        description: { name: "credentials", kind: "vault", spec: {} },
    };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, credentials], fixture.environment.crypto),
    );
    const { deployments } = await serveControllers(fixture);

    // wait rather than fail, naming the kind
    await deployments.reconcile(notes.installationId);
    expect(await readiness(fixture.database, notes.installationId)).toEqual([
        "waiting",
        "waiting for a provider of vault resources",
    ]);
});

test("wait to revise an installation selecting an object of a kind no server here serves yet, naming the kind", async () => {
    // install notes needing a connection, and select an existing one before any server here serves connections
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const github: Declared = {
        definer: objectPackage,
        name: "github",
        description: { name: "github", kind: "connection", spec: {} },
    };
    const notes = await installNotes(
        fixture,
        ids.notes,
        await notesBuild([main, github], fixture.environment.crypto),
    );
    const { deployments } = await serveControllers(fixture);
    await selectBinding(database, notes.installationId, "github", {
        type: "connection",
        id: Identifier.create("connection", fixture.environment),
    });

    // wait rather than fail the revision, naming the kind
    await deployments.reconcile(notes.installationId);
    expect(await readiness(database, notes.installationId)).toEqual([
        "waiting",
        "waiting for a provider of connection resources",
    ]);
});

/** Read an installation's Ready condition, apart from when it last changed. */
function readyOf(installed: Awaited<ReturnType<typeof readInstalled>>) {
    const { lastTransitionAt: _changed, ...condition } = present(
        installed.installation.conditions["Ready"],
        "a Ready",
    );

    return condition;
}
