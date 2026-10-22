import { Identifier, schema } from "@destack/schema";
import { Snapshot } from "@destack/db";
import { setting, SettingValue } from "@destack/setting/object";
import { expect, test } from "@destack/test";
import { buildRetention } from "../setting/index.ts";
import { Build, build, installation, installationRevision } from "./index.ts";
import { ids, StackFixture } from "../test/index.ts";

/** The builds the stress test records in one space, enough that a read per build shows. */
const STRESS_BUILDS = 2000;

/** The directories the stress test builds from, each a source of its own. */
const STRESS_SOURCES = 4;

test("retain each source's latest builds and every build a revision applies", async () => {
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const now = Date.now();

    // record four builds of the root, one of another directory and a failed one
    const record = async (directory: string, manifest: string | null, finishedAt: number) => {
        const id = Identifier.create("build");
        await database.insert(build.table).values({
            id,
            scope: ids.space,
            source: { kind: "checkout", machine: ids.machine, checkout: ids.checkout, directory },
            outputs: ["bun"],
            state: manifest === null ? "failed" : "succeeded",
            manifest,
            failure:
                manifest === null ? { code: "INTERNAL_SERVER_ERROR", message: "failed" } : null,
            startedAt: finishedAt,
            finishedAt,
            createdAt: finishedAt,
            updatedAt: finishedAt,
        });

        return id;
    };
    const oldest = await record(".", "a".repeat(64), now - 4000);
    const dropped = await record(".", "b".repeat(64), now - 3000);
    await record(".", "c".repeat(64), now - 2000);
    await record(".", null, now - 1500);
    await record(".", "d".repeat(64), now - 1000);
    await record("tools", "e".repeat(64), now - 5000);

    // keep two builds of each source in the space and apply the oldest build in a revision
    await database.insert(setting.table).values({
        id: Identifier.create("setting"),
        scope: ids.space,
        packageId: buildRetention.package.id,
        name: buildRetention.name,
        mode: "set",
        value: 2,
        release: buildRetention.package.version,
        createdAt: now,
        updatedAt: now,
    });
    await database.insert(installation.table).values({
        id: ids.notes,
        scope: ids.space,
        packageId: ids.package,
        role: "application",
        selection: {
            kind: "checkout",
            machine: ids.machine,
            checkout: ids.checkout,
            directory: ".",
        },
        name: "notes",
        createdAt: now,
        updatedAt: now,
    });
    await database.insert(installationRevision.table).values({
        id: schema
            .identifier("installation-revision")
            .parse("installation-revision-01996ab0-0000-7000-8000-0000000000f1"),
        scope: ids.space,
        installationId: ids.notes,
        buildId: oldest,
        dependencies: [],
        digest: "f".repeat(64),
        createdAt: now,
        updatedAt: now,
    });

    const retention = (space: string) =>
        SettingValue.resolve(Snapshot.live(database), buildRetention, { scope: space });
    const { retained, expired } = await Build.retention(database, retention);
    expect([[...retained].toSorted(), expired]).toEqual([
        ["a".repeat(64), "c".repeat(64), "d".repeat(64), "e".repeat(64)],
        [{ id: dropped, scope: ids.space }],
    ]);
});

test("decide the retention of many builds in two statements and one retention read per space", async () => {
    // record many builds of a few directories in one space
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const now = Date.now();
    await database.insert(build.table).values(
        Array.from({ length: STRESS_BUILDS }, (_, index) => ({
            id: Identifier.create("build"),
            scope: ids.space,
            source: {
                kind: "checkout" as const,
                machine: ids.machine,
                checkout: ids.checkout,
                directory: `directory-${String(index % STRESS_SOURCES)}`,
            },
            outputs: ["bun" as const],
            state: "succeeded" as const,
            manifest: index.toString(16).padStart(64, "0"),
            failure: null,
            startedAt: now - index,
            finishedAt: now - index,
            createdAt: now - index,
            updatedAt: now - index,
        })),
    );

    // keep two builds of each source, counting the statements and retention reads
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
        retained: 2 * STRESS_SOURCES,
        expired: STRESS_BUILDS - 2 * STRESS_SOURCES,
        statements: 2,
        reads: 1,
    });
});
