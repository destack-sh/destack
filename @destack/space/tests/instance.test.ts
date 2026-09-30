import { Plan } from "@destack/resource";
import { expect, onTestFinished, test } from "@destack/test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { defineTable, eq, json, TABLE, type Table, text } from "@destack/db";
import { Expression } from "@destack/schema/expression";
import { defineDatabase } from "@destack/db/declare";
import { describeDatabase } from "@destack/db/inspect";
import { sqliteProvider } from "@destack/db/sqlite";
import { Package } from "@destack/package";
import { describeFile } from "@destack/package/file";
import { type PackageManifest, BuildReader } from "@destack/package/manifest";
import { identifier, type Identifier, schema } from "@destack/schema";
import { type ServerRuntime } from "@destack/package/runtime";
import { v7 } from "uuid";
import * as object from "../src/object/index.ts";
import { Approval, type OpenBuild, serveObjects } from "../src/server/index.ts";
import {
    ids,
    openBuild,
    openSpace,
    reconcile,
    spaceOptions,
    TestRuntime,
    wake,
    workload,
    reconcileResources,
} from "./fixture/space.ts";

/** The notes package the scenarios install. */
const notesPackage = Package.parse({
    id: ids.package,
    name: "@example/notes",
    version: "2026.9.0",
});

/** The package defining databases. */
const databasePackage = Package.parse({
    id: "package-01996ab0-0000-7000-8000-00000000000d",
    name: "@destack/db",
    version: "2026.9.0",
});

/** Notes with a body. */
const note = defineTable("note", { id: text("id").primaryKey(), body: text("body") });

/** The main database notes declares. */
const mainDatabase = defineDatabase({ name: "main", tables: [note] });

/** The main workload of the notes build, reaching its main database. */
const reaching = { ...workload, resources: [{ packageId: ids.package, name: "main" }] };

/** The package defining service bindings. */
const servicePackage = Package.parse({
    id: "package-01996ab0-0000-7000-8000-00000000000e",
    name: "@destack/service",
    version: "2026.9.0",
});

/** A declaration of the notes build, with the package defining its kind. */
interface Declared {
    /** The declaration kind, a resource when absent. */
    readonly kind?: "resource" | "service" | "schedule";
    /** The package defining the declaration's kind. */
    readonly definer: Package;
    /** The declaration's description, as the definer's inspector records it. */
    readonly description: { readonly name: string } & Record<string, unknown>;
}

/** The main database, as the database package's inspector records it. */
const main: Declared = { definer: databasePackage, description: describeDatabase(mainDatabase) };

/** The tasks service binding, calling the service of the notes package. */
const tasks: Declared = {
    definer: servicePackage,
    description: {
        name: "tasks",
        kind: "service",
        spec: { service: { packageId: ids.package, name: "service" } },
    },
};

/** Open every build as the notes build: its main workload in a bun and a workerd output, reaching the declared resources. */
const openNotesWith =
    (declared: readonly Declared[]): OpenBuild =>
    async () => {
        // describe each resource as its definer's inspector records it, one collection per definer
        const descriptions: Record<string, unknown> = {};
        const bytes = new Map<string, Uint8Array<ArrayBuffer>>();
        for (const definer of new Set(declared.map((entry) => entry.definer))) {
            const declarations = declared
                .filter((entry) => entry.definer === definer)
                .map((entry) => ({
                    name: entry.description.name,
                    kind: entry.kind ?? "resource",
                    package: definer,
                    constructor: {
                        package: definer,
                        symbol: { module: "src/declare.ts", name: "define" },
                    },
                    symbol: {
                        package: notesPackage,
                        symbol: { module: "src/resource.ts", name: entry.description.name },
                    },
                    source: { file: "src/resource.ts", line: 1, column: 0 },
                    description: entry.description,
                }));
            const path = `manifest/${definer.name.replace("@destack/", "")}.json`;
            const encoded = new TextEncoder().encode(JSON.stringify(declarations));
            bytes.set(path, encoded);
            descriptions[definer.name] = {
                package: definer,
                file: await describeFile(path, "application/json", encoded),
            };
        }

        // keep the workload reaching every declared resource in both server outputs
        const reached = {
            ...workload,
            resources: declared
                .filter((entry) => (entry.kind ?? "resource") === "resource")
                .map((entry) => ({
                    packageId: ids.package,
                    name: entry.description.name,
                })),
        };
        const output = (runtime: string) => ({
            runtime,
            emit: true,
            workloads: { main: reached },
            views: {},
        });

        return new BuildReader(
            {
                package: notesPackage,
                descriptions,
                outputs: { server: output("bun"), worker: output("workerd") },
            } as unknown as PackageManifest,
            async (path) => bytes.get(path)!,
        );
    };

