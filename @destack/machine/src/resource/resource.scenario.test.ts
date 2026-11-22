import type { Environment } from "@destack/environment";
import { expect, onTestFinished, test } from "@destack/test";
import { Scope, ObjectReference } from "@destack/sync";
import { pathToFileURL } from "node:url";
import { DatabaseKind, eq } from "@destack/db";
import { databaseProvider } from "@destack/db/database";
import * as sqlite from "@destack/db/bun";
import { nth, schema } from "@destack/schema";
import { ProviderIndex } from "../index.ts";
import { testHost } from "@destack/test/host";
import { ids, TestSpace } from "../test/index.ts";
import { principal } from "@destack/access";
import * as object from "@destack/stack/object";

/** The resources and records of the scenarios, in addition to the fixture's space. */
const named = {
    kept: schema.identifier("database").parse("database-01996ab0-0000-7000-8000-000000000006"),
    removed: schema.identifier("database").parse("database-01996ab0-0000-7000-8000-000000000007"),
    installation: schema
        .identifier("installation")
        .parse("installation-01996ab0-0000-7000-8000-000000000009"),
};

/** Store the space's other databases as SQLite files below a directory. */
function providers(directory: string, environment: Environment<"files" | "server">) {
    const root = pathToFileURL(`${directory}/`);

    return [
        databaseProvider(new sqlite.BunSqliteDatabaseHost(root, environment), object.database),
    ] as const;
}

test("provision a space's database, and leave it alone while its space is fenced", async () => {
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
                ...ObjectReference.columns(
                    "owner",
                    principal.space.reference(Scope.universe.id, ids.space),
                ),
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
    await fixture.serve();
    const stored = async () =>
        nth(
            await database
                .select()
                .from(object.database.table)
                .where(eq(object.database.table.id, named.kept)),
            0,
        );

    // provision it as a file at the generation carrying the space's approval threshold
    await fixture.simulation.idle();
    const provisioned = await stored();

    // leave it alone once a transfer fences the space's scope, even when its deletion is requested
    await database.commit((transaction) => Scope.fence(transaction, ids.space, ids.machine, now));
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(object.database.table)
                .set({ deletedAt: now })
                .where(eq(object.database.table.id, named.kept)),
    );
    await fixture.simulation.idle();
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
    await fixture.serve();

    // declare a database and provision it from the change its insert commits
    await database.transaction(
        async (transaction) =>
            await transaction.insert(object.database.table).values({
                id: named.kept,
                scope: ids.space,
                ...ObjectReference.columns(
                    "owner",
                    principal.space.reference(Scope.universe.id, ids.space),
                ),
                name: "archive",
                definitionPackageId: ids.package,
                definitionVersion: "1.0.0",
                definitionName: "archive",
                spec: { replicated: [] },
                createdAt: now,
                updatedAt: now,
            }),
    );
    await fixture.simulation.idle();
    const [provisioned] = await database
        .select()
        .from(object.database.table)
        .where(eq(object.database.table.id, named.kept));
    expect(provisioned?.conditions["Ready"]?.reason).toBe("Applied");
});

test("retry a provisioning its provider failed", async () => {
    // fail the database provider's first provisioning
    let isFailing = true;
    const fixture = await TestSpace.open({
        resources: (directory, environment) => {
            const [base] = providers(directory, environment);
            const flaky: typeof base = {
                ...base,
                provision: {
                    isPure: false,
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

            return { providers: [flaky], connectors: [] };
        },
    });
    const { database } = fixture;
    const now = fixture.machine.clock.now();
    await database.transaction(
        async (transaction) =>
            await transaction.insert(object.database.table).values({
                id: named.removed,
                scope: ids.space,
                ...ObjectReference.columns(
                    "owner",
                    principal.space.reference(Scope.universe.id, ids.space),
                ),
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
    await fixture.serve();
    const read = async () =>
        (
            await database
                .select()
                .from(object.database.table)
                .where(eq(object.database.table.id, named.removed))
        )[0];

    // retry the provider's failure as unavailable, and provision on the next attempt
    await fixture.simulation.idle();
    const failure = (await read())?.conditions;
    const isApplied = async () => (await read())?.conditions["Ready"]?.reason === "Applied";
    await fixture.simulation.waitUntil(isApplied);
    fixture.expectRefusals([]);
    expect(failure).toEqual({});
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
