import { MemoryBuild } from "@destack/package/test";
import { Plan } from "@destack/resource";
import { ObjectReference, Scope } from "@destack/sync";
import { expect, onTestFinished, test, TestClock } from "@destack/test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { type DatabaseConnection, Snapshot, eq, Expression, DatabaseState } from "@destack/db";
import { aligned, found, Identifier, present, schema } from "@destack/schema";
import { databaseProvider, type LogPosition } from "@destack/db";
import { BunSqliteDatabaseHost } from "@destack/db/bun";
import {
    BROWSER_CAPABILITIES,
    HOST_CAPABILITIES,
    Capabilities,
    type CapabilityName,
    type DirectoryGrant,
    Package,
} from "@destack/package";
import { accessRole, principal, Caller } from "@destack/access";
import { client, space } from "@destack/account/object";
import { defineObject, type Field, field, type ObjectDefinition } from "@destack/object";
import type { ServerRuntime } from "@destack/package/runtime";
import type { BuildReader } from "@destack/package/manifest";
import { PackageStore } from "@destack/build/store";
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
    T0,
    TestSpaceDatabaseHost,
} from "../src/test/index.ts";
import { InstallationBindingSpec } from "@destack/runtime/declare";
import { RequestId } from "@destack/runtime/request";

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