/** Open every build as the notes build reaching its main database. */
const openNotesBuild = openNotesWith([main]);

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
    host: ids.host,
    checkout: identifier("checkout").parse("checkout-01996ab0-0000-7000-8000-000000000008"),
    directory: "." as const,
};

/** Install notes from a checkout at one revision of its build and return the installation. */
async function installNotes(
    database: Awaited<ReturnType<typeof openSpace>>,
    id: Identifier<"installation"> = ids.notes,
) {
    const now = Date.now();
    const [notes] = await database
        .insert(object.installation.table)
        .values({
            scope: ids.space,
            createdAt: now,
            updatedAt: now,
            id,
            packageId: ids.package,
            role: "application",
            selection,
            alias: "notes",
        })
        .returning();
    await submitRevision(database, id, "e");

    return notes!;
}

/** Retain a revision of an installation's checkout build, digested from one repeated character, and follow it. */
async function submitRevision(
    database: Awaited<ReturnType<typeof openSpace>>,
    installationId: Identifier<"installation">,
    character: string,
) {
    const now = Date.now();
    const [revision] = await database
        .insert(object.installationRevision.table)
        .values({
            scope: ids.space,
            createdAt: now,
            updatedAt: now,
            id: identifier("installation-revision").parse(`installation-revision-${v7()}`),
            installationId,
            build: { ...selection, manifest: character.repeat(64) },
            views: {},
            settings: [],
            digest: character.repeat(64),
        })
        .returning();
    await database
        .update(object.installation.table)
        .set({ revisionId: revision!.id })
        .where(eq(object.installation.table.id, installationId));

    return revision!;
}

