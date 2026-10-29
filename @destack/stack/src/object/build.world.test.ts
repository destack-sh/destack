import { Failure } from "@destack/error";
import { Identifier, schema, Digest } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Build, build, installationRevision } from "./index.ts";
import { ids, insertRollout, TestSpace } from "../test/index.ts";

/** The builds the stress test records in one space, enough that a read per build shows. */
const STRESS_BUILDS = 2000;

/** The directories the stress test builds from, each a source of its own. */
const STRESS_SOURCES = 4;

test("retain each source's latest builds and every build a revision applies", async () => {
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const now = fixture.machine.clock.now();

    // record four builds of the root, one of another directory and a failed one, as retention reads them
    const record = async (directory: string, manifest: Digest | null, finishedAt: number) => {
        const id = Identifier.create("build", fixture.environment);
        await database.transaction(
            async (transaction) =>
                await transaction.insert(build.table).values({
                    id,
                    scope: ids.space,
                    source: {
                        kind: "checkout",
                        machine: ids.machine,
                        checkout: ids.checkout,
                        directory,
                    },
                    outputs: ["bun"],
                    state: manifest === null ? "failed" : "succeeded",
                    manifest,
                    failure:
                        manifest === null ? Failure.http("INTERNAL_SERVER_ERROR", "failed") : null,
                    startedAt: finishedAt,
                    finishedAt,
                    createdAt: finishedAt,
                    updatedAt: finishedAt,
                }),
        );

        return id;
    };
    const oldest = await record(".", Digest.parse("a".repeat(64)), now - 4000);
    const dropped = await record(".", Digest.parse("b".repeat(64)), now - 3000);
    await record(".", Digest.parse("c".repeat(64)), now - 2000);
    await record(".", null, now - 1500);
    await record(".", Digest.parse("d".repeat(64)), now - 1000);
    await record("tools", Digest.parse("e".repeat(64)), now - 5000);

    // apply the oldest build in a revision, as the rows retention reads record it
    await insertRollout(
        database,
        {
            id: ids.notes,
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
    await database.transaction(
        async (transaction) =>
            await transaction.insert(installationRevision.table).values({
                id: schema
                    .identifier("installation-revision")
                    .parse("installation-revision-01996ab0-0000-7000-8000-0000000000f1"),
                scope: ids.space,
                installationId: ids.notes,
                buildId: oldest,
                dependencies: [],
                states: {},
                digest: Digest.parse("f".repeat(64)),
                createdAt: now,
                updatedAt: now,
            }),
    );

    // retain two builds of each source
    const { retained, expired } = await Build.retention(database, async () => 2);

    // retain the stack's applied build too, which its own revision applies
    expect([[...retained].toSorted(), expired]).toEqual([
        [
            Digest.parse("a".repeat(64)),
            Digest.parse("c".repeat(64)),
            Digest.parse("d".repeat(64)),
            fixture.releases.source.manifest,
            Digest.parse("e".repeat(64)),
        ],
        [{ id: dropped, scope: ids.space }],
    ]);
});

test("decide the retention of many builds in two statements and one retention read per space", async () => {
    // record many builds of a few directories in one space
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const now = fixture.machine.clock.now();
    await database.transaction(
        async (transaction) =>
            await transaction.insert(build.table).values(
                Array.from({ length: STRESS_BUILDS }, (_, index) => ({
                    id: Identifier.create("build", fixture.environment),
                    scope: ids.space,
                    source: {
                        kind: "checkout" as const,
                        machine: ids.machine,
                        checkout: ids.checkout,
                        directory: `directory-${String(index % STRESS_SOURCES)}`,
                    },
                    outputs: ["bun" as const],
                    state: "succeeded" as const,
                    manifest: Digest.parse(index.toString(16).padStart(64, "0")),
                    failure: null,
                    startedAt: now - index,
                    finishedAt: now - index,
                    createdAt: now - index,
                    updatedAt: now - index,
                })),
            ),
    );

    // retain two builds of each source, counting the statements and retention reads
    const reader = database.metered();
    let reads = 0;
    const retention = async () => {
        reads += 1;

        return 2;
    };
    const { retained, expired } = await Build.retention(reader, retention);

    // retain the latest two of each source and expire the rest
    expect({
        retained: retained.size,
        expired: expired.length,
        statements: reader.driver.operations,
        reads,
    }).toEqual({
        // the latest two of each source, and the build the stack's revision applies
        retained: 2 * STRESS_SOURCES + 1,
        expired: STRESS_BUILDS - 2 * STRESS_SOURCES,
        statements: 2,
        reads: 1,
    });
});
