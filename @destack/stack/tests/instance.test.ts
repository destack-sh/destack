import { PACKAGE_KIND_PACKAGE_ID } from "@destack/runtime";
import { MemoryBuild } from "@destack/package/test";
import { Plan } from "@destack/resource";
import { ObjectReference, Scope } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { type DatabaseConnection, Snapshot, eq, Expression, DatabaseState } from "@destack/db";
import { Digest, nth, Identifier, present, schema } from "@destack/schema";
import { databaseProvider, type LogPosition } from "@destack/db";
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
    Caller,
    type PermissionRequest,
    principal,
} from "@destack/access";
import { space } from "@destack/account/object";
import { defineObject, type Field, field, type ObjectDefinition } from "@destack/object";
import type { ServerRuntime } from "@destack/package/runtime";
import type { BuildReader } from "@destack/package/manifest";
import * as object from "../src/object/index.ts";
import * as accountObject from "@destack/account/object";
import { Approval, serveSpace, type SpaceOptions } from "../src/server/index.ts";
import {
    fixtureBuild,
    fixtureOutputs,
    ids,
    openUniverse,
    placeSpace,
    reconcile,
    TestRuntime,
    settle,
    workload,
    reconcileResources,
    StackFixture,
    TestSpaceDatabaseHost,
} from "../src/test/index.ts";
import { InstallationBindingSpec } from "@destack/runtime/declare";
import { RequestId } from "@destack/runtime/request";
import { VirtualClock } from "@destack/environment/simulation";

/** The time the backoff scenario starts at, 09:00 UTC on 27 September 2026. */
const START = Date.UTC(2026, 8, 27, 9);

/** The notes package the scenarios install. */
const notesPackage = Package.parse({
    id: ids.package,
    name: "@example/notes",
    version: "2026.9.0",
});