/** Serve a database's deployments, instances and resources through the controllers of a host. */
async function serveControllers(
    database: Awaited<ReturnType<typeof openSpace>>,
    options: {
        readonly runtimes?: readonly ServerRuntime[];
        readonly runtime?: TestRuntime;
        readonly openBuild?: OpenBuild;
    } = {},
) {
    // provision the host's databases as files in a temporary directory
    const directory = await mkdtemp(join(tmpdir(), "destack-instance-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    const root = pathToFileURL(`${directory}/`);
    const runtime = options.runtime ?? new TestRuntime();
    const runtimes = options.runtimes ?? ["bun"];

    // run the chosen runtimes, recording instances on the one test runtime
    const server = serveObjects(
        await spaceOptions(database, {
            cell: { hostId: ids.host },
            providers: [sqliteProvider(root)],
            openBuild: options.openBuild ?? openNotesBuild,
            runtimes: runtimes.map((name) =>
                name === runtime.name ? runtime : new TestRuntime(name),
            ),
        }),
    );

    return {
        root,
        runtime,
        deployments: {
            reconcile: (id: string) => reconcile(server, "installation", { id }),
        },
        instances: {
            reconcile: (id: string) => reconcile(server, "deployment", { id }),
            wake: (row: object.Resource) =>
                wake(server, "deployment", {
                    sequence: 0,
                    transaction: null,
                    table: object.resource.table as Table,
                    key: { id: row.id },
                    operation: "update",
                    scope: row.scope,
                    changedAt: Date.now(),
                    after: row,
                }),
        },
        resources: {
            reconcile: (scope: string) => reconcileResources(server, scope),
        },
        approve: async (id: Identifier<"installation">) => {
            // approve the plan the installation waits on as it stands
            const [target] = await database
                .select()
                .from(object.installation.table)
                .where(eq(object.installation.table.id, id));
            const approval = await Approval.of(database, target!);
            const digest = await Plan.digest(approval.plan());

            return database.transaction((transaction) =>
                approval.grant(digest, (type, name, input) =>
                    server.invoke(transaction, ids.space, type, name, input, Date.now()),
                ),
            );
        },
    };
}

test("deploy each workload from the output the cell runs, run its instance with the resources its stack binds, and drain and retire it once suspended", async () => {
    // install notes with a stack that binds its main database to a provisioned resource
    const database = await openSpace();
    const notes = await installNotes(database);
    const now = Date.now();
    const record = { scope: ids.space, createdAt: now, updatedAt: now };
    const stack = {
        ...record,
        id: identifier("installation").parse("installation-01996ab0-0000-7000-8000-000000000010"),
        packageId: databasePackage.id,
        role: "stack" as const,
        selection: { kind: "release" as const, version: "2026.9.0" },
        export: "default",
        parameters: {},
        alias: "stack",
    };
    const main = {
        ...record,
        id: identifier("resource").parse("resource-01996ab0-0000-7000-8000-000000000011"),
        name: "main",
        kind: "database",
        definitionPackageId: ids.package,
        definitionVersion: "2026.9.0",
        definitionName: "main",
        spec: {},
        providerCode: "sqlite",
        reference: "file:///main.db",
        conditions: {
            ready: {
                status: "true" as const,
                observedGeneration: 1,
                reason: "Applied",
                message: "1 steps applied",
                lastTransitionAt: now,
            },
        },
    };
    await database.insert(object.installation.table).values(stack);
    await database.insert(object.resource.table).values(main);
    await database.insert(object.binding.table).values({
        ...record,
        id: identifier("binding").parse("binding-01996ab0-0000-7000-8000-000000000012"),
        installationId: notes.id,
        packageId: ids.package,
        name: "main",
        target: main.id,
        state: {},
        managerInstallationId: stack.id,
        managerPackageId: stack.packageId,
        managerName: `installations/notes/${ids.package}/main`,
    });

    // deploy it on a bun cell, then run its instance on this host
    const { runtime, deployments, instances } = await serveControllers(database);
    await deployments.reconcile(notes.id);
    const deployed = await database.select().from(object.deployment.table);
    await instances.reconcile(deployed[0]!.id);
    const [running] = await database.select().from(object.instance.table);
    const [installed] = await database
        .select()
        .from(object.installation.table)
        .where(eq(object.installation.table.id, notes.id));
    const retained = await database
        .select({ id: object.resource.table.id })
        .from(object.resource.table);
    const [run] = runtime.started;

    // deploy only the bun output, and run it with the stack's resource instead of one of its own
    expect({
        deployments: deployed.map((entry) => [
            entry.status,
            entry.output,
            entry.runtime,
            entry.workload,
            entry.description,
        ]),
        instance: [running!.status, running!.hostId],
        run: [run!.instanceId, run!.deploymentId, run!.scope, run!.output, run!.workload],
        resources: run!.resources,
        retained,
        applied: installed!.appliedRevisionId === installed!.revisionId,
    }).toEqual({
        deployments: [["active", "server", "bun", "main", reaching]],
        instance: ["running", ids.host],
        run: [running!.id, deployed[0]!.id, ids.space, "server", "main"],
        resources: [
            {
                packageId: ids.package,
                name: "main",
                id: main.id,
                scope: ids.space,
                kind: "database",
                spec: {},
                reference: "file:///main.db",
                providerCode: "sqlite",
            },
        ],
        retained: [{ id: main.id }],
        applied: true,
    });

    // suspend the application: drain its deployment, stop its instance, and retire it
    await database
        .update(object.installation.table)
        .set({ status: "suspended" })
        .where(eq(object.installation.table.id, notes.id));
    await deployments.reconcile(notes.id);
    await instances.reconcile(deployed[0]!.id);
    await instances.reconcile(deployed[0]!.id);
    const [retired] = await database.select().from(object.deployment.table);
    const [stopped] = await database.select().from(object.instance.table);
    expect([retired!.status, stopped!.status, runtime.stopped]).toEqual([
        "retired",
        "stopped",
        [running!.id],
    ]);
});

test("provision the database an application declares without a binding, start its instance once the database is applied, and retain the database across an uninstall and reinstall of the same package", async () => {
    // install notes without a stack, and deploy it
    const database = await openSpace();
    const notes = await installNotes(database);
    const { root, runtime, deployments, instances, resources } = await serveControllers(database);
    await deployments.reconcile(notes.id);

    // wait with the instance while the database notes owns is not applied
    const [deployed] = await database.select().from(object.deployment.table);
    await instances.reconcile(deployed!.id);
    const [owned] = await database.select().from(object.resource.table);
    const [waiting] = await database.select().from(object.instance.table);
    expect({
        started: runtime.started.length,
        instance: [waiting!.status, waiting!.restarts, waiting!.conditions.ready?.status],
        ready: [waiting!.conditions.ready?.reason, waiting!.conditions.ready?.message],
    }).toEqual({
        started: 0,
        instance: ["starting", 0, "false"],
        ready: ["WaitingForResources", `resources ${owned!.id} are not applied`],
    });

    // provision and migrate the database to start the waiting instance
    await resources.reconcile(ids.space);
    const [applied] = await database.select().from(object.resource.table);
    await instances.wake(applied!);
    const [attached] = await database.select().from(object.binding.table);
    const [captured] = await database.select().from(object.capture.table);
    const [running] = await database.select().from(object.instance.table);
    const [run] = runtime.started;
    const reference = new URL(`${ids.space}/${owned!.id}.db`, root).href;
    expect({
        resource: [
            applied!.name,
            applied!.kind,
            applied!.ownerInstallationId,
            applied!.definitionPackageId,
            applied!.definitionVersion,
            applied!.definitionName,
            applied!.spec,
            applied!.conditions.ready?.reason,
        ],
        binding: [
            attached!.installationId,
            attached!.packageId,
            attached!.name,
            attached!.target,
            attached!.state,
        ],
        capture: [captured!.deploymentId, captured!.name, captured!.target],
        instance: [running!.id, running!.status, running!.conditions.ready?.reason],
        resources: run!.resources,
    }).toEqual({
        resource: [
            "notes.main",
            "database",
            notes.id,
            ids.package,
            "2026.9.0",
            "main",
            { tier: "zonal" },
            "Applied",
        ],
        binding: [notes.id, ids.package, "main", owned!.id, mainDatabase.state()],
        capture: [deployed!.id, "main", owned!.id],
        instance: [waiting!.id, "running", "Running"],
        resources: [
            {
                packageId: ids.package,
                name: "main",
                id: owned!.id,
                scope: ids.space,
                kind: "database",
                spec: { tier: "zonal" },
                reference,
                providerCode: "sqlite",
            },
        ],
    });

    // uninstall notes: drain its deployment, then remove it with its history, retaining its database
    await database
        .update(object.installation.table)
        .set({ deletionRequestedAt: Date.now() })
        .where(eq(object.installation.table.id, notes.id));
    await deployments.reconcile(notes.id);
    await instances.reconcile(deployed!.id);
    await instances.reconcile(deployed!.id);
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const [retained] = await database.select().from(object.resource.table);
    expect({
        installations: await database.select().from(object.installation.table),
        revisions: await database.select().from(object.installationRevision.table),
        deployments: await database.select().from(object.deployment.table),
        instances: await database.select().from(object.instance.table),
        captures: await database.select().from(object.capture.table),
        bindings: await database.select().from(object.binding.table),
        resource: [
            retained!.id,
            retained!.ownerInstallationId,
            retained!.deletionRequestedAt !== null,
            retained!.conditions.ready?.reason,
        ],
    }).toEqual({
        installations: [],
        revisions: [],
        deployments: [],
        instances: [],
        captures: [],
        bindings: [],
        resource: [owned!.id, null, true, "Retained"],
    });

    // refuse owning it again while another package's declaration defines it
    const define = (packageId: string) =>
        database
            .update(object.resource.table)
            .set({ definitionPackageId: packageId as typeof ids.package })
            .where(eq(object.resource.table.id, owned!.id));
    await define("package-01996ab0-0000-7000-8000-0000000000f0");
    const again = await installNotes(
        database,
        identifier("installation").parse(`installation-${v7()}`),
    );
    const refusal = await deployments.reconcile(again.id).then(
        () => "reconciled",
        (error: Error) => error.message,
    );
    expect(refusal).toBe("resource notes.main exists and is not a retained database to own again");

    // own and run the retained database again once the package's declaration defines it
    await define(ids.package);
    await deployments.reconcile(again.id);
    await resources.reconcile(ids.space);
    const [redeployed] = await database.select().from(object.deployment.table);
    await instances.reconcile(redeployed!.id);
    const [adopted] = await database.select().from(object.resource.table);
    expect({
        resource: [
            adopted!.id,
            adopted!.ownerInstallationId,
            adopted!.deletionRequestedAt,
            adopted!.conditions.ready?.reason,
        ],
        resources: runtime.started.at(-1)!.resources,
    }).toEqual({
        resource: [owned!.id, again.id, null, "Applied"],
        resources: [
            {
                packageId: ids.package,
                name: "main",
                id: owned!.id,
                scope: ids.space,
                kind: "database",
                spec: { tier: "zonal" },
                reference,
                providerCode: "sqlite",
            },
        ],
    });
});

test("restart a failing instance in place after a doubling backoff, keeping one record with its last failure", async () => {
    // deploy notes with an applied database on a cell with a failing runner
    const database = await openSpace();
    const notes = await installNotes(database);
    const failing = new FailingRuntime();
    const { deployments, instances, resources } = await serveControllers(database, {
        runtime: failing,
    });
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const [deployed] = await database.select().from(object.deployment.table);

    // fail, wait out the backoff, then restart
    const first = await instances.reconcile(deployed!.id);
    const early = await instances.reconcile(deployed!.id);
    await database
        .update(object.instance.table)
        .set({ stoppedAt: Date.now() - 1000 })
        .where(eq(object.instance.table.deploymentId, deployed!.id));
    const second = await instances.reconcile(deployed!.id);
    const recorded = await database.select().from(object.instance.table);
    expect({
        delays: [first, early! > 0 && early! <= 1000, second],
        attempts: failing.attempts,
        instances: recorded.map((entry) => [
            entry.status,
            entry.restarts,
            entry.conditions.ready?.status,
            entry.conditions.ready?.reason,
            entry.conditions.ready?.message,
        ]),
    }).toEqual({
        delays: [1000, true, 2000],
        attempts: 2,
        instances: [["failed", 1, "false", "StartFailed", "runner exited before serving"]],
    });
});

test("start a running instance again once its cell restarted and no longer runs its process", async () => {
    // run notes with an applied database
    const database = await openSpace();
    const notes = await installNotes(database);
    const before = new TestRuntime();
    const first = await serveControllers(database, { runtime: before });
    await first.deployments.reconcile(notes.id);
    await first.resources.reconcile(ids.space);
    const [deployed] = await database.select().from(object.deployment.table);
    await first.instances.reconcile(deployed!.id);

    // restart the cell with an idle runtime and reconcile again
    const after = new TestRuntime();
    const second = await serveControllers(database, { runtime: after });
    await second.instances.reconcile(deployed!.id);
    const recorded = await database.select().from(object.instance.table);
    expect({
        before: before.started.map((run) => run.instanceId),
        after: after.started.map((run) => run.instanceId),
        instances: recorded.map((entry) => [entry.id, entry.status]),
    }).toEqual({
        before: [recorded[0]!.id],
        after: [recorded[0]!.id],
        instances: [[recorded[0]!.id, "running"]],
    });
});

test("deploy a submitted revision beside the previous one, keep the previous one serving until the new one runs, then retire it", async () => {
    // run the first revision of notes with its applied database
    const database = await openSpace();
    const notes = await installNotes(database);
    const { deployments, instances, resources } = await serveControllers(database);
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const [first] = await database.select().from(object.deployment.table);
    await instances.reconcile(first!.id);

    // submit a second revision that deploys beside the first and drains it
    const second = await submitRevision(database, notes.id, "f");
    await deployments.reconcile(notes.id);
    const [replacement] = await database
        .select()
        .from(object.deployment.table)
        .where(eq(object.deployment.table.revisionId, second.id));
    await instances.reconcile(first!.id);
    const [serving] = await database
        .select({ status: object.instance.table.status })
        .from(object.instance.table)
        .where(eq(object.instance.table.deploymentId, first!.id));

    // run the replacement, then stop and retire the drained first revision, as its instance's changes wake it
    for (const id of [replacement!.id, first!.id, first!.id]) {
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
    const [installed] = await database
        .select()
        .from(object.installation.table)
        .where(eq(object.installation.table.id, notes.id));
    // keep the first revision serving until its replacement runs
    expect({
        serving: serving!.status,
        deployments: deployed.map((entry) => [
            entry.revisionId,
            entry.output,
            entry.workload,
            entry.status,
        ]),
        instances: recorded.map((entry) => [entry.deploymentId, entry.status, entry.restarts]),
        applied: installed!.appliedRevisionId,
    }).toEqual({
        serving: "running",
        deployments: [
            [first!.revisionId, "server", "main", "retired"],
            [second.id, "server", "main", "active"],
        ],
        instances: [
            [first!.id, "stopped", 0],
            [replacement!.id, "running", 0],
        ],
        applied: second.id,
    });
});

test("redeploy the revision an application follows once the outputs its cell runs change", async () => {
    // deploy notes on a cell running workerd
    const database = await openSpace();
    const notes = await installNotes(database);
    const { deployments: workerd } = await serveControllers(database, { runtimes: ["workerd"] });
    await workerd.reconcile(notes.id);

    // serve it on a bun cell to replace the workerd deployment
    const { deployments: bun } = await serveControllers(database, { runtimes: ["bun"] });
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
    const database = await openSpace();
    const notes = await installNotes(database);
    const { deployments, resources } = await serveControllers(database);
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const [owned] = await database.select().from(object.resource.table);

    // rebind main to a second database of the same declaration
    const now = Date.now();
    const [second] = await database
        .insert(object.resource.table)
        .values({
            ...owned!,
            id: identifier("resource").parse(`resource-${v7()}`),
            name: "notes.archive",
            ownerInstallationId: null,
            createdAt: now,
            updatedAt: now,
        })
        .returning();
    await database
        .update(object.binding.table)
        .set({ target: second!.id })
        .where(eq(object.binding.table.installationId, notes.id));

    // deploy again with the new target, draining the deployment that captured the old one
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
            [deployed[0]!.id, owned!.id],
            [deployed[1]!.id, second!.id],
        ],
    });
});

