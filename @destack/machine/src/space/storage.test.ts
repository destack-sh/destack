import { principal } from "@destack/access";
import { defineDatabase, sql, TABLE } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Digest, Identifier } from "@destack/schema";
import { MAIN_DATABASE } from "@destack/stack/declare";
import { deployment, installationRevision } from "@destack/stack/object";
import { Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/object/test";
import {
    ids,
    insertInstallation,
    insertRelease,
    NOTES_RELEASE,
    openSpaceDatabase,
    spaceDatabase,
    workload,
} from "../test/index.ts";

/** The person installing the notes package. */
const INSTALLER = principal.user.reference(
    Scope.universe.id,
    "user-01996ab0-0000-7000-8000-0000000000b1",
);

test("record what each workload deployed before edition 2 answers, the hook workload by its old name", async () => {
    // open the space's main database with its deployments at their first edition
    const first = TestDatabase.edition(deployment.table, 1);
    const swap = (table: (typeof spaceDatabase.tables)[number]) =>
        table === deployment.table ? first : table;
    const replicated = spaceDatabase.tables.filter((table) =>
        spaceDatabase.replicated.has(table[TABLE].sqlName),
    );
    const earlier = defineDatabase({
        name: MAIN_DATABASE,
        tables: spaceDatabase.tables.filter((table) => !replicated.includes(table)).map(swap),
        replicated,
    });
    const { database, now } = await openSpaceDatabase(earlier);
    const environment = testSimulation().environment("test");

    // install notes at a revision of a pulled release
    await insertInstallation(
        database,
        {
            id: ids.notes,
            scope: ids.space,
            packageId: ids.package,
            name: "notes",
            version: NOTES_RELEASE,
            installer: INSTALLER,
        },
        now,
    );
    const pulled = await insertRelease(
        database,
        ids.space,
        { package: ids.package, version: "2026.9.0", image: Digest.parse("e".repeat(64)) },
        [],
        environment,
    );
    const revisionId = Identifier.create("installation-revision", environment);
    await database.transaction(
        async (transaction) =>
            await transaction.insert(installationRevision.table).values({
                id: revisionId,
                scope: ids.space,
                installationId: ids.notes,
                buildId: pulled.id,
                dependencies: [],
                states: {},
                release: "2026.9.0",
                declarations: Digest.parse("e".repeat(64)),
                generation: 1,
                bindings: [],
                digest: Digest.parse("e".repeat(64)),
                createdAt: now,
                updatedAt: now,
            }),
    );

    // deploy its server and its hook workload as the first edition stored them, without what they serve
    for (const name of ["main", "hooks"]) {
        await database.transaction(
            async (transaction) =>
                await transaction.insert(first).values({
                    id: Identifier.create("deployment", environment),
                    scope: ids.space,
                    installationId: ids.notes,
                    packageId: ids.package,
                    revisionId,
                    output: "server",
                    workload: name,
                    runtime: "bun",
                    release: "2026.9.0",
                    description: workload,
                    activatedAt: now,
                    createdAt: now,
                    updatedAt: now,
                }),
        );
    }
    const unserved =
        TestDatabase.target() === "sqlite"
            ? sql`json_remove(description, '$.kind')`
            : sql`description - 'kind'`;
    await database.transaction(
        async (transaction) =>
            await transaction.run(
                sql`UPDATE ${sql.relation(first[TABLE].sqlName)} SET description = ${unserved}`,
            ),
    );
    const { kind: _kind, ...stored } = workload;

    // migrate to the second edition
    await database.migrate(spaceDatabase.tables);
    const rows = await database
        .select({ workload: deployment.table.workload, description: deployment.table.description })
        .from(deployment.table)
        .orderBy(deployment.table.workload);

    // record a call workload as serving calls, and the hook workload by its old name as serving hooks
    expect(rows).toEqual([
        { workload: "hooks", description: { ...stored, kind: "hook" } },
        { workload: "main", description: { ...stored, kind: "call" } },
    ]);
});
