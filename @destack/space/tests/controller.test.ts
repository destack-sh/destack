import { spaceTables } from "../src/stack/index.ts";
import { Plan } from "@destack/resource";
import { expect, onTestFinished, test } from "@destack/test";
import { Scope } from "@destack/sync";
import { existsSync } from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { copyScope } from "@destack/access/test";
import { account } from "@destack/account/object";
import { defineTable, eq, TABLE, type Table, text } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import * as sqlite from "@destack/db/bun";
import { sqliteProvider } from "@destack/db/sqlite";
import { identifier } from "@destack/schema";
import { ControlLoop } from "@destack/service/control";
import { ServiceError } from "@destack/service/error";
import { appliedStates, serveObjects } from "../src/server/index.ts";
import { spaceOptions, reconcileResources } from "./fixture/space.ts";
import {
    binding,
    capture,
    deployment,
    installation,
    installationRevision,
    resource,
    space,
} from "../src/object/index.ts";

/** Identifiers of the fixture space, its host and its resources. */
const ids = {
    account: identifier("account").parse("account-01996ab0-0000-7000-8000-000000000001"),
    space: identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
    region: identifier("region").parse("region-01996ab0-0000-7000-8000-000000000003"),
    host: identifier("host").parse("host-01996ab0-0000-7000-8000-000000000004"),
    package: identifier("package").parse("package-01996ab0-0000-7000-8000-000000000005"),
    kept: identifier("resource").parse("resource-01996ab0-0000-7000-8000-000000000006"),
    removed: identifier("resource").parse("resource-01996ab0-0000-7000-8000-000000000007"),
    installation: identifier("installation").parse(
        "installation-01996ab0-0000-7000-8000-000000000009",
    ),
    binding: identifier("binding").parse("binding-01996ab0-0000-7000-8000-00000000000a"),
    checkout: identifier("checkout").parse("checkout-01996ab0-0000-7000-8000-00000000000b"),
};

/** Notes with a body. */
const noteOne = defineTable("note", { id: text("id").primaryKey(), body: text("body") });

/** Notes after dropping the body. */
const noteTwo = defineTable("note", { id: text("id").primaryKey() });

test("provision SQLite files, keep retained resources, and destroy deleted ones", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-controller-"));
    const database = await sqlite.connect(":memory:", spaceTables);
    onTestFinished(async () => {
        await database.close();
        await rm(directory, { recursive: true });
    });
    await database.migrate(spaceTables);

    // administer a space with one retained and one deletable database
    const now = Date.now();
    await database.insert(space.table).values({
        id: ids.space,
        scope: ids.account,
        name: "personal",
        createdAt: now,
        updatedAt: now,
    });
    await copyScope(database, account.reference(Scope.universe.id, ids.account));
    await copyScope(database, space.reference(ids.account, ids.space));
    const declared = {
        scope: ids.space,
        kind: "database",
        definitionPackageId: ids.package,
        definitionVersion: "1.0.0",
        definitionName: "main",
        spec: { tier: "zonal" },
        createdAt: now,
        updatedAt: now,
    };
    await database.insert(resource.table).values([
        { ...declared, id: ids.kept, name: "kept", retention: "retain" },
        { ...declared, id: ids.removed, name: "removed", retention: "delete" },
    ]);
    const server = serveObjects(
        await spaceOptions(database, {
            providers: [sqliteProvider(pathToFileURL(`${directory}/`))],
        }),
    );
    const read = async (id: typeof ids.kept) =>
        (await database.select().from(resource.table).where(eq(resource.table.id, id)))[0];

    // provision both databases as files and observe their generation
    await reconcileResources(server, ids.space);
    const kept = (await read(ids.kept))!;
    const removed = (await read(ids.removed))!;
    expect([kept.providerCode, kept.observedGeneration, kept.conditions.ready?.reason]).toEqual([
        "sqlite",
        1,
        "Applied",
    ]);
    const isStored = () => existsSync(fileURLToPath(removed.reference!));
    expect(isStored()).toBe(true);

    // keep retained content and destroy deleted content
    await database
        .update(resource.table)
        .set({ deletionRequestedAt: now })
        .where(eq(resource.table.scope, ids.space));
    await reconcileResources(server, ids.space);
    expect((await read(ids.kept))!.conditions.ready?.reason).toBe("Retained");
    expect(await read(ids.removed)).toBeUndefined();
    expect(isStored()).toBe(false);

    // leave the resources alone once a transfer fences the space's scope
    await Scope.fence(database, ids.space, ids.host, now);
    await database
        .update(resource.table)
        .set({ deletionRequestedAt: null })
        .where(eq(resource.table.id, ids.kept));
    await reconcileResources(server, ids.space);
    expect((await read(ids.kept))!.conditions.ready?.reason).toBe("Retained");
});