test("deploy nothing of a build whose workloads have no output on a runtime the cell runs, and report why", async () => {
    // install notes with its workload only in a bun output
    const database = await openSpace();
    const notes = await installNotes(database);
    const { deployments } = await serveControllers(database, {
        openBuild,
        runtimes: ["workerd"],
    });

    // deploy it on a workerd cell
    await deployments.reconcile(notes.id);
    const [installed] = await database
        .select()
        .from(object.installation.table)
        .where(eq(object.installation.table.id, notes.id));
    expect({
        deployments: await database.select().from(object.deployment.table),
        ready: installed!.conditions.ready,
        applied: installed!.appliedRevisionId,
    }).toEqual({
        deployments: [],
        ready: {
            status: "false",
            reason: "NoRuntime",
            message: "workloads main have no output on workerd",
            observedGeneration: installed!.generation,
            lastTransitionAt: installed!.conditions.ready!.lastTransitionAt,
        },
        applied: null,
    });
});

test("refuse to deploy a service binding without an address, then run the workload with the address its stack declares", async () => {
    // install notes reaching the tasks service besides its main database, bound to a provisioned database
    const database = await openSpace();
    const notes = await installNotes(database);
    const { runtime, deployments, instances, resources } = await serveControllers(database, {
        openBuild: openNotesWith([main, tasks]),
    });

    // refuse to deploy while no stack binds the tasks service to an address
    await deployments.reconcile(notes.id);
    const readiness = async () =>
        (
            await database
                .select({ conditions: object.installation.table.conditions })
                .from(object.installation.table)
                .where(eq(object.installation.table.id, notes.id))
        )[0]!.conditions.ready;
    const refused = await readiness();

    // declare the tasks service at the address of the notes installation itself, and bind it
    const now = Date.now();
    const record = { scope: ids.space, createdAt: now, updatedAt: now };
    const declared = {
        ...record,
        id: identifier("resource").parse("resource-01996ab0-0000-7000-8000-000000000021"),
        name: "tasks",
        kind: "service",
        definitionPackageId: servicePackage.id,
        definitionVersion: "2026.9.0",
        definitionName: "tasks",
        spec: tasks.description.spec as Record<string, never>,
        origin: "declared" as const,
        providerCode: "http",
        reference: "notes",
    };
    await database.insert(object.resource.table).values(declared);
    await database.insert(object.binding.table).values({
        ...record,
        id: identifier("binding").parse("binding-01996ab0-0000-7000-8000-000000000022"),
        installationId: notes.id,
        packageId: ids.package,
        name: "tasks",
        target: declared.id,
        state: {},
    });

    // observe it ready as declared, then deploy and run with the database the installation owns
    await resources.reconcile(ids.space);
    const [observed] = await database
        .select({ conditions: object.resource.table.conditions })
        .from(object.resource.table)
        .where(eq(object.resource.table.id, declared.id));
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const [deployed] = await database.select().from(object.deployment.table);
    await instances.reconcile(deployed!.id);
    const service = runtime.started[0]?.resources.find((entry) => entry.kind === "service");

    // report the missing address, then run with the declared address and its connecting provider
    expect({
        refused: [refused?.status, refused?.reason, refused?.message],
        declared: [observed!.conditions.ready?.status, observed!.conditions.ready?.reason],
        service: [service?.name, service?.reference, service?.providerCode],
    }).toEqual({
        refused: [
            "false",
            "MissingBinding",
            `bind services ${ids.package} tasks to their addresses`,
        ],
        declared: ["true", "Declared"],
        service: ["tasks", "notes", "http"],
    });
});

