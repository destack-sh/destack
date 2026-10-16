import { Plan } from "@destack/resource";
import { expect, onTestFinished, test } from "@destack/test";
import { Scope } from "@destack/sync";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import {
    databaseProvider,
    defineDatabase,
    defineTable,
    eq,
    TABLE,
    type Table,
    text,
    Transaction,
} from "@destack/db";
import * as sqlite from "@destack/db/bun";
import { nth, schema, present } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { appliedStates, ConsumerController, serveSpace } from "../index.ts";
import {
    reconcileResources,
    ids,
    spaceDatabase,
    StackFixture,
    settle,
    TestSpaceDatabaseHost,
} from "../../test/index.ts";
import { principal } from "@destack/access";
import { RequestId } from "@destack/runtime/request";
import * as object from "../../object/index.ts";
import {
    binding,
    capture,
    deployment,
    installation,
    installationRevision,
} from "../../object/index.ts";

/** The resources and records the scenarios name beside the fixture's space. */
const named = {
    kept: schema.identifier("database").parse("database-01996ab0-0000-7000-8000-000000000006"),
    removed: schema.identifier("database").parse("database-01996ab0-0000-7000-8000-000000000007"),
    installation: schema
        .identifier("installation")
        .parse("installation-01996ab0-0000-7000-8000-000000000009"),
    binding: schema.identifier("binding").parse("binding-01996ab0-0000-7000-8000-00000000000a"),
};

/** Keep the space's databases as SQLite files below a directory, main as the space does. */
function providers(directory: string) {
    const root = pathToFileURL(`${directory}/`);

    return [
        databaseProvider(new sqlite.BunSqliteDatabaseHost(root), object.database),
        databaseProvider(new TestSpaceDatabaseHost(root), object.database),
    ] as const;
}

/** Notes with a body. */
const noteOne = defineTable("note", { id: text("id").primaryKey(), body: text("body") });

/** Notes after dropping the body. */
const noteTwo = defineTable("note", { id: text("id").primaryKey() });

test("provision a space's database under the space's approval threshold, and leave it alone while a move fences the space", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-controller-"));
    const fixture = await StackFixture.open();
    const { database } = fixture;
    onTestFinished(async () => {
        await rm(directory, { recursive: true });
    });

    // declare a database in the space, served by a SQLite provider
    const now = Date.now();
    await database.insert(object.database.table).values({
        scope: ids.space,
        definitionPackageId: ids.package,
        definitionVersion: "1.0.0",
        definitionName: "kept",
        spec: { replicated: [] },
        createdAt: now,
        updatedAt: now,
        id: named.kept,
        name: "kept",
        retention: "forever",
    });
    const store = serveSpace(
        await fixture.options({
            providers: providers(directory),
        }),
    ).store;
    const stored = async () =>
        nth(
            await database
                .select()
                .from(object.database.table)
                .where(eq(object.database.table.id, named.kept)),
            0,
        );

    // provision it as a file at the generation carrying the space's approval threshold
    await reconcileResources(store, ids.space);
    const provisioned = await stored();

    // leave it alone once a transfer fences the space's scope, even when its deletion is requested
    await Transaction.run(database, (transaction) =>
        Scope.fence(transaction, ids.space, ids.machine, now),
    );
    await database
        .update(object.database.table)
        .set({ deletedAt: now })
        .where(eq(object.database.table.id, named.kept));
    await reconcileResources(store, ids.space);
    const fenced = await stored();
    expect([
        provisioned.provider,
        provisioned.observedGeneration,
        provisioned.conditions["ready"]?.reason,
        fenced.conditions["ready"]?.reason,
    ]).toEqual(["sqlite", 2, "Applied", "Applied"]);
});

test("provision a resource as soon as its declaration changes", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-controller-"));
    const fixture = await StackFixture.open();
    const { database } = fixture;
    onTestFinished(async () => {
        await rm(directory, { recursive: true });
    });

    // serve the database controller on an empty space
    const now = Date.now();
    const store = serveSpace(
        await fixture.options({
            providers: providers(directory),
        }),
    ).store;

    // declare a database and provision it from the change its insert commits
    const declared = await database.log.position();
    await database.insert(object.database.table).values({
        id: named.kept,
        scope: ids.space,
        name: "archive",
        definitionPackageId: ids.package,
        definitionVersion: "1.0.0",
        definitionName: "archive",
        spec: { replicated: [] },
        createdAt: now,
        updatedAt: now,
    });
    await settle(
        database,
        [...store.controllers(), new ConsumerController(store, [object.database], spaceDatabase)],
        declared,
    );
    const [provisioned] = await database
        .select()
        .from(object.database.table)
        .where(eq(object.database.table.id, named.kept));
    expect(provisioned?.conditions["ready"]?.reason).toBe("Applied");
});