test("provision a resource as soon as its declaration changes", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-controller-"));
    const database = await sqlite.connect(":memory:", spaceTables);
    const stopping = new AbortController();
    onTestFinished(async () => {
        stopping.abort();
        await database.close();
        await rm(directory, { recursive: true });
    });
    await database.migrate(spaceTables);

    // run the controller's loop on an empty space
    const now = Date.now();
    await database.insert(space.table).values({
        id: ids.space,
        scope: ids.account,
        name: "personal",
        createdAt: now,
        updatedAt: now,
    });
    await copyScope(database, account.reference(Scope.universe.id, ids.account));
    await copyScope(database, space.reference(ids.account, ids.space));
    const server = serveObjects(
        await spaceOptions(database, {
            providers: [sqliteProvider(pathToFileURL(`${directory}/`))],
        }),
    );
    const running = new ControlLoop(database, server.controllers(), {
        report: (_controller, _key, error) => {
            throw error;
        },
    }).run(stopping.signal);

    // declare a database and wait for the watched change to provision it
    await database.insert(resource.table).values({
        id: ids.kept,
        scope: ids.space,
        name: "main",
        kind: "database",
        definitionPackageId: ids.package,
        definitionVersion: "1.0.0",
        definitionName: "main",
        spec: { tier: "zonal" },
        createdAt: now,
        updatedAt: now,
    });
    await expect
        .poll(
            async () =>
                (
                    await database
                        .select()
                        .from(resource.table)
                        .where(eq(resource.table.id, ids.kept))
                )[0]?.conditions.ready?.reason,
            { timeout: 5000, interval: 20 },
        )
        .toBe("Applied");
    stopping.abort();
    await running;
});

test("apply bound desired states, wait for approval of a destructive plan, then apply the approved plan, reading applied states only once applied", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-controller-"));
    const database = await sqlite.connect(":memory:", spaceTables);
    onTestFinished(async () => {
        await database.close();
        await rm(directory, { recursive: true });
    });
    await database.migrate(spaceTables);

    // administer a space with one database bound by one installation
    const now = Date.now();
    await database.insert(space.table).values({
        id: ids.space,
        scope: ids.account,
        name: "personal",
        createdAt: now,
        updatedAt: now,
    });
    await copyScope(database, account.reference(Scope.universe.id, ids.account));
    await copyScope(database, space.reference(ids.account, ids.space));
    await database.insert(resource.table).values({
        id: ids.kept,
        scope: ids.space,
        name: "main",
        kind: "database",
        definitionPackageId: ids.package,
        definitionVersion: "1.0.0",
        definitionName: "main",
        spec: { tier: "zonal" },
        createdAt: now,
        updatedAt: now,
    });
    await database.insert(installation.table).values({
        id: ids.installation,
        scope: ids.space,
        packageId: ids.package,
        role: "application",
        alias: "notes",
        selection: { kind: "checkout", host: ids.host, checkout: ids.checkout, directory: "." },
        createdAt: now,
        updatedAt: now,
    });
    const bind = async (tables: readonly Table[]) =>
        await database
            .insert(binding.table)
            .values({
                id: ids.binding,
                scope: ids.space,
                installationId: ids.installation,
                packageId: ids.package,
                name: "main",
                target: ids.kept,
                state: defineDatabase({
                    name: "main",
                    tables,
                }).state(),
                createdAt: now,
                updatedAt: now,
            })
            .onConflictDoUpdate({
                target: binding.table.id,
                set: {
                    state: defineDatabase({
                        name: "main",
                        tables,
                    }).state(),
                },
            });
    const server = serveObjects(
        await spaceOptions(database, {
            providers: [sqliteProvider(pathToFileURL(`${directory}/`))],
        }),
    );
    const read = async () =>
        (await database.select().from(resource.table).where(eq(resource.table.id, ids.kept)))[0]!;

    // create the bound table without approval, since the plan is safe
    await bind([noteOne]);
    await reconcileResources(server, ids.space);
    expect((await read()).conditions.ready?.reason).toBe("Applied");
    const [bound] = await database.select().from(binding.table);
    expect(await appliedStates(database, await read())).toEqual([bound!.state]);

    // wait for approval of a plan dropping a column, recording the plan for review
    await bind([noteTwo]);
    await reconcileResources(server, ids.space);
    const waiting = await read();
    const plan = waiting.status.state!.plan!;
    expect([waiting.conditions.ready?.reason, plan.steps]).toEqual([
        "AwaitingApproval",
        [
            {
                action: "replace",
                target: `table/${noteTwo[TABLE].sqlName}`,
                risk: "destructive",
                detail: "rebuild table: drop body",
            },
        ],
    ]);

    // leave the waiting resource untouched when reconciled again, since each write would wake the controller
    await reconcileResources(server, ids.space);
    expect(await read()).toEqual(waiting);

    // refuse reading the applied states while the plan waits, since the resource has neither state whole
    await expect(appliedStates(database, waiting)).rejects.toEqual(
        new ServiceError("PRECONDITION_FAILED", {
            message: `resource ${ids.kept} has not applied its bindings' desired states: approve or reject its plan first`,
        }),
    );

    // apply the approved plan and forget the approval
    await database
        .update(resource.table)
        .set({ approvedPlan: await Plan.digest(plan) })
        .where(eq(resource.table.id, ids.kept));
    await reconcileResources(server, ids.space);
    const applied = await read();
    const [rebound] = await database.select().from(binding.table);
    expect([
        applied.conditions.ready?.reason,
        applied.approvedPlan,
        await appliedStates(database, applied),
    ]).toEqual(["Applied", null, [rebound!.state]]);
});