test("keep the columns a draining release still declares, and drop them for approval once it retires", async () => {
    // run the first revision of notes, whose notes carry a color
    const database = await openSpace();
    const colored = defineTable("note", {
        id: text("id").primaryKey(),
        body: text("body"),
        color: text("color"),
    });
    let declared: readonly Declared[] = [
        {
            definer: databasePackage,
            description: describeDatabase(defineDatabase({ name: "main", tables: [colored] })),
        },
    ];
    const notes = await installNotes(database);
    const { deployments, instances, resources } = await serveControllers(database, {
        openBuild: (packageId, build) => openNotesWith(declared)(packageId, build),
    });
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const [first] = await database.select().from(object.deployment.table);
    await instances.reconcile(first!.id);

    // deploy a second revision dropping the color beside the first, and plan while the first drains
    declared = [main];
    const second = await submitRevision(database, notes.id, "f");
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const plan = async () => {
        const [row] = await database
            .select()
            .from(object.resource.table)
            .where(eq(object.resource.table.kind, "database"));
        const status = row!.status as {
            state?: { plan?: { steps: { action: string; risk: string; detail: string }[] } };
        };

        return [
            row!.conditions.ready?.reason,
            (status.state?.plan?.steps ?? []).map((step) => [step.action, step.risk, step.detail]),
        ];
    };
    const whileDraining = await plan();

    // run the second revision, stop and retire the first as its instance's changes wake it, and plan again
    const [replacement] = await database
        .select()
        .from(object.deployment.table)
        .where(eq(object.deployment.table.revisionId, second.id));
    await instances.reconcile(replacement!.id);
    await instances.reconcile(first!.id);
    await instances.reconcile(first!.id);
    await resources.reconcile(ids.space);

    // keep the color while the first release serves, then wait for approval to drop it
    expect({ whileDraining, retired: await plan() }).toEqual({
        whileDraining: ["Applied", []],
        retired: ["AwaitingApproval", [["replace", "destructive", "rebuild table: drop color"]]],
    });
});