/** The package defining object types. */
const objectPackage = Package.parse({
    id: "package-01996ab0-0000-7000-8000-00000000000d",
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

/** The package defining resource kinds and triggers. */
const runtimePackage = Package.parse({
    id: PACKAGE_KIND_PACKAGE_ID,
    name: "@destack/runtime",
    version: "2026.9.0",
});

/** A declaration of the notes build, with the package defining its kind. */
interface Declared {
    /** The declaration kind, a resource when absent. */
    readonly kind?: "resource" | "package" | "trigger" | "object";
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
    const { name, definition } = note.describe();
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
    definer: runtimePackage,
    name: "tasks",
    description: {
        name: "tasks",
        kind: "installation",
        spec: { package: ids.package },
    },
};

/** The notes package's declaration, requesting permissions by scope. */
function requesting(permissions: PermissionRequest): Declared {
    return {
        kind: "package",
        definer: runtimePackage,
        name: "notes",
        description: {
            api: { name: "notes", procedures: [] },
            objects: {},
            adapters: {},
            views: [],
            permissions,
            capabilities: {},
        },
    };
}

/** Write the notes build: its main workload in bun and workerd outputs, and a home view. */
async function notesBuild(
    declared: readonly Declared[],
    sandbox: Sandbox = {},
): Promise<BuildReader> {
    // keep the workload using every declared resource in both server outputs
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
        workloads: { main: used },
        views:
            browser.length === 0
                ? {}
                : {
                      home: {
                          entrypoint: "view.js",
                          sandbox: Sandbox.select(sandbox, browser),
                          presents: [],
                          packages: [],
                      },
                  },
    });

    // declare each resource, package and trigger as its definer's inspector records it
    const built = await MemoryBuild.of(
        notesPackage,
        declared.map((entry) => ({
            kind: entry.kind ?? "resource",
            kindPackageId: entry.definer.id,
            module: "src/resource.ts",
            name: entry.name,
            description: entry.description,
        })),
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
    fixture: StackFixture,
    id: Identifier<"installation"> = ids.notes,
    build?: BuildReader,
) {
    const { database } = fixture;
    const now = Date.now();
    const notes = nth(
        await database
            .insert(object.installation.table)
            .values({
                scope: ids.space,
                createdAt: now,
                updatedAt: now,
                id,
                packageId: ids.package,
                role: "application",
                selection,
                name: "notes",
            })
            .returning(),
        0,
    );
    await submitRevision(fixture, id, "e", build);

    return notes;
}

/** Retain and follow a revision of a checkout build digested from one repeated character. */
async function submitRevision(
    fixture: StackFixture,
    installationId: Identifier<"installation">,
    character: string,
    build?: BuildReader,
) {
    const { database } = fixture;
    const now = Date.now();
    const reader = build ?? (await notesBuild([main]));
    const built = await fixture.recordBuild(reader);
    const revision = nth(
        await database
            .insert(object.installationRevision.table)
            .values({
                scope: ids.space,
                createdAt: now,
                updatedAt: now,
                id: Identifier.create("installation-revision"),
                installationId,
                buildId: built.build,
                dependencies: [],
                digest: character.repeat(64),
            })
            .returning(),
        0,
    );
    await database
        .update(object.installation.table)
        .set({ revisionId: revision.id })
        .where(eq(object.installation.table.id, installationId));

    return revision;
}

/** Serve a database's deployments, instances and resources through the controllers of a machine. */
async function serveControllers(
    fixture: StackFixture,
    options: {
        readonly runtimes?: readonly ServerRuntime[];
        readonly runtime?: TestRuntime;
        readonly opened?: Awaited<ReturnType<typeof openUniverse>>;
        readonly accounts?: SpaceOptions["accounts"];
        readonly clock?: SpaceOptions["clock"];
        readonly isEnforcingEntitlements?: boolean;
    } = {},
) {
    // provision the machine's databases as files in a temporary directory
    const { database } = fixture;
    const directory = await mkdtemp(join(tmpdir(), "destack-instance-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    const root = pathToFileURL(`${directory}/`);
    const runtime = options.runtime ?? new TestRuntime();
    const runtimes = options.runtimes ?? ["bun"];

    // run the chosen runtimes on the machine serving the space and record instances on the one test runtime
    await placeSpace(database, ids.space, ids.machine);
    const served = serveSpace(
        await fixture.options({
            machine: ids.machine,
            providers: [
                databaseProvider(new BunSqliteDatabaseHost(root), object.database),
                databaseProvider(new TestSpaceDatabaseHost(root), object.database),
            ],
            ...options.opened,
            ...(options.accounts === undefined ? {} : { accounts: options.accounts }),
            ...(options.clock === undefined ? {} : { clock: options.clock }),
            ...(options.isEnforcingEntitlements === true
                ? { isEnforcingEntitlements: true, publisherAt: () => TestEntitlement.publisher }
                : {}),
            runtimes: runtimes.map((name) =>
                name === runtime.name ? runtime : new TestRuntime(name),
            ),
        }),
    );
    const { store } = served;

    return {
        root,
        runtime,
        authorizer: store.authorizer,
        deployments: {
            reconcile: async (id: string) => {
                // deploy, apply the space's resources, and deploy again once the space serves the revision
                await reconcile(store, "installation", { id });
                await reconcileResources(store, ids.space);

                return reconcile(store, "installation", { id });
            },
            deploy: (id: string) => reconcile(store, "installation", { id }),
        },
        instances: {
            reconcile: (id: string) => reconcile(store, "deployment", { id }),
            settle: (after: LogPosition) =>
                settle(
                    database,
                    [
                        ...store.controllers(),
                        // write main's desired states, as the space's consumer controller does
                        ...(nth(served.packages, 0).controllers ?? []).filter(
                            (controller) => controller.name === "consumer",
                        ),
                    ],
                    after,
                ),
        },
        resources: {
            reconcile: (scope: string) => reconcileResources(store, scope),
        },
        approve: (id: Identifier<"installation">) => approvePlan(database, store, id),
        uninstall: (id: Identifier<"installation">) =>
            store.mutate(store.serving(object.installation), {
                id: RequestId.create(Date.now()),
                calls: [
                    object.installation.call(
                        "delete",
                        object.installation.target({ scope: ids.space, id }),
                    ),
                ],
            }),
    };
}

/** Read an installation's ready condition as its reason and message. */
async function readiness(database: DatabaseConnection, id: Identifier<"installation">) {
    const [row] = await database
        .select({ conditions: object.installation.table.conditions })
        .from(object.installation.table)
        .where(eq(object.installation.table.id, id));
    const condition = present(row, "the installation").conditions["ready"];

    return [condition?.reason, condition?.message];
}

/** Approve the plan an installation waits on as it stands. */
async function approvePlan(
    database: DatabaseConnection,
    store: ReturnType<typeof serveSpace>["store"],
    id: Identifier<"installation">,
) {
    // read the plan the installation waits on
    const target = nth(
        await database
            .select()
            .from(object.installation.table)
            .where(eq(object.installation.table.id, id)),
        0,
    );
    const resources = [object.database, object.dependency];
    const approval = await Approval.of(Snapshot.live(database), resources, target);
    const digest = await Plan.digest(approval.plan());

    // approve it as the system
    const [granted] = await store.mutate(store.serving(object.installation), {
        id: RequestId.create(Date.now()),
        calls: [
            object.installation.call("approve", {
                ...object.installation.target(target),
                plan: digest,
            }),
        ],
    });

    return granted;
}

test("deploy each workload with its bound resources, and drain and retire it once suspended", async () => {
    // install notes, whose notes the space's main database keeps
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const notes = await installNotes(fixture);

    // deploy it on a bun machine, apply main, and run its instance on this machine
    const { runtime, deployments, instances, resources } = await serveControllers(fixture);
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const deployed = await database.select().from(object.deployment.table);
    await instances.reconcile(nth(deployed, 0).id);
    const running = nth(await database.select().from(object.instance.table), 0);
    const installed = nth(
        await database
            .select()
            .from(object.installation.table)
            .where(eq(object.installation.table.id, notes.id)),
        0,
    );
    const captured = await database
        .select({ name: object.capture.table.name })
        .from(object.capture.table);
    const run = nth(runtime.started, 0);

    // deploy only the bun output and run it without its main database, which only its deployment captures
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
        captured,
        applied: installed.appliedRevisionId === installed.revisionId,
    }).toEqual({
        deployments: [["active", "server", "bun", "main", mainWorkload]],
        instance: ["running", ids.machine],
        run: [running.id, nth(deployed, 0).id, ids.space, "server", "main"],
        resources: [],
        captured: [{ name: "main" }],
        applied: true,
    });

    // suspend the application
    await database
        .update(object.installation.table)
        .set({ status: "suspended" })
        .where(eq(object.installation.table.id, notes.id));
    await deployments.reconcile(notes.id);
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

test("bind an application's tables to the space's main database, start its instance once main applies them, and keep them for approval once it uninstalls", async () => {
    // install notes without a stack and deploy it
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const notes = await installNotes(fixture);
    const { runtime, deployments, instances, resources } = await serveControllers(fixture);
    await deployments.deploy(notes.id);

    // activate nothing while main has not applied the notes' tables the space would serve
    const mainDatabase = nth(await database.select().from(object.database.table), 0);
    const [waiting] = await database
        .select()
        .from(object.installation.table)
        .where(eq(object.installation.table.id, notes.id));
    expect({
        started: runtime.started.length,
        deployments: await database.select().from(object.deployment.table),
        ready: [waiting?.conditions["ready"]?.reason, waiting?.conditions["ready"]?.message],
    }).toEqual({
        started: 0,
        deployments: [],
        ready: ["WaitingForSpace", `waiting for the space to serve revision ${waiting?.revisionId}`],
    });

    // apply main, deploy once the space serves the revision, and start the instance
    const provisioned = await database.log.position();
    await resources.reconcile(ids.space);
    await deployments.deploy(notes.id);
    const deployed = nth(await database.select().from(object.deployment.table), 0);
    await instances.settle(provisioned);
    const applied = nth(await database.select().from(object.database.table), 0);
    const attached = nth(await database.select().from(object.binding.table), 0);
    const running = nth(await database.select().from(object.instance.table), 0);
    const bound = DatabaseState.parse(attached.state);
    expect({
        main: [applied.name, applied.ownerId, applied.conditions["ready"]?.reason],
        binding: [attached.installationId, attached.name, attached.target],
        tables: bound.tables.sqlite.map((state) => state.table.name),
        instance: [running.deploymentId, running.status],
        resources: present(runtime.started.at(-1), "the started run").resources,
    }).toEqual({
        main: ["main", null, "Applied"],
        binding: [notes.id, "main", mainDatabase.id],
        tables: ["example__notes__note"],
        instance: [deployed.id, "running"],
        resources: [],
    });

    // uninstall notes: drain its deployment, remove it, and wait for approval to drop its tables
    await database
        .update(object.installation.table)
        .set({ deletedAt: Date.now() })
        .where(eq(object.installation.table.id, notes.id));
    await deployments.reconcile(notes.id);
    await instances.reconcile(deployed.id);
    await instances.reconcile(deployed.id);
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const retained = nth(await database.select().from(object.database.table), 0);
    expect({
        installations: await database.select().from(object.installation.table),
        bindings: await database.select().from(object.binding.table),
        main: [
            retained.conditions["ready"]?.reason,
            retained.plan?.steps.map((step) => [step.action, step.risk, step.target]) ?? null,
        ],
    }).toEqual({
        installations: [],
        bindings: [],
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

test("restart a failing instance in place after a doubling backoff, keeping one record with its last failure", async () => {
    // deploy notes with an applied database on a machine with a failing runner
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const notes = await installNotes(fixture);
    const failing = new FailingRuntime();
    const clock = new VirtualClock(START);
    const { deployments, instances, resources } = await serveControllers(fixture, {
        runtime: failing,
        clock,
    });
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const deployed = nth(await database.select().from(object.deployment.table), 0);

    // fail, wait out the backoff and restart
    const first = await instances.reconcile(deployed.id);
    const early = await instances.reconcile(deployed.id);
    await database
        .update(object.instance.table)
        .set({ stoppedAt: START - 1000 })
        .where(eq(object.instance.table.deploymentId, deployed.id));
    const second = await instances.reconcile(deployed.id);
    const recorded = await database.select().from(object.instance.table);
    expect({
        delays: [first, early, second],
        attempts: failing.attempts,
        instances: recorded.map((entry) => [
            entry.status,
            entry.restarts,
            entry.conditions["ready"]?.status,
            entry.conditions["ready"]?.reason,
            entry.conditions["ready"]?.message,
        ]),
    }).toEqual({
        delays: [1000, 1000, 2000],
        attempts: 2,
        instances: [["failed", 1, "false", "StartFailed", "runner exited before serving"]],
    });
});

test("start a running instance again once its machine restarted and no longer runs its process", async () => {
    // run notes with an applied database
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const notes = await installNotes(fixture);
    const before = new TestRuntime();
    const first = await serveControllers(fixture, { runtime: before });
    await first.deployments.reconcile(notes.id);
    await first.resources.reconcile(ids.space);
    const deployed = nth(await database.select().from(object.deployment.table), 0);
    await first.instances.reconcile(deployed.id);

    // restart the machine with an idle runtime and reconcile again
    const after = new TestRuntime();
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

test("deploy a submitted revision beside the previous one, keep the previous one serving until the new one runs, then retire it", async () => {
    // run the first revision of notes with its applied database
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const notes = await installNotes(fixture);
    const { deployments, instances, resources } = await serveControllers(fixture);
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const first = nth(await database.select().from(object.deployment.table), 0);
    await instances.reconcile(first.id);

    // submit a second revision that deploys beside the first and drains it
    const second = await submitRevision(fixture, notes.id, "f");
    await deployments.reconcile(notes.id);
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
    await deployments.reconcile(notes.id);
    const deployed = await database
        .select()
        .from(object.deployment.table)
        .orderBy(object.deployment.table.createdAt, object.deployment.table.id);
    const recorded = await database
        .select()
        .from(object.instance.table)
        .orderBy(object.instance.table.createdAt, object.instance.table.id);
    const installed = nth(
        await database
            .select()
            .from(object.installation.table)
            .where(eq(object.installation.table.id, notes.id)),
        0,
    );
    // keep the first revision serving until its replacement runs
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
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const notes = await installNotes(fixture);
    const { deployments: workerd } = await serveControllers(fixture, { runtimes: ["workerd"] });
    await workerd.reconcile(notes.id);

    // serve it on a bun machine to replace the workerd deployment
    const { deployments: bun } = await serveControllers(fixture, { runtimes: ["bun"] });
    await bun.reconcile(notes.id);
    await bun.reconcile(notes.id);
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
    // deploy notes with the database it owns
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const notes = await installNotes(fixture);
    const { deployments, resources } = await serveControllers(fixture);
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const owned = nth(await database.select().from(object.database.table), 0);

    // rebind main to a second database of the same declaration
    const now = Date.now();
    const second = nth(
        await database
            .insert(object.database.table)
            .values({
                ...owned,
                id: Identifier.create("database"),
                name: "notes.archive",
                ...ObjectReference.columns("owner", undefined),
                createdAt: now,
                updatedAt: now,
            })
            .returning(),
        0,
    );
    await database
        .update(object.binding.table)
        .set({ target: second.id })
        .where(eq(object.binding.table.installationId, notes.id));

    // deploy again with the new target and drain the deployment that captured the old one
    await deployments.reconcile(notes.id);
    const deployed = await database
        .select()
        .from(object.deployment.table)
        .orderBy(object.deployment.table.createdAt, object.deployment.table.id);
    const captured = await database
        .select()
        .from(object.capture.table)
        .orderBy(object.capture.table.createdAt, object.capture.table.id);
    expect({
        deployments: deployed.map((entry) => entry.status),
        captures: captured.map((entry) => [entry.deploymentId, entry.target]),
    }).toEqual({
        deployments: ["draining", "active"],
        captures: [
            [nth(deployed, 0).id, owned.id],
            [nth(deployed, 1).id, second.id],
        ],
    });
});

test("deploy nothing of a build whose workloads have no output on a runtime the machine runs, and report why", async () => {
    // install notes with its workload only in a bun output
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const notes = await installNotes(
        fixture,
        ids.notes,
        await fixtureBuild(ids.package, "2026.9.0"),
    );
    const { deployments } = await serveControllers(fixture, {
        runtimes: ["workerd"],
    });

    // deploy it on a workerd machine
    await deployments.reconcile(notes.id);
    const installed = nth(
        await database
            .select()
            .from(object.installation.table)
            .where(eq(object.installation.table.id, notes.id)),
        0,
    );
    expect({
        deployments: await database.select().from(object.deployment.table),
        ready: installed.conditions["ready"],
        applied: installed.appliedRevisionId,
    }).toEqual({
        deployments: [],
        ready: {
            status: "false",
            reason: "NoRuntime",
            message: "workloads main have no output on workerd",
            observedGeneration: installed.generation,
            lastTransitionAt: installed.conditions["ready"]?.lastTransitionAt,
        },
        applied: null,
    });
});

test("deploy nothing of a build declaring its own database, and report why", async () => {
    // install notes declaring a database beside the space's main database
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const archive: Declared = {
        definer: runtimePackage,
        name: "archive",
        description: { name: "archive", kind: "database", spec: {} },
    };
    const notes = await installNotes(fixture, ids.notes, await notesBuild([main, archive]));
    const { deployments } = await serveControllers(fixture);

    // deploy it
    await deployments.reconcile(notes.id);
    const installed = nth(
        await database
            .select()
            .from(object.installation.table)
            .where(eq(object.installation.table.id, notes.id)),
        0,
    );
    expect({
        deployments: await database.select().from(object.deployment.table),
        ready: installed.conditions["ready"],
        applied: installed.appliedRevisionId,
    }).toEqual({
        deployments: [],
        ready: {
            status: "false",
            reason: "DeclaredDatabase",
            message: `${ids.package} declares the databases archive: the space's main database keeps every installation's tables`,
            observedGeneration: installed.generation,
            lastTransitionAt: installed.conditions["ready"]?.lastTransitionAt,
        },
        applied: null,
    });
});

test("run each workload with its build's sandbox, what its allowed capabilities unlock, and its directories", async () => {
    // install notes whose workload runs git, and whose optional capabilities call an issue tracker, index chosen directories or scan receipts
    const fixture = await StackFixture.open();
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
                    sandbox: { fs: { access: "read" } },
                },
                scanning: { reason: "Scan receipts", optional: true, sandbox: { camera: {} } },
            },
        },
    };
    const notes = await installNotes(fixture, ids.notes, await notesBuild([main, request], base));
    const { runtime, deployments, instances, resources } = await serveControllers(fixture);
    const notesDirectory = { path: "/Users/person/Notes", access: "read" as const };
    const grant = async (
        capabilities: readonly string[],
        directories: readonly DirectoryGrant[] = [],
    ) => {
        await database
            .update(object.installation.table)
            .set({ capabilities: [...capabilities], directories: [...directories] })
            .where(eq(object.installation.table.id, notes.id));
        await deployments.reconcile(notes.id);
    };
    const deployed = async () =>
        (
            await database
                .select()
                .from(object.deployment.table)
                .orderBy(object.deployment.table.createdAt, object.deployment.table.id)
        ).map((entry) => [entry.status, entry.description.sandbox, entry.directories]);

    // refuse allowing an undeclared capability, a directory without fs, and writing a directory fs reads
    await grant(["microphone"]);
    const undeclared = await readiness(database, notes.id);
    await grant([], [notesDirectory]);
    const unreadable = await readiness(database, notes.id);
    await grant(["index"], [{ ...notesDirectory, access: "write" }]);
    const overreaching = await readiness(database, notes.id);
    const refusedDeployments = await deployed();

    // deploy with the build's sandbox alone
    await grant([]);
    await resources.reconcile(ids.space);
    const first = nth(await database.select().from(object.deployment.table), 0);
    await instances.reconcile(first.id);
    const unallowed = await deployed();

    // allow the issues, indexing and scanning, and redeploy the workload with their host abilities and the directory
    await grant(["issues", "index", "scanning"], [notesDirectory]);
    const allowed = await deployed();
    expect({
        undeclared,
        unreadable,
        overreaching,
        refusedDeployments,
        started: runtime.started.map((started) => [started.sandbox, started.directories]),
        unallowed,
        allowed,
    }).toEqual({
        undeclared: [
            "UndeclaredCapability",
            "notes 2026.9.0 declares no capabilities microphone to allow",
        ],
        unreadable: [
            "UndeclaredCapability",
            "notes 2026.9.0 enables no fs ability to grant directories /Users/person/Notes",
        ],
        overreaching: [
            "UndeclaredCapability",
            "notes 2026.9.0 enables fs for reading only, refusing to grant writing /Users/person/Notes",
        ],
        refusedDeployments: [],
        started: [[base, []]],
        unallowed: [["active", base, []]],
        allowed: [
            ["draining", base, []],
            [
                "active",
                { ...base, network: { connect: ["api.github.com"] }, fs: { access: "read" } },
                [notesDirectory],
            ],
        ],
    });
});

test("fail an instance granted fs on a runtime without a filesystem to grant", async () => {
    // deploy notes requiring fs on a machine running workerd only
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const fs = Sandbox.parse({ fs: { access: "read" } });
    const notes = await installNotes(fixture, ids.notes, await notesBuild([main], fs));
    const { deployments, instances, resources } = await serveControllers(fixture, {
        runtimes: ["workerd"],
        runtime: new TestRuntime("workerd"),
    });
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const deployed = nth(await database.select().from(object.deployment.table), 0);

    // record the refusal on the instance
    await instances.reconcile(deployed.id);
    const [recorded] = await database.select().from(object.instance.table);
    expect([
        recorded?.status,
        recorded?.conditions["ready"]?.reason,
        recorded?.conditions["ready"]?.message,
    ]).toEqual(["failed", "StartFailed", "the workerd runtime has no filesystem to grant"]);
});

test("refuse to deploy an installation binding without an address, then run the workload with the address its stack declares", async () => {
    // install notes calling the tasks installation besides its main database bound to a provisioned database
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const notes = await installNotes(fixture, ids.notes, await notesBuild([main, tasks]));
    const { runtime, deployments, instances, resources } = await serveControllers(fixture);

    // refuse to deploy while no stack binds the tasks installation to an address
    await deployments.reconcile(notes.id);
    const condition = async () =>
        nth(
            await database
                .select({ conditions: object.installation.table.conditions })
                .from(object.installation.table)
                .where(eq(object.installation.table.id, notes.id)),
            0,
        ).conditions["ready"];
    const refused = await condition();

    // declare the tasks dependency at the address of the notes installation itself and bind it
    const now = Date.now();
    const record = { scope: ids.space, createdAt: now, updatedAt: now };
    const declared = {
        ...record,
        id: schema
            .identifier("dependency")
            .parse("dependency-01996ab0-0000-7000-8000-000000000021"),
        name: "tasks",
        definitionPackageId: runtimePackage.id,
        definitionVersion: "2026.9.0",
        definitionName: "tasks",
        spec: InstallationBindingSpec.parse(tasks.description["spec"]),
        origin: "declared" as const,
        provider: "http",
        reference: "notes",
    };
    await database.insert(object.dependency.table).values(declared);
    await database.insert(object.binding.table).values({
        ...record,
        id: schema.identifier("binding").parse("binding-01996ab0-0000-7000-8000-000000000022"),
        installationId: notes.id,
        packageId: ids.package,
        name: "tasks",
        target: declared.id,
        state: {},
    });

    // observe it ready as declared and deploy and run with the database the installation owns
    await resources.reconcile(ids.space);
    const observed = nth(
        await database
            .select({ conditions: object.dependency.table.conditions })
            .from(object.dependency.table)
            .where(eq(object.dependency.table.id, declared.id)),
        0,
    );
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const deployed = nth(await database.select().from(object.deployment.table), 0);
    await instances.reconcile(deployed.id);
    const dependency = runtime.started[0]?.resources.find((entry) => entry.kind === "installation");

    // report the missing address and run with the declared address and its connecting provider
    expect({
        refused: [refused?.status, refused?.reason, refused?.message],
        declared: [observed.conditions["ready"]?.status, observed.conditions["ready"]?.reason],
        dependency: [dependency?.name, dependency?.reference, dependency?.provider],
    }).toEqual({
        refused: [
            "false",
            "MissingBinding",
            `bind installations ${ids.package} tasks to their addresses`,
        ],
        declared: ["true", "Declared"],
        dependency: ["tasks", "notes", "http"],
    });
});

test("bind a workload to an installation another space grants", async () => {
    // install notes bound to the board installation of another space, by its name there
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const notes = await installNotes(fixture, ids.notes, await notesBuild([main, tasks]));
    const { runtime, deployments, instances, resources } = await serveControllers(fixture);
    const acme = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000031");
    const board = "board";
    const now = Date.now();
    await database.insert(object.binding.table).values({
        scope: ids.space,
        createdAt: now,
        updatedAt: now,
        id: schema.identifier("binding").parse("binding-01996ab0-0000-7000-8000-000000000033"),
        installationId: notes.id,
        packageId: ids.package,
        name: "tasks",
        target: board,
        targetSpaceId: acme,
        state: {},
    });

    // deploy it and run its instance
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const deployed = nth(await database.select().from(object.deployment.table), 0);
    await instances.reconcile(deployed.id);
    const captured = await database
        .select({
            name: object.capture.table.name,
            target: object.capture.table.target,
            targetSpaceId: object.capture.table.targetSpaceId,
            version: object.capture.table.version,
        })
        .from(object.capture.table);

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

test("keep the columns a draining release still declares, and drop them for approval once it retires", async () => {
    // run the first revision of notes whose notes carry a color
    const fixture = await StackFixture.open();
    const { database } = fixture;
    let declared: readonly Declared[] = [
        noteType({ body: field.string().optional(), color: field.string().optional() }),
    ];
    const notes = await installNotes(fixture, ids.notes, await notesBuild(declared));
    const { deployments, instances, resources } = await serveControllers(fixture);
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const first = nth(await database.select().from(object.deployment.table), 0);
    await instances.reconcile(first.id);

    // deploy a second revision dropping the color beside the first and plan while the first drains
    declared = [main];
    const second = await submitRevision(fixture, notes.id, "f", await notesBuild(declared));
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const plan = async () => {
        const row = nth(await database.select().from(object.database.table), 0);
        return [
            row.conditions["ready"]?.reason,
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

    // keep the color while the first release serves and wait for approval to drop it
    expect({ whileDraining, retired: await plan() }).toEqual({
        whileDraining: ["Applied", null],
        retired: ["AwaitingApproval", [["replace", "destructive", "rebuild table: drop color"]]],
    });
});

test("recreate over a draining release whose values a conversion narrows: stop it once approved, convert, then run the replacement", async () => {
    // run the first revision of notes whose modes include emacs
    const fixture = await StackFixture.open();
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
    const notes = await installNotes(fixture, ids.notes, await notesBuild(declared));
    const { deployments, instances, resources, approve } = await serveControllers(fixture);
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const original = nth(await database.select().from(object.deployment.table), 0);
    await instances.reconcile(original.id);

    // deploy the narrowing revision beside it and plan a recreation that waits for approval
    declared = [narrowed];
    const second = await submitRevision(fixture, notes.id, "f", await notesBuild(declared));
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const read = async () => {
        const row = nth(await database.select().from(object.database.table), 0);
        return {
            reason: row.conditions["ready"]?.reason,
            plan: row.plan ?? undefined,
            id: row.id,
        };
    };
    const waiting = await read();

    // approve it through notes and replace the first release
    const approved = await approve(notes.id);
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

test("keep a release serving while a caller pins it, and retire it once the caller retires", async () => {
    // run the first revision of notes
    const fixture = await StackFixture.open();
    const { database } = fixture;
    let declared: readonly Declared[] = [main];
    const notes = await installNotes(fixture, ids.notes, await notesBuild(declared));
    const { deployments, instances, resources } = await serveControllers(fixture);
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const first = nth(await database.select().from(object.deployment.table), 0);
    await instances.reconcile(first.id);

    // bind a caller's active deployment to notes and pin the first release
    const now = Date.now();
    const record = { scope: ids.space, createdAt: now, updatedAt: now };
    const dependency = {
        ...record,
        id: schema
            .identifier("dependency")
            .parse("dependency-01996ab0-0000-7000-8000-000000000031"),
        name: "calls.notes",
        definitionPackageId: runtimePackage.id,
        definitionVersion: "2026.9.0",
        definitionName: "notes",
        spec: { package: ids.package },
        origin: "declared" as const,
        provider: "http",
        reference: "notes",
    };
    const caller = {
        ...record,
        id: schema
            .identifier("deployment")
            .parse("deployment-01996ab0-0000-7000-8000-000000000032"),
        installationId: first.installationId,
        packageId: ids.package,
        revisionId: first.revisionId,
        output: "server",
        workload: "caller",
        runtime: "bun" as const,
        release: "2026.9.0",
        description: first.description,
        status: "active" as const,
        activatedAt: first.activatedAt,
    };
    await database.insert(object.dependency.table).values(dependency);
    await database.insert(object.deployment.table).values(caller);
    await database.insert(object.capture.table).values({
        ...record,
        id: schema.identifier("capture").parse("capture-01996ab0-0000-7000-8000-000000000033"),
        deploymentId: caller.id,
        packageId: ids.package,
        name: "notes",
        target: dependency.id,
        version: 1,
        state: { release: "2026.9.0" },
    });

    // deploy a second revision whose package serves only callers since 2026.10.0 and run it
    declared = [
        main,
        {
            kind: "package",
            definer: runtimePackage,
            name: "notes",
            description: {
                since: "2026.10.0",
                api: { name: "notes", procedures: [] },
                objects: {},
                adapters: {},
                views: [],
                permissions: {},
                capabilities: {},
            },
        },
    ];
    const second = await submitRevision(fixture, notes.id, "f", await notesBuild(declared));
    await deployments.reconcile(notes.id);
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

    // retire the caller and its pin and stop and retire the first release as its retirement and its instance wake it
    await database
        .update(object.deployment.table)
        .set({ status: "draining" })
        .where(eq(object.deployment.table.id, caller.id));
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

test("grant an applied application exactly the space permissions its package requests, its space's grant beyond the space, and drop both once it is uninstalled", async () => {
    // install notes requesting reading the space and reading where people receive notifications
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const recipient = accountObject.user.permission("recipient");
    const request = requesting({
        installation: { space: [space.permission("read")], universe: [recipient] },
    });
    const notes = await installNotes(fixture, ids.notes, await notesBuild([main, request]));

    // apply it on a bun machine
    const opened = await openUniverse();
    const { authorizer, deployments, instances } = await serveControllers(fixture, { opened });
    await deployments.reconcile(notes.id);
    const deployed = nth(await database.select().from(object.deployment.table), 0);

    // decide the installation's reading and managing of its space, and read the space's grants
    const spaceObject = await Scope.object(Snapshot.live(database), ids.space);
    const installed = principal.installation.reference(ids.space, notes.id);
    const decide = async () => {
        const snapshot = Snapshot.live(database);
        const access = await authorizer.resolve(
            snapshot,
            ids.space,
            AccessContext.of(Caller.of(installed), Date.now()),
        );

        return Promise.all(
            [space.permission("read"), space.permission("manage")].map(
                async (permission) =>
                    (await authorizer.check(snapshot, permission, spaceObject, access)).isAllowed,
            ),
        );
    };
    const grants = async () =>
        (await database.select().from(accessGrant)).map((row) => ({
            delegator: ObjectReference.at("delegator", row),
            delegate: ObjectReference.at("delegate", row),
            permissions: row.permissions,
        }));
    const granted = { space: await decide(), grants: await grants() };

    // uninstall it
    await database
        .update(object.installation.table)
        .set({ deletedAt: Date.now() })
        .where(eq(object.installation.table.id, notes.id));
    await deployments.reconcile(notes.id);
    await instances.reconcile(deployed.id);
    await deployments.reconcile(notes.id);
    const roles = await database.select({ name: accessRole.name }).from(accessRole);
    expect({
        granted,
        dropped: { space: await decide(), grants: await grants() },
        roles: roles.filter((role) => role.name.startsWith("installation/")),
    }).toEqual({
        granted: {
            space: [true, false],
            grants: [
                {
                    delegator: principal.space.reference(Scope.universe.id, ids.space),
                    delegate: installed,
                    permissions: [{ ...recipient }],
                },
            ],
        },
        dropped: { space: [false, false], grants: [] },
        roles: [],
    });
});

test("let an application ask for its optional capability in approval mode, and grant the capability's permissions once allowed", async () => {
    // install notes declaring an optional push capability needing to unsubscribe people's browsers
    const fixture = await StackFixture.open();
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
                    permissions: { installation: { universe: [unsubscribe] } },
                },
            },
        },
    };
    const notes = await installNotes(fixture, ids.notes, await notesBuild([main, request]));
    const opened = await openUniverse();
    const { deployments } = await serveControllers(fixture, { opened });
    const installed = principal.installation.reference(ids.space, notes.id);
    const grants = async () =>
        (await database.select().from(accessGrant)).map((row) => row.permissions);

    // deploy it, then allow push as its space would once it approves the request
    await deployments.reconcile(notes.id);
    const asking = await grants();
    await database
        .update(object.installation.table)
        .set({ allowed: ["push"] })
        .where(eq(object.installation.table.id, notes.id));
    await deployments.reconcile(notes.id);
    const enabled = (
        await database
            .select({
                enabled: object.deployment.table.enabled,
                status: object.deployment.table.status,
            })
            .from(object.deployment.table)
    ).toSorted((left, right) => left.status.localeCompare(right.status));

    // ask for push in approval mode on its own row, then grant push's permissions and run with it
    const allow = {
        ...object.installation.permission("allow"),
        scope: ids.space,
        objectId: notes.id,
        mode: "approval",
    };
    expect({ installed: installed.id, asking, allowed: await grants(), enabled }).toEqual({
        installed: notes.id,
        asking: [[allow]],
        allowed: [[{ ...unsubscribe }, allow]],
        enabled: [
            { enabled: ["push"], status: "active" },
            { enabled: [], status: "draining" },
        ],
    });
});

test("refuse deploying a capability whose feature the account lacks, and deploy it once the account is entitled", async () => {
    // install notes requiring a sync capability that needs the plan's sync feature
    const fixture = await StackFixture.open();
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
    const notes = await installNotes(fixture, ids.notes, await notesBuild([main, request]));
    const opened = await openUniverse();
    const { deployments } = await serveControllers(fixture, {
        opened,
        isEnforcingEntitlements: true,
    });
    const deployed = async () =>
        (
            await database
                .select({ enabled: object.deployment.table.enabled })
                .from(object.deployment.table)
        ).map((row) => row.enabled);

    // deploy with the account replicated without the feature, then once it is entitled to it
    await TestEntitlement.set(database, ids.account, "database", { kind: "boolean" });
    await deployments.reconcile(notes.id);
    const refused = { ready: await readiness(database, notes.id), deployed: await deployed() };
    await TestEntitlement.set(database, ids.account, { feature }, { kind: "boolean" });
    await deployments.reconcile(notes.id);

    // refuse naming the missing feature, then run with the capability
    expect({ refused, entitled: await deployed() }).toEqual({
        refused: {
            ready: [
                "UnentitledCapability",
                "capability sync needs sync: no plan of the account grants feature sync",
            ],
            deployed: [],
        },
        entitled: [["sync"]],
    });
});

test("refuse deploying an application until its space installs its dependency, and refuse uninstalling the dependency while the application needs it", async () => {
    // install notes depending on a vault package its space lacks, published at the registry
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const vault = await fixture.registry.push(await MemoryBuild.of(VAULT, []));
    const notes = await installNotes(fixture);
    await database
        .update(object.installationRevision.table)
        .set({ dependencies: [{ package: VAULT, manifest: Digest.parse(vault) }] })
        .where(eq(object.installationRevision.table.installationId, notes.id));
    const { deployments, instances, uninstall } = await serveControllers(fixture, {
        opened: await openUniverse(),
    });

    // deploy without the vault, then once installing it wakes notes through the controllers' watches
    await deployments.reconcile(notes.id);
    const refused = await readiness(database, notes.id);
    const before = await database.log.position();
    const now = Date.now();
    const installed = nth(
        await database
            .insert(object.installation.table)
            .values({
                scope: ids.space,
                createdAt: now,
                updatedAt: now,
                id: Identifier.create("installation"),
                packageId: VAULT.id,
                role: "application",
                selection: { kind: "release", version: VAULT.version },
                name: "vault",
            })
            .returning(),
        0,
    );
    await submitRevision(fixture, installed.id, "f", await MemoryBuild.of(VAULT, []));
    await instances.settle(before);
    const deployed = await readiness(database, notes.id);

    // uninstall the vault while notes needs it
    const uninstalled = await refusal(uninstall(installed.id));

    // refuse naming the missing package, deploy once installed, and keep the vault installed
    expect({ refused, deployed, uninstalled }).toEqual({
        refused: ["UninstalledDependency", "notes needs @acme/vault installed"],
        deployed: ["Deployed", "1 workloads deployed"],
        uninstalled: [
            "PRECONDITION_FAILED",
            "notes needs vault, which stays installed while it does",
        ],
    });
});

test("deploy an application without a package only its optional capability needs, and refuse the capability once allowed until the space installs it", async () => {
    // install notes with an optional vault sync capability needing a vault package its space lacks
    const fixture = await StackFixture.open();
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
    const notes = await installNotes(fixture, ids.notes, await notesBuild([main, request]));
    const vault = await fixture.registry.push(await MemoryBuild.of(VAULT, []));
    await database
        .update(object.installationRevision.table)
        .set({ dependencies: [{ package: VAULT, manifest: Digest.parse(vault) }] })
        .where(eq(object.installationRevision.table.installationId, notes.id));
    const { deployments } = await serveControllers(fixture, { opened: await openUniverse() });

    // deploy without the capability, then with it allowed
    await deployments.reconcile(notes.id);
    const without = await readiness(database, notes.id);
    await database
        .update(object.installation.table)
        .set({ allowed: ["vault-sync"] })
        .where(eq(object.installation.table.id, notes.id));
    await deployments.reconcile(notes.id);
    const allowed = await readiness(database, notes.id);

    // deploy without the vault until the capability needs it
    expect({ without, allowed }).toEqual({
        without: ["Deployed", "1 workloads deployed"],
        allowed: ["UninstalledDependency", "capability vault-sync needs @acme/vault installed"],
    });
});
