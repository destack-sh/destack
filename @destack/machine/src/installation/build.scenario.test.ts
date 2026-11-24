import { Failure } from "@destack/error";
import { Commit, Identifier, schema, Digest, Version } from "@destack/schema";
import { expect, Recording, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { Build, build, installationRevision } from "@destack/stack/object";
import {
    FIXTURE_BRANCH,
    fixtureSource,
    ids,
    insertInstallation,
    TestSpace,
} from "../test/index.ts";

/** The builds the stress test records in one space, enough that a read per build shows. */
const STRESS_BUILDS = 2000;

/** The directories the stress test builds from, each a source of its own. */
const STRESS_SOURCES = 4;

test("retain each source's latest builds and every build a revision applies", async () => {
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const now = fixture.machine.clock.now();

    // record four builds of the root, one of another directory and a failed one, as retention reads them
    const record = async (directory: string, image: Digest | null, finishedAt: number) => {
        // build a commit per finish time
        const id = Identifier.create("build", fixture.environment);
        const commit = Commit.parse(String(finishedAt).padStart(40, "0"));
        const source = { ...fixtureSource(directory), commit };
        await database.transaction(
            async (transaction) =>
                await transaction.insert(build.table).values({
                    id,
                    scope: ids.space,
                    source,
                    key: Build.key({ source, outputs: null, version: null }),
                    commit,
                    outputs: ["bun"],
                    state: image === null ? "failed" : "succeeded",
                    image,
                    failure:
                        image === null ? Failure.http("INTERNAL_SERVER_ERROR", "failed") : null,
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
    await insertInstallation(
        database,
        {
            id: ids.notes,
            scope: ids.space,
            packageId: ids.package,
            name: "notes",
            version: FIXTURE_BRANCH,
            directory: ".",
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
                release: Version.development,
                declarations: Digest.parse("f".repeat(64)),
                generation: 1,
                bindings: [],
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
            fixture.releases.sourceImage,
            Digest.parse("e".repeat(64)),
        ].toSorted(),
        [{ id: dropped, scope: ids.space }],
    ]);
});

test("decide the retention of many builds in at most two statements and one retention read per space", async () => {
    // record many builds of a few directories in one space, its machine stopped so no job reads beside
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await fixture.stop();
    const now = fixture.machine.clock.now();
    await database.transaction(
        async (transaction) =>
            await transaction.insert(build.table).values(
                Array.from({ length: STRESS_BUILDS }, (_, index) => {
                    // build a commit per build, of one of a few directories
                    const commit = Commit.parse(index.toString(16).padStart(40, "0"));
                    const directory = `directory-${String(index % STRESS_SOURCES)}`;
                    const source = { ...fixtureSource(directory), commit };

                    return {
                        id: Identifier.create("build", fixture.environment),
                        scope: ids.space,
                        source,
                        key: Build.key({ source, outputs: null, version: null }),
                        commit,
                        outputs: ["bun" as const],
                        state: "succeeded" as const,
                        image: Digest.parse(index.toString(16).padStart(64, "0")),
                        failure: null,
                        startedAt: now - index,
                        finishedAt: now - index,
                        createdAt: now - index,
                        updatedAt: now - index,
                    };
                }),
            ),
    );

    // retain two builds of each source, counting the statements and retention reads
    const before = await Recording.read(fixture.simulation.telemetry);
    let reads = 0;
    const retention = async () => {
        reads += 1;

        return 2;
    };
    const { retained, expired } = await Build.retention(database, retention);
    const statements = (await Recording.read(fixture.simulation.telemetry))
        .since(before)
        .count(TestDatabase.statements(database));

    // retain the latest two of each source and expire the rest
    expect({
        retained: retained.size,
        expired: expired.length,
        reads,
    }).toEqual({
        // the latest two of each source, and the build the stack's revision applies
        retained: 2 * STRESS_SOURCES + 1,
        expired: STRESS_BUILDS - 2 * STRESS_SOURCES,
        reads: 1,
    });

    // read the builds once and the applied ones at most once, as the connection may still hold them
    expect(statements).toBeLessThanOrEqual(2);
});