test("recreate over a draining release whose values a conversion narrows: stop it once approved, convert, then run the replacement", async () => {
    // run the first revision of notes, whose modes include emacs
    const database = await openSpace();
    const modes = (values: [string, ...string[]]) => json("mode", schema.enum(values));
    const first = defineTable("setting", {
        id: text("id").primaryKey(),
        mode: modes(["standard", "vim", "emacs"]),
    });
    const next = { package: { ...first[TABLE].package, version: "2026.10.0" } };
    const narrowed = defineTable(
        "setting",
        { id: text("id").primaryKey(), mode: modes(["standard", "vim"]) },
        {
            convert: {
                "2026.10.0": {
                    mode: Expression.case(
                        Expression.scalar(Expression.column("mode"), "text"),
                        [{ when: "emacs", then: Expression.json("standard") }],
                        Expression.column("mode"),
                    ),
                },
            },
        },
        next,
    );
    const databaseOf = (table: Table) => ({
        definer: databasePackage,
        description: describeDatabase(defineDatabase({ name: "main", tables: [table] })),
    });
    let declared: readonly Declared[] = [databaseOf(first)];
    const notes = await installNotes(database);
    const { deployments, instances, resources, approve } = await serveControllers(database, {
        openBuild: (packageId, build) => openNotesWith(declared)(packageId, build),
    });
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const [original] = await database.select().from(object.deployment.table);
    await instances.reconcile(original!.id);

    // deploy the narrowing revision beside it, planning a recreation that waits for approval
    declared = [databaseOf(narrowed)];
    const second = await submitRevision(database, notes.id, "f");
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const read = async () => {
        const [row] = await database
            .select()
            .from(object.resource.table)
            .where(eq(object.resource.table.kind, "database"));
        const status = row!.status as {
            state?: { plan?: Plan };
        };

        return {
            reason: row!.conditions.ready?.reason,
            plan: status.state?.plan,
            id: row!.id,
        };
    };
    const waiting = await read();

    // approve it through notes, stop the first release, retire it, then convert and run the replacement
    const approved = await approve(notes.id);
    await resources.reconcile(ids.space);
    const recreating = await read();
    const [stopping] = await database
        .select({ status: object.deployment.table.status })
        .from(object.deployment.table)
        .where(eq(object.deployment.table.id, original!.id));
    await instances.reconcile(original!.id);
    await instances.reconcile(original!.id);
    await resources.reconcile(ids.space);
    const converted = await read();
    const [replacement] = await database
        .select()
        .from(object.deployment.table)
        .where(eq(object.deployment.table.revisionId, second.id));
    await instances.reconcile(replacement!.id);
    const statuses = await database
        .select({ id: object.deployment.table.id, status: object.deployment.table.status })
        .from(object.deployment.table)
        .orderBy(object.deployment.table.createdAt, object.deployment.table.id);

    // recreate only once approved, with the first release stopped before converting
    expect({
        waiting: [waiting.reason, waiting.plan?.steps.map((step) => [step.action, step.risk])],
        approved,
        recreating: [recreating.reason, stopping!.status],
        converted: converted.reason,
        statuses: statuses.map((entry) => entry.status),
    }).toEqual({
        waiting: [
            "AwaitingApproval",
            [
                ["delete", "backward-incompatible"],
                ["convert", "data-dependent"],
            ],
        ],
        approved: {
            steps: waiting.plan!.steps.map((step) => ({
                ...step,
                target: `resource/notes.main/${step.target}`,
            })),
        },
        recreating: ["Recreating", "stopping"],
        converted: "Applied",
        statuses: ["retired", "active"],
    });
});