/** The package defining resource kinds and triggers. */
const runtimePackage = Package.parse({
    id: "package-01996ab0-0000-7000-8000-00000000000e",
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
    }).methods();
    const { definition } = note.describe();
    const description = {
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

/** Write the notes build: its main workload in bun and workerd outputs, and a home view. */
async function notesBuild(
    declared: readonly Declared[],
    capabilities: Capabilities = {},
): Promise<BuildReader> {
    // keep the workload using every declared resource in both server outputs
    const used = {
        ...workload,
        capabilities: Capabilities.select(
            capabilities,
            HOST_CAPABILITIES.filter((name) => capabilities[name] !== undefined),
        ),
        resources: declared
            .filter((entry) => (entry.kind ?? "resource") === "resource")
            .map((entry) => ({
                packageId: ids.package,
                name: entry.name,
            })),
    };
    const browser = BROWSER_CAPABILITIES.filter((name) => capabilities[name] !== undefined);
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
                          permissions: {},
                          capabilities: Capabilities.select(capabilities, browser),
                          presents: [],
                          packages: [],
                      },
                  },
    });

    // declare each resource, package and trigger as its definer's inspector records it
    const built = await MemoryBuild.declaring(
        notesPackage,
        declared.map((entry) => ({
            kind: entry.kind ?? "resource",
            package: entry.definer.id,
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
    const notes = aligned(
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
    const manifest = await PackageStore.digest(reader.manifest);
    const revision = aligned(
        await database
            .insert(object.installationRevision.table)
            .values({
                scope: ids.space,
                createdAt: now,
                updatedAt: now,
                id: Identifier.create("installation-revision"),
                installationId,
                build: { ...built, manifest },
                views: {},
                settings: [],
                catalogs: [],
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
    const store = serveSpace(
        await fixture.options({
            machine: ids.machine,
            providers: [
                databaseProvider(new BunSqliteDatabaseHost(root), object.database),
                databaseProvider(new TestSpaceDatabaseHost(root), object.database),
            ],
            ...options.opened,
            ...(options.accounts === undefined ? {} : { accounts: options.accounts }),
            ...(options.clock === undefined ? {} : { clock: options.clock }),
            runtimes: runtimes.map((name) =>
                name === runtime.name ? runtime : new TestRuntime(name),
            ),
        }),
    ).store;

    return {
        root,
        runtime,
        authorizer: store.authorizer,
        deployments: {
            reconcile: (id: string) => reconcile(store, "installation", { id }),
        },
        instances: {
            reconcile: (id: string) => reconcile(store, "deployment", { id }),
            settle: (after: LogPosition) => settle(database, store.controllers(), after),
        },
        resources: {
            reconcile: (scope: string) => reconcileResources(store, scope),
        },
        approve: (id: Identifier<"installation">) => approvePlan(database, store, id),
    };
}

/** Approve the plan an installation waits on as it stands. */
async function approvePlan(
    database: DatabaseConnection,
    store: ReturnType<typeof serveSpace>["store"],
    id: Identifier<"installation">,
) {
    // read the plan the installation waits on
    const target = aligned(
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
    const [granted] = await store.mutate(Caller.of(store.principal, Date.now()), {
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
    await instances.reconcile(aligned(deployed, 0).id);
    const running = aligned(await database.select().from(object.instance.table), 0);
    const installed = aligned(
        await database
            .select()
            .from(object.installation.table)
            .where(eq(object.installation.table.id, notes.id)),
        0,
    );
    const captured = await database
        .select({ name: object.capture.table.name })
        .from(object.capture.table);
    const run = aligned(runtime.started, 0);

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
        run: [running.id, aligned(deployed, 0).id, ids.space, "server", "main"],
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
    await instances.reconcile(aligned(deployed, 0).id);
    await instances.reconcile(aligned(deployed, 0).id);
    const retired = aligned(await database.select().from(object.deployment.table), 0);
    const stopped = aligned(await database.select().from(object.instance.table), 0);
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
    await deployments.reconcile(notes.id);

    // wait with the instance while main has not applied the notes' tables
    const deployed = aligned(await database.select().from(object.deployment.table), 0);
    await instances.reconcile(deployed.id);
    const mainDatabase = aligned(await database.select().from(object.database.table), 0);
    const waiting = aligned(await database.select().from(object.instance.table), 0);
    expect({
        started: runtime.started.length,
        instance: [waiting.status, waiting.restarts, waiting.conditions["ready"]?.status],
        ready: [waiting.conditions["ready"]?.reason, waiting.conditions["ready"]?.message],
    }).toEqual({
        started: 0,
        instance: ["starting", 0, "false"],
        ready: ["WaitingForResources", `resources ${mainDatabase.id} are not applied`],
    });

    // apply main to start the waiting instance
    const provisioned = await database.log.position();
    await resources.reconcile(ids.space);
    await instances.settle(provisioned);
    const applied = aligned(await database.select().from(object.database.table), 0);
    const attached = aligned(await database.select().from(object.binding.table), 0);
    const running = aligned(await database.select().from(object.instance.table), 0);
    const bound = DatabaseState.parse(attached.state);
    expect({
        main: [applied.name, applied.ownerId, applied.conditions["ready"]?.reason],
        binding: [attached.installationId, attached.name, attached.target],
        tables: bound.tables.sqlite.map((state) => state.table.name),
        instance: [running.id, running.status],
        resources: present(runtime.started.at(-1), "the started run").resources,
    }).toEqual({
        main: ["main", null, "Applied"],
        binding: [notes.id, "main", mainDatabase.id],
        tables: ["example__notes__note"],
        instance: [waiting.id, "running"],
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
    const retained = aligned(await database.select().from(object.database.table), 0);
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
    const clock = new TestClock(T0);
    const { deployments, instances, resources } = await serveControllers(fixture, {
        runtime: failing,
        clock,
    });
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const deployed = aligned(await database.select().from(object.deployment.table), 0);

    // fail, wait out the backoff and restart
    const first = await instances.reconcile(deployed.id);
    const early = await instances.reconcile(deployed.id);
    await database
        .update(object.instance.table)
        .set({ stoppedAt: T0 - 1000 })
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
    const deployed = aligned(await database.select().from(object.deployment.table), 0);
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
        before: [aligned(recorded, 0).id],
        after: [aligned(recorded, 0).id],
        instances: [[aligned(recorded, 0).id, "running"]],
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
    const first = aligned(await database.select().from(object.deployment.table), 0);
    await instances.reconcile(first.id);

    // submit a second revision that deploys beside the first and drains it
    const second = await submitRevision(fixture, notes.id, "f");
    await deployments.reconcile(notes.id);
    const replacement = aligned(
        await database
            .select()
            .from(object.deployment.table)
            .where(eq(object.deployment.table.revisionId, second.id)),
        0,
    );
    await instances.reconcile(first.id);
    const serving = aligned(
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
    const installed = aligned(
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
    const owned = aligned(await database.select().from(object.database.table), 0);

    // rebind main to a second database of the same declaration
    const now = Date.now();
    const second = aligned(
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
            [aligned(deployed, 0).id, owned.id],
            [aligned(deployed, 1).id, second.id],
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
    const installed = aligned(
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
    const installed = aligned(
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

test("run each workload with the capabilities and directories its build declares and its installation allows", async () => {
    // install notes whose workload runs git and optionally calls an issue tracker or reads chosen directories and whose view may use the camera
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const declared = Capabilities.parse({
        run: { commands: ["git"], reason: "reads history" },
        network: { connect: ["api.github.com"], reason: "fetches issues", optional: true },
        fs: { access: "read", reason: "indexes your notes", optional: true },
        camera: { reason: "scans receipts", optional: true },
    });
    const notes = await installNotes(fixture, ids.notes, await notesBuild([main], declared));
    const { runtime, deployments, instances, resources } = await serveControllers(fixture);
    const notesDirectory = { path: "/Users/person/Notes", access: "read" as const };
    const grant = async (
        capabilities: readonly CapabilityName[],
        directories: readonly DirectoryGrant[] = [],
    ) => {
        await database
            .update(object.installation.table)
            .set({ capabilities: [...capabilities], directories: [...directories] })
            .where(eq(object.installation.table.id, notes.id));
        await deployments.reconcile(notes.id);
    };
    const readiness = async () => {
        const [installed] = await database
            .select({ conditions: object.installation.table.conditions })
            .from(object.installation.table)
            .where(eq(object.installation.table.id, notes.id));
        const ready = installed?.conditions["ready"];

        return [ready?.reason, ready?.message];
    };
    const deployed = async () =>
        (
            await database
                .select()
                .from(object.deployment.table)
                .orderBy(object.deployment.table.createdAt, object.deployment.table.id)
        ).map((entry) => [entry.status, entry.description.capabilities, entry.directories]);

    // refuse allowing an undeclared capability and granting a directory for writing that fs reads
    await grant(["microphone"]);
    const undeclared = await readiness();
    await grant(["fs"], [{ ...notesDirectory, access: "write" }]);
    const overreaching = await readiness();
    const refusedDeployments = await deployed();

    // deploy without allowances with only the required capability and no directory
    await grant([], [notesDirectory]);
    await resources.reconcile(ids.space);
    const first = aligned(await database.select().from(object.deployment.table), 0);
    await instances.reconcile(first.id);
    const unallowed = await deployed();

    // allow the network, fs and the view's camera and redeploy the workload with the directory but without the camera
    await grant(["network", "fs", "camera"], [notesDirectory]);
    const allowed = await deployed();
    const required = { run: declared.run };
    expect({
        undeclared,
        overreaching,
        refusedDeployments,
        started: runtime.started.map((started) => [started.capabilities, started.directories]),
        unallowed,
        allowed,
    }).toEqual({
        undeclared: [
            "UndeclaredCapability",
            "notes 2026.9.0 declares no capabilities microphone to allow",
        ],
        overreaching: [
            "UndeclaredCapability",
            "notes 2026.9.0 declares fs for reading only, refusing to grant writing /Users/person/Notes",
        ],
        refusedDeployments: [],
        started: [[required, []]],
        unallowed: [["active", required, []]],
        allowed: [
            ["draining", required, []],
            [
                "active",
                {
                    ...required,
                    network: declared.network,
                    fs: declared.fs,
                },
                [notesDirectory],
            ],
        ],
    });
});

test("fail an instance granted fs on a runtime without a filesystem to grant", async () => {
    // deploy notes requiring fs on a machine running workerd only
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const fs = Capabilities.parse({ fs: { access: "read", reason: "indexes your notes" } });
    const notes = await installNotes(fixture, ids.notes, await notesBuild([main], fs));
    const { deployments, instances, resources } = await serveControllers(fixture, {
        runtimes: ["workerd"],
        runtime: new TestRuntime("workerd"),
    });
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const deployed = aligned(await database.select().from(object.deployment.table), 0);

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
    const readiness = async () =>
        aligned(
            await database
                .select({ conditions: object.installation.table.conditions })
                .from(object.installation.table)
                .where(eq(object.installation.table.id, notes.id)),
            0,
        ).conditions["ready"];
    const refused = await readiness();

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
    const observed = aligned(
        await database
            .select({ conditions: object.dependency.table.conditions })
            .from(object.dependency.table)
            .where(eq(object.dependency.table.id, declared.id)),
        0,
    );
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const deployed = aligned(await database.select().from(object.deployment.table), 0);
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
    const deployed = aligned(await database.select().from(object.deployment.table), 0);
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
    const first = aligned(await database.select().from(object.deployment.table), 0);
    await instances.reconcile(first.id);

    // deploy a second revision dropping the color beside the first and plan while the first drains
    declared = [main];
    const second = await submitRevision(fixture, notes.id, "f", await notesBuild(declared));
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const plan = async () => {
        const row = aligned(await database.select().from(object.database.table), 0);
        return [
            row.conditions["ready"]?.reason,
            row.plan?.steps.map((step) => [step.action, step.risk, step.detail]) ?? null,
        ];
    };
    const whileDraining = await plan();

    // run the second revision and retire the first
    const replacement = aligned(
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
    const original = aligned(await database.select().from(object.deployment.table), 0);
    await instances.reconcile(original.id);

    // deploy the narrowing revision beside it and plan a recreation that waits for approval
    declared = [narrowed];
    const second = await submitRevision(fixture, notes.id, "f", await notesBuild(declared));
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const read = async () => {
        const row = aligned(await database.select().from(object.database.table), 0);
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
    const stopping = aligned(
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
    const replacement = aligned(
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
    const first = aligned(await database.select().from(object.deployment.table), 0);
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
            },
        },
    ];
    const second = await submitRevision(fixture, notes.id, "f", await notesBuild(declared));
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const replacement = aligned(
        await database
            .select()
            .from(object.deployment.table)
            .where(eq(object.deployment.table.revisionId, second.id)),
        0,
    );
    await instances.reconcile(replacement.id);
    await instances.reconcile(first.id);
    const status = async () =>
        aligned(
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

/** A daily schedule trigger the notes build declares, pinning the first note at an hour. */
function daily(hour: number): Declared {
    return {
        kind: "trigger",
        definer: runtimePackage,
        name: "daily",
        description: {
            name: "daily",
            on: {
                schedule: {
                    timing: { timing: "cron", cron: `0 ${hour} * * *`, timezone: "UTC" },
                    concurrency: "forbid",
                    deadline: 60_000,
                    call: {
                        type: { packageId: notesPackage.id, type: "note" },
                        method: "update",
                        input: { id: "note-1", pinned: true },
                        release: "2026.9.0",
                    },
                },
            },
        },
    };
}

test("keep exactly the triggers each applied revision's build declares, leaving the ones the installation created", async () => {
    // declare a daily schedule trigger at nine, at ten and none in one revision each
    const builds = new Map<string, readonly Declared[]>([
        ["e", [main, daily(9)]],
        ["f", [main, daily(10)]],
        ["a", [main]],
    ]);
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const notes = await installNotes(fixture, ids.notes, await notesBuild(found(builds, "e")));
    const { deployments } = await serveControllers(fixture);

    // keep a schedule trigger the installation created beside the declared ones
    const now = Date.now();
    await database.insert(object.trigger.table).values({
        id: Identifier.create("trigger"),
        scope: ids.space,
        createdAt: now,
        updatedAt: now,
        installationId: notes.id,
        name: "reminder",
        on: {
            schedule: {
                timing: { timing: "once", startsAt: now + 60_000 },
                concurrency: "allow",
                deadline: 60_000,
                call: {
                    type: { packageId: notesPackage.id, type: "note" },
                    method: "update",
                    input: { id: "note-2", pinned: true },
                    release: "2026.9.0",
                },
            },
        },
    });

    // apply each revision and read the triggers after each
    const schedules = async () =>
        (await database.select().from(object.trigger.table).orderBy(object.trigger.table.name)).map(
            (entry) => {
                if (!("schedule" in entry.on)) {
                    throw new TypeError(`trigger ${entry.name} fires on no schedule`);
                }
                const { timing } = entry.on.schedule;

                return [
                    entry.name,
                    entry.origin,
                    timing.timing === "cron" ? timing.cron : timing.timing,
                ];
            },
        );
    const applied = [];
    for (const character of ["e", "f", "a"]) {
        if (character !== "e") {
            await submitRevision(
                fixture,
                notes.id,
                character,
                await notesBuild(found(builds, character)),
            );
        }
        await deployments.reconcile(notes.id);
        applied.push(await schedules());
    }

    expect(applied).toEqual([
        [
            ["daily", "declared", "0 9 * * *"],
            ["reminder", "created", "once"],
        ],
        [
            ["daily", "declared", "0 10 * * *"],
            ["reminder", "created", "once"],
        ],
        [["reminder", "created", "once"]],
    ]);
});

test("grant an applied application exactly the space permissions its workloads request, and drop the grant once it is uninstalled", async () => {
    // install notes whose workload requests reading the space and forgetting push subscriptions
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const requesting = await MemoryBuild.write(new Map(), {
        package: notesPackage,
        outputs: fixtureOutputs({
            server: {
                runtime: "bun",
                emit: true,
                workloads: {
                    main: {
                        ...workload,
                        permissions: {
                            space: [accountObject.space.permission("read")],
                            home: [client.permission("unsubscribe")],
                        },
                    },
                },
                views: {},
            },
        }),
    });
    const notes = await installNotes(fixture, ids.notes, requesting);
    const owner = principal.user.reference(
        Scope.universe.id,
        "user-01996ab0-0000-7000-8000-0000000000a1",
    );
    await database
        .update(object.installation.table)
        .set(ObjectReference.columns("submittedBy", owner));

    // apply it on a bun machine
    const opened = await openUniverse();
    const { authorizer, deployments, instances } = await serveControllers(fixture, { opened });
    await deployments.reconcile(notes.id);
    const deployed = aligned(await database.select().from(object.deployment.table), 0);

    // decide the installation's reading and managing of its space
    const spaceObject = await Scope.object(Snapshot.live(database), ids.space);
    const decide = async () => {
        const snapshot = Snapshot.live(database);
        const access = await authorizer.resolve(snapshot, ids.space, {
            subjects: [principal.installation.reference(ids.space, notes.id)],
            attributes: {},
            now: Date.now(),
        });

        return Promise.all(
            [accountObject.space.permission("read"), accountObject.space.permission("manage")].map(
                async (permission) =>
                    (await authorizer.check(snapshot, permission, spaceObject, access)).isAllowed,
            ),
        );
    };
    const granted = await decide();

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
        dropped: await decide(),
        roles: roles.filter((role) => role.name.startsWith("installation/")),
    }).toEqual({
        granted: [true, false],
        dropped: [false, false],
        roles: [],
    });
});
