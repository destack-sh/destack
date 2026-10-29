import type { Environment } from "@destack/environment";
import { Plan } from "@destack/resource";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Scope, RequestId } from "@destack/sync";
import { pathToFileURL } from "node:url";
import {
    DatabaseKind,
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
import { nth, schema, present, Digest } from "@destack/schema";
import { appliedStates, ConsumerController, ProviderIndex, serveSpace } from "../index.ts";
import { testHost } from "@destack/test/host";
import {
    ids,
    insertRelease,
    insertRollout,
    reconcileResources,
    settle,
    spaceDatabase,
    TestSpace,
    TestSpaceDatabaseHost,
} from "../../test/index.ts";
import { principal } from "@destack/access";
import * as object from "../../object/index.ts";
import { binding, capture, deployment, installationRevision } from "../../object/index.ts";

/** The resources and records of the scenarios, in addition to the fixture's space. */
const named = {
    kept: schema.identifier("database").parse("database-01996ab0-0000-7000-8000-000000000006"),
    removed: schema.identifier("database").parse("database-01996ab0-0000-7000-8000-000000000007"),
    installation: schema
        .identifier("installation")
        .parse("installation-01996ab0-0000-7000-8000-000000000009"),
    binding: schema.identifier("binding").parse("binding-01996ab0-0000-7000-8000-00000000000a"),
};

/** Store the space's databases as SQLite files below a directory, main as the space does. */
function providers(directory: string, environment: Environment<"files" | "server">) {
    const root = pathToFileURL(`${directory}/`);

    return [
        databaseProvider(new sqlite.BunSqliteDatabaseHost(root, environment), object.database),
        databaseProvider(new TestSpaceDatabaseHost(root, environment), object.database),
    ] as const;
}

/** Notes with a body. */
const noteOne = defineTable("note", { id: text("id").primaryKey(), body: text("body") });

/** Notes after dropping the body. */
const noteTwo = defineTable("note", { id: text("id").primaryKey() });

test("provision a space's database under the space's approval threshold, and leave it alone while a move fences the space", async () => {
    const fixture = await TestSpace.open();
    const directory = await fixture.environment.files.makeTemporaryDirectory("destack-controller-");
    const { database } = fixture;
    onTestFinished(async () => {
        await fixture.environment.files.remove(directory, { recursive: true });
    });

    // declare a database in the space, served by a SQLite provider
    const now = fixture.machine.clock.now();
    await database.transaction(
        async (transaction) =>
            await transaction.insert(object.database.table).values({
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
            }),
    );
    const store = serveSpace(
        await fixture.options({
            providers: providers(directory, fixture.environment),
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
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(object.database.table)
                .set({ deletedAt: now })
                .where(eq(object.database.table.id, named.kept)),
    );
    await reconcileResources(store, ids.space);
    const fenced = await stored();
    expect([
        provisioned.provider,
        provisioned.conditions["Ready"]?.observedGeneration,
        provisioned.conditions["Ready"]?.reason,
        fenced.conditions["Ready"]?.reason,
    ]).toEqual(["sqlite", 2, "Applied", "Applied"]);
});

test("provision a resource as soon as its declaration changes", async () => {
    const fixture = await TestSpace.open();
    const directory = await fixture.environment.files.makeTemporaryDirectory("destack-controller-");
    const { database } = fixture;
    onTestFinished(async () => {
        await fixture.environment.files.remove(directory, { recursive: true });
    });

    // serve the database controller on an empty space
    const now = fixture.machine.clock.now();
    const store = serveSpace(
        await fixture.options({
            providers: providers(directory, fixture.environment),
        }),
    ).store;

    // declare a database and provision it from the change its insert commits
    const declared = await database.log.position();
    await database.transaction(
        async (transaction) =>
            await transaction.insert(object.database.table).values({
                id: named.kept,
                scope: ids.space,
                name: "archive",
                definitionPackageId: ids.package,
                definitionVersion: "1.0.0",
                definitionName: "archive",
                spec: { replicated: [] },
                createdAt: now,
                updatedAt: now,
            }),
    );
    await settle(
        database,
        [...store.controllers(), new ConsumerController(store, [object.database], spaceDatabase)],
        declared,
    );
    const [provisioned] = await database
        .select()
        .from(object.database.table)
        .where(eq(object.database.table.id, named.kept));
    expect(provisioned?.conditions["Ready"]?.reason).toBe("Applied");
});

test("apply a resource's bound states, waiting for approval of a destructive plan", async () => {
    const fixture = await TestSpace.open();
    const directory = await fixture.environment.files.makeTemporaryDirectory("destack-controller-");
    const { database } = fixture;
    onTestFinished(async () => {
        await fixture.environment.files.remove(directory, { recursive: true });
    });

    // administer a space with one database bound by one installation
    const now = fixture.machine.clock.now();
    await database.transaction(
        async (transaction) =>
            await transaction.insert(object.database.table).values({
                id: named.kept,
                scope: ids.space,
                name: "archive",
                definitionPackageId: ids.package,
                definitionVersion: "1.0.0",
                definitionName: "archive",
                spec: { replicated: [] },
                createdAt: now,
                updatedAt: now,
            }),
    );
    await insertRollout(
        database,
        {
            id: named.installation,
            scope: ids.space,
            packageId: ids.package,
            name: "notes",
            selection: {
                kind: "checkout",
                machine: ids.machine,
                checkout: ids.checkout,
                directory: ".",
            },
            installer: fixture.owner.subject,
        },
        now,
    );
    const bind = async (tables: readonly Table[]) => {
        // write the tables' state as a blob, then bind it
        const state = await fixture.blobs.writeJson(
            defineDatabase({ name: "archive", tables }).state(),
        );
        await database.transaction(
            async (transaction) =>
                await transaction
                    .insert(binding.table)
                    .values({
                        id: named.binding,
                        scope: ids.space,
                        installationId: named.installation,
                        packageId: ids.package,
                        name: "archive",
                        target: named.kept,
                        state,
                        createdAt: now,
                        updatedAt: now,
                    })
                    .onConflictDoUpdate({ target: binding.table.id, set: { state } }),
        );
    };
    const store = serveSpace(
        await fixture.options({
            providers: providers(directory, fixture.environment),
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
    expect((await read()).conditions["Ready"]?.reason).toBe("Applied");
    const [bound] = await database.select().from(binding.table);
    expect(await appliedStates(store.blobs, await read())).toEqual([
        await fixture.blobs.readJson(present(bound, "the binding").state),
    ]);

    // wait for approval of a plan dropping a column and record the plan for review
    await bind([noteTwo]);
    await reconcileResources(store, ids.space);
    const waiting = await read();
    const plan = present(waiting.plan, "the waiting plan");
    expect([waiting.conditions["Ready"]?.reason, plan.steps]).toEqual([
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
    expect(await refusal(appliedStates(store.blobs, waiting))).toEqual([
        "PRECONDITION_FAILED",
        "PLAN_UNAPPLIED",
        `resource ${named.kept} has not applied its desired states: approve or reject its plan first`,
    ]);

    // apply the approved plan and forget the approval
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(object.database.table)
                .set({ approvedPlan: await Plan.digest(plan, database.state.environment.crypto) })
                .where(eq(object.database.table.id, named.kept)),
    );
    await reconcileResources(store, ids.space);
    const applied = await read();
    const [rebound] = await database.select().from(binding.table);
    expect([
        applied.conditions["Ready"]?.reason,
        applied.approvedPlan,
        await appliedStates(store.blobs, applied),
    ]).toEqual([
        "Applied",
        null,
        [await fixture.blobs.readJson(present(rebound, "the rebound binding").state)],
    ]);
});

test("retry a provisioning that failed, and retain a deleted resource while its installation consumes it through a live deployment", async () => {
    const fixture = await TestSpace.open();
    const directory = await fixture.environment.files.makeTemporaryDirectory("destack-controller-");
    const { database } = fixture;
    onTestFinished(async () => {
        await fixture.environment.files.remove(directory, { recursive: true });
    });

    // administer a space whose provider fails its first provisioning
    const now = fixture.machine.clock.now();
    const [base, main] = providers(directory, fixture.environment);
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
    await database.transaction(
        async (transaction) =>
            await transaction.insert(object.database.table).values({
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
            }),
    );
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
    const failure = (await read())?.conditions["Ready"]?.reason;
    await reconcileResources(store, ids.space);
    expect([failed, failure, (await read())?.conditions["Ready"]?.reason]).toEqual([
        "the disk is full",
        "ProvisioningFailed",
        "Applied",
    ]);

    // run a live deployment with the resource and request its deletion
    await insertRollout(
        database,
        {
            id: named.installation,
            scope: ids.space,
            packageId: ids.package,
            name: "notes",
            selection: {
                kind: "checkout",
                machine: ids.machine,
                checkout: ids.checkout,
                directory: ".",
            },
            installer: fixture.owner.subject,
        },
        now,
    );
    const revisionId = schema
        .identifier("installation-revision")
        .parse("installation-revision-01996ab0-0000-7000-8000-00000000000c");
    const release = {
        package: ids.package,
        version: "2026.9.0",
        manifest: Digest.parse("e".repeat(64)),
    };
    const pulled = await insertRelease(
        database,
        ids.space,
        release,
        [],
        fixture.machine.environment,
    );
    await database.transaction(
        async (transaction) =>
            await transaction.insert(installationRevision.table).values({
                id: revisionId,
                scope: ids.space,
                installationId: named.installation,
                buildId: pulled.id,
                dependencies: [],
                digest: Digest.parse("e".repeat(64)),
                createdAt: now,
                updatedAt: now,
            }),
    );
    const deploymentId = schema
        .identifier("deployment")
        .parse("deployment-01996ab0-0000-7000-8000-00000000000d");
    await database.transaction(
        async (transaction) =>
            await transaction.insert(deployment.table).values({
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
                    sandbox: {},
                },
                createdAt: now,
                updatedAt: now,
            }),
    );
    const removedState = await fixture.blobs.writeJson(
        defineDatabase({ name: "removed", tables: [] }).state(),
    );
    await database.transaction(
        async (transaction) =>
            await transaction.insert(capture.table).values({
                id: schema
                    .identifier("capture")
                    .parse("capture-01996ab0-0000-7000-8000-00000000000e"),
                scope: ids.space,
                deploymentId,
                packageId: ids.package,
                name: "removed",
                target: named.removed,
                version: 1,
                state: removedState,
                createdAt: now,
                updatedAt: now,
            }),
    );
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(object.database.table)
                .set({ deletedAt: now })
                .where(eq(object.database.table.id, named.removed)),
    );

    // relate the installation as consumer of what its live deployment captured, as its deploy does
    const consumer = principal.installation.reference(ids.space, named.installation);
    const relate = (consumed: readonly string[]) =>
        store.mutate(store.as(object.database), {
            id: RequestId.create(fixture.machine.environment),
            calls: [
                object.database.call("bind", {
                    ...object.database.in(ids.space),
                    consumer,
                    ids: [...consumed],
                }),
            ],
        });
    await relate([named.removed]);

    // retain it while the installation consumes it, and destroy it once the deployment retires
    await reconcileResources(store, ids.space);
    const kept = (await read())?.conditions["Ready"];
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(deployment.table)
                .set({ status: "retired", retiredAt: now })
                .where(eq(deployment.table.id, deploymentId)),
    );
    await relate([]);
    await reconcileResources(store, ids.space);
    expect([kept?.reason, kept?.message, await read()]).toEqual([
        "InUse",
        `consumed by ${named.installation}`,
        undefined,
    ]);
});

test("hand a workload a local database at its file below the provider's root, and a remote one's reference as it is", async () => {
    // index a local SQLite provider below a root, and a provider whose references are remote addresses
    const root = pathToFileURL("/machine/databases/");
    const local = databaseProvider(
        new sqlite.BunSqliteDatabaseHost(root, testHost()),
        object.database,
    );
    const remote = { kind: DatabaseKind, code: "postgres", object: object.database };
    const index = new ProviderIndex(ids.machine, [local, remote]);

    // resolve each reference to where a workload on this machine opens it
    const reached = (provider: string, reference: string) =>
        index.resolve({ kind: DatabaseKind.name, provider, reference });
    expect([
        reached("sqlite", `${ids.space}/database-01996ab0-0000-7000-8000-000000000006.db`),
        reached("postgres", "postgres://database.test/notes"),
    ]).toEqual([
        `file:///machine/databases/${ids.space}/database-01996ab0-0000-7000-8000-000000000006.db`,
        "postgres://database.test/notes",
    ]);
});