test("keep a release serving while a caller in its space pins it below its replacement's oldest served release, and retire it once the caller retires", async () => {
    // run the first revision of notes
    const database = await openSpace();
    let declared: readonly Declared[] = [main];
    const notes = await installNotes(database);
    const { deployments, instances, resources } = await serveControllers(database, {
        openBuild: (packageId, build) => openNotesWith(declared)(packageId, build),
    });
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const [first] = await database.select().from(object.deployment.table);
    await instances.reconcile(first!.id);

    // bind a caller's active deployment to notes, pinning the first release
    const now = Date.now();
    const record = { scope: ids.space, createdAt: now, updatedAt: now };
    const service = {
        ...record,
        id: identifier("resource").parse("resource-01996ab0-0000-7000-8000-000000000031"),
        name: "calls.notes",
        kind: "service",
        definitionPackageId: servicePackage.id,
        definitionVersion: "2026.9.0",
        definitionName: "notes",
        spec: {},
        origin: "declared" as const,
        providerCode: "http",
        reference: "notes",
    };
    const caller = {
        ...record,
        id: identifier("deployment").parse("deployment-01996ab0-0000-7000-8000-000000000032"),
        installationId: first!.installationId,
        packageId: ids.package,
        revisionId: first!.revisionId,
        output: "server",
        workload: "caller",
        runtime: "bun" as const,
        release: "2026.9.0",
        description: first!.description,
        policies: first!.policies,
        status: "active" as const,
    };
    await database.insert(object.resource.table).values(service);
    await database.insert(object.deployment.table).values(caller);
    await database.insert(object.capture.table).values({
        ...record,
        id: identifier("capture").parse("capture-01996ab0-0000-7000-8000-000000000033"),
        deploymentId: caller.id,
        packageId: ids.package,
        name: "notes",
        target: service.id,
        version: 1,
        state: { release: "2026.9.0" },
    });

    // deploy a second revision whose service serves only releases since 2026.10.0, and run it
    declared = [
        main,
        {
            kind: "service",
            definer: servicePackage,
            description: {
                name: "service",
                since: "2026.10.0",
                protocol: "http",
                api: { name: "service", procedures: [] },
                objects: [],
                routes: [],
            },
        },
    ];
    const second = await submitRevision(database, notes.id, "f");
    await deployments.reconcile(notes.id);
    await resources.reconcile(ids.space);
    const [replacement] = await database
        .select()
        .from(object.deployment.table)
        .where(eq(object.deployment.table.revisionId, second.id));
    await instances.reconcile(replacement!.id);
    await instances.reconcile(first!.id);
    const status = async () =>
        (
            await database
                .select({ status: object.deployment.table.status })
                .from(object.deployment.table)
                .where(eq(object.deployment.table.id, first!.id))
        )[0]!.status;
    const whilePinned = await status();

    // retire the caller, releasing its pin, then stop and retire the first release as its retirement and its instance wake it
    await database
        .update(object.deployment.table)
        .set({ status: "draining" })
        .where(eq(object.deployment.table.id, caller.id));
    await instances.reconcile(caller.id);
    await instances.reconcile(first!.id);
    await instances.reconcile(first!.id);

    // serve the pinned release until its caller retires
    expect({ since: replacement!.since, whilePinned, released: await status() }).toEqual({
        since: "2026.10.0",
        whilePinned: "draining",
        released: "retired",
    });
});