test("retry a provisioning that failed, and keep a deleted resource while a live deployment runs with it", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-controller-"));
    const database = await sqlite.connect(":memory:", spaceTables);
    onTestFinished(async () => {
        await database.close();
        await rm(directory, { recursive: true });
    });
    await database.migrate(spaceTables);

    // administer a space whose provider fails its first provisioning
    const now = Date.now();
    await database.insert(space.table).values({
        id: ids.space,
        scope: ids.account,
        name: "personal",
        createdAt: now,
        updatedAt: now,
    });
    await copyScope(database, account.reference(Scope.universe.id, ids.account));
    await copyScope(database, space.reference(ids.account, ids.space));
    const base = sqliteProvider(pathToFileURL(`${directory}/`));
    let isFailing = true;
    const flaky: typeof base = {
        ...base,
        provision: async (record) => {
            if (isFailing) {
                isFailing = false;
                throw new Error("the disk is full");
            }

            return base.provision(record);
        },
    };
    await database.insert(resource.table).values({
        id: ids.removed,
        scope: ids.space,
        name: "removed",
        retention: "delete",
        kind: "database",
        definitionPackageId: ids.package,
        definitionVersion: "1.0.0",
        definitionName: "main",
        spec: { tier: "zonal" },
        createdAt: now,
        updatedAt: now,
    });
    const server = serveObjects(await spaceOptions(database, { providers: [flaky] }));
    const read = async () =>
        (await database.select().from(resource.table).where(eq(resource.table.id, ids.removed)))[0];

    // record the failure and throw it for the loop's backoff, then provision on the next attempt
    const failed = await reconcileResources(server, ids.space).then(
        () => "reconciled",
        (error: Error) => error.message,
    );
    const failure = (await read())!.conditions.ready?.reason;
    await reconcileResources(server, ids.space);
    expect([failed, failure, (await read())!.conditions.ready?.reason]).toEqual([
        "the disk is full",
        "ProvisioningFailed",
        "Applied",
    ]);

    // run a live deployment with the resource, then request its deletion
    await database.insert(installation.table).values({
        id: ids.installation,
        scope: ids.space,
        packageId: ids.package,
        role: "application",
        alias: "notes",
        selection: { kind: "checkout", host: ids.host, checkout: ids.checkout, directory: "." },
        createdAt: now,
        updatedAt: now,
    });
    const revisionId = identifier("installation-revision").parse(
        "installation-revision-01996ab0-0000-7000-8000-00000000000c",
    );
    await database.insert(installationRevision.table).values({
        id: revisionId,
        scope: ids.space,
        installationId: ids.installation,
        build: { kind: "release", version: "2026.9.0" },
        views: {},
        settings: [],
        digest: "e".repeat(64),
        createdAt: now,
        updatedAt: now,
    } as never);
    const deploymentId = identifier("deployment").parse(
        "deployment-01996ab0-0000-7000-8000-00000000000d",
    );
    await database.insert(deployment.table).values({
        id: deploymentId,
        scope: ids.space,
        installationId: ids.installation,
        packageId: ids.package,
        revisionId,
        output: "main",
        workload: "main",
        runtime: "bun",
        release: "2026.9.0",
        description: {
            entrypoint: ".",
            services: [],
            triggers: [],
            resources: [],
            secrets: [],
            connections: [],
            compute: {},
        },
        policies: { packages: [], network: [] },
        createdAt: now,
        updatedAt: now,
    } as never);
    await database.insert(capture.table).values({
        id: identifier("capture").parse("capture-01996ab0-0000-7000-8000-00000000000e"),
        scope: ids.space,
        deploymentId,
        packageId: ids.package,
        name: "main",
        target: ids.removed,
        version: 1,
        state: {},
        createdAt: now,
        updatedAt: now,
    } as never);
    await database
        .update(resource.table)
        .set({ deletionRequestedAt: now })
        .where(eq(resource.table.id, ids.removed));

    // keep it while the deployment runs, and destroy it once the deployment retires
    await reconcileResources(server, ids.space);
    const kept = (await read())!.conditions.ready;
    await database
        .update(deployment.table)
        .set({ status: "retired", retiredAt: now })
        .where(eq(deployment.table.id, deploymentId));
    await reconcileResources(server, ids.space);
    expect([kept?.reason, kept?.message, await read()]).toEqual([
        "InUse",
        `captured by ${deploymentId}`,
        undefined,
    ]);
});