test("apply a resource's bound states, waiting for approval of a destructive plan", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-controller-"));
    const fixture = await StackFixture.open();
    const { database } = fixture;
    onTestFinished(async () => {
        await rm(directory, { recursive: true });
    });

    // administer a space with one database bound by one installation
    const now = Date.now();
    await database.insert(object.database.table).values({
        id: named.kept,
        scope: ids.space,
        name: "archive",
        definitionPackageId: ids.package,
        definitionVersion: "1.0.0",
        definitionName: "archive",
        spec: { replicated: [] },
        createdAt: now,
        updatedAt: now,
    });
    await database.insert(installation.table).values({
        id: named.installation,
        scope: ids.space,
        packageId: ids.package,
        role: "application",
        name: "notes",
        selection: {
            kind: "checkout",
            machine: ids.machine,
            checkout: ids.checkout,
            directory: ".",
        },
        createdAt: now,
        updatedAt: now,
    });
    const bind = async (tables: readonly Table[]) =>
        await database
            .insert(binding.table)
            .values({
                id: named.binding,
                scope: ids.space,
                installationId: named.installation,
                packageId: ids.package,
                name: "archive",
                target: named.kept,
                state: defineDatabase({
                    name: "archive",
                    tables,
                }).state(),
                createdAt: now,
                updatedAt: now,
            })
            .onConflictDoUpdate({
                target: binding.table.id,
                set: {
                    state: defineDatabase({
                        name: "archive",
                        tables,
                    }).state(),
                },
            });
    const store = serveSpace(
        await fixture.options({
            providers: providers(directory),
        }),
    ).store;
    const read = async () =>
        nth(
            await database
                .select()
                .from(object.database.table)
                .where(eq(object.database.table.id, named.kept)),
            0,
        );

    // create the bound table of a safe plan without approval
    await bind([noteOne]);
    await reconcileResources(store, ids.space);
    expect((await read()).conditions["ready"]?.reason).toBe("Applied");
    const [bound] = await database.select().from(binding.table);
    expect(await appliedStates(database, await read())).toEqual([
        present(bound, "the binding").state,
    ]);

    // wait for approval of a plan dropping a column and record the plan for review
    await bind([noteTwo]);
    await reconcileResources(store, ids.space);
    const waiting = await read();
    const plan = present(waiting.plan, "the waiting plan");
    expect([waiting.conditions["ready"]?.reason, plan.steps]).toEqual([
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

    // leave the waiting resource untouched when reconciled again
    await reconcileResources(store, ids.space);
    expect(await read()).toEqual(waiting);

    // refuse reading the applied states while the plan waits
    await expect(appliedStates(database, waiting)).rejects.toEqual(
        new CallError("PRECONDITION_FAILED", {
            message: `resource ${named.kept} has not applied its desired states: approve or reject its plan first`,
        }),
    );

    // apply the approved plan and forget the approval
    await database
        .update(object.database.table)
        .set({ approvedPlan: await Plan.digest(plan) })
        .where(eq(object.database.table.id, named.kept));
    await reconcileResources(store, ids.space);
    const applied = await read();
    const [rebound] = await database.select().from(binding.table);
    expect([
        applied.conditions["ready"]?.reason,
        applied.approvedPlan,
        await appliedStates(database, applied),
    ]).toEqual(["Applied", null, [present(rebound, "the rebound binding").state]]);
});

test("retry a provisioning that failed, and keep a deleted resource while its installation consumes it through a live deployment", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-controller-"));
    const fixture = await StackFixture.open();
    const { database } = fixture;
    onTestFinished(async () => {
        await rm(directory, { recursive: true });
    });

    // administer a space whose provider fails its first provisioning
    const now = Date.now();
    const [base, main] = providers(directory);
    let isFailing = true;
    const flaky: typeof base = {
        ...base,
        provision: {
            provision: async (record) => {
                if (isFailing) {
                    isFailing = false;
                    throw new Error("the disk is full");
                }

                return base.provision.provision(record);
            },
            destroy: (record) => base.provision.destroy(record),
        },
    };
    await database.insert(object.database.table).values({
        id: named.removed,
        scope: ids.space,
        name: "removed",
        retention: "delete",
        definitionPackageId: ids.package,
        definitionVersion: "1.0.0",
        definitionName: "removed",
        spec: { replicated: [] },
        createdAt: now,
        updatedAt: now,
    });
    const store = serveSpace(await fixture.options({ providers: [flaky, main] })).store;
    const read = async () =>
        (
            await database
                .select()
                .from(object.database.table)
                .where(eq(object.database.table.id, named.removed))
        )[0];

    // record and throw the failure and provision on the next attempt
    const failed = await reconcileResources(store, ids.space).then(
        () => "reconciled",
        (error: unknown) => (error instanceof Error ? error.message : error),
    );
    const failure = (await read())?.conditions["ready"]?.reason;
    await reconcileResources(store, ids.space);
    expect([failed, failure, (await read())?.conditions["ready"]?.reason]).toEqual([
        "the disk is full",
        "ProvisioningFailed",
        "Applied",
    ]);

    // run a live deployment with the resource and request its deletion
    await database.insert(installation.table).values({
        id: named.installation,
        scope: ids.space,
        packageId: ids.package,
        role: "application",
        name: "notes",
        selection: {
            kind: "checkout",
            machine: ids.machine,
            checkout: ids.checkout,
            directory: ".",
        },
        createdAt: now,
        updatedAt: now,
    });
    const revisionId = schema
        .identifier("installation-revision")
        .parse("installation-revision-01996ab0-0000-7000-8000-00000000000c");
    await database.insert(installationRevision.table).values({
        id: revisionId,
        scope: ids.space,
        installationId: named.installation,
        build: { kind: "release", version: "2026.9.0", manifest: "e".repeat(64) },
        views: {},
        settings: [],
        catalogs: [],
        digest: "e".repeat(64),
        createdAt: now,
        updatedAt: now,
    });
    const deploymentId = schema
        .identifier("deployment")
        .parse("deployment-01996ab0-0000-7000-8000-00000000000d");
    await database.insert(deployment.table).values({
        id: deploymentId,
        scope: ids.space,
        activatedAt: now,
        installationId: named.installation,
        packageId: ids.package,
        revisionId,
        output: "main",
        workload: "main",
        runtime: "bun",
        release: "2026.9.0",
        description: {
            entrypoint: ".",
            packages: [],
            triggers: [],
            resources: [],
            secrets: [],
            compute: {},
            capabilities: {},
            permissions: {},
        },
        createdAt: now,
        updatedAt: now,
    });
    await database.insert(capture.table).values({
        id: schema.identifier("capture").parse("capture-01996ab0-0000-7000-8000-00000000000e"),
        scope: ids.space,
        deploymentId,
        packageId: ids.package,
        name: "removed",
        target: named.removed,
        version: 1,
        state: defineDatabase({ name: "removed", tables: [] }).state(),
        createdAt: now,
        updatedAt: now,
    });
    await database
        .update(object.database.table)
        .set({ deletedAt: now })
        .where(eq(object.database.table.id, named.removed));

    // relate the installation as consumer of what its live deployment captured, as its deploy does
    const consumer = principal.installation.reference(ids.space, named.installation);
    const relate = (consumed: readonly string[]) =>
        store.mutate(store.self, {
            id: RequestId.create(Date.now()),
            calls: [
                object.database.call("bind", {
                    ...object.database.in(ids.space),
                    consumer,
                    ids: [...consumed],
                }),
            ],
        });
    await relate([named.removed]);

    // keep it while the installation consumes it and destroy it once the deployment retires
    await reconcileResources(store, ids.space);
    const kept = (await read())?.conditions["ready"];
    await database
        .update(deployment.table)
        .set({ status: "retired", retiredAt: now })
        .where(eq(deployment.table.id, deploymentId));
    await relate([]);
    await reconcileResources(store, ids.space);
    expect([kept?.reason, kept?.message, await read()]).toEqual([
        "InUse",
        `consumed by ${named.installation}`,
        undefined,
    ]);
});