/** A daily schedule the notes build declares, pinning the first note at an hour. */
function daily(hour: number): Declared {
    return {
        kind: "schedule",
        definer: servicePackage,
        description: {
            name: "daily",
            timing: "cron",
            cron: `0 ${hour} * * *`,
            timezone: "UTC",
            concurrency: "forbid",
            deadline: 60_000,
            call: {
                method: "note.update",
                input: { id: "note-1", pinned: true },
                release: "2026.9.0",
            },
        },
    };
}

test("keep exactly the schedules each applied revision's build declares, leaving the ones the installation created", async () => {
    // declare a daily schedule at nine, then at ten, then none, one revision each
    const builds = new Map<string, OpenBuild>([
        ["e", openNotesWith([main, daily(9)])],
        ["f", openNotesWith([main, daily(10)])],
        ["a", openNotesWith([main])],
    ]);
    const openBuild: OpenBuild = (packageId, build) =>
        builds.get((build as { readonly manifest: string }).manifest[0]!)!(packageId, build);
    const database = await openSpace();
    const notes = await installNotes(database);
    const { deployments } = await serveControllers(database, { openBuild });

    // keep a schedule the installation created beside the declared ones
    const now = Date.now();
    await database.insert(object.schedule.table).values({
        id: identifier("schedule").parse(`schedule-${v7()}`),
        scope: ids.space,
        createdAt: now,
        updatedAt: now,
        installation: notes.id,
        name: "reminder",
        timing: { timing: "once", startsAt: now + 60_000 },
        call: { method: "note.update", input: { id: "note-2", pinned: true }, release: "2026.9.0" },
        concurrency: "allow",
        deadline: 60_000,
    });

    // apply each revision, reading the schedules after each
    const schedules = async () =>
        (
            await database.select().from(object.schedule.table).orderBy(object.schedule.table.name)
        ).map((entry) => [
            entry.name,
            entry.packageId,
            entry.timing.timing === "cron" ? entry.timing.cron : entry.timing.timing,
        ]);
    const applied = [];
    for (const character of ["e", "f", "a"]) {
        if (character !== "e") {
            await submitRevision(database, notes.id, character);
        }
        await deployments.reconcile(notes.id);
        applied.push(await schedules());
    }

    expect(applied).toEqual([
        [
            ["daily", ids.package, "0 9 * * *"],
            ["reminder", null, "once"],
        ],
        [
            ["daily", ids.package, "0 10 * * *"],
            ["reminder", null, "once"],
        ],
        [["reminder", null, "once"]],
    ]);
});
