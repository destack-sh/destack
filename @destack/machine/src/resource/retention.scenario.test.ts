import { fixtureGraph } from "@destack/build/test";
import { buildManifest } from "@destack/build/table";
import { eq } from "@destack/db";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { BuildStore } from "@destack/build/store";
import { build } from "@destack/stack/object";
import { fixtureBuild, ids, imageOf, TestSpace } from "../test/index.ts";

/** The builds of a source a space retains by default, the build retention setting's default. */
const RETAINED = 10;

/** How long a build store leases what it stores for a root to retain: half an hour. */
const LEASE_MILLISECONDS = 30 * 60_000;

/** How long an unreferenced blob stays before deletion: a blob store's hour of grace. */
const GRACE_MILLISECONDS = 60 * 60_000;

/** A week, far past every lease, in milliseconds. */
const WEEK_MILLISECONDS = 7 * 24 * 60 * 60_000;

test("expire a build past retention and collect it with its graph after its lease", async () => {
    // record one build more of the fixture's checkout than the space retains by default, a millisecond apart
    const fixture = await TestSpace.open();
    const { simulation } = fixture;
    const { clock } = simulation;
    await fixture.serve();
    const start = clock.now();
    const recorded = [];
    for (let minor = 0; minor <= RETAINED; minor += 1) {
        const built = await fixtureBuild(
            ids.package,
            `2026.${minor + 1}.0`,
            fixture.environment.crypto,
        );
        const { build: id } = await fixture.recordBuild(built, await fixtureGraph(built));
        const manifest = await BuildStore.digest(built.manifest, fixture.environment.crypto);
        const image = await imageOf(built, fixture.environment.crypto);
        recorded.push({ id, manifest, image });

        // read the first build's graph while it is live, attaching its index
        if (minor === 0) {
            await (await fixture.index()).open(id);
        }
        await simulation.run(clock.now() + 1);
    }

    // read the expiry the last recorded build caused, then wake past the leases and a week on
    const [oldest, ...newer] = recorded;
    const dropped = present(oldest, "the oldest build");
    const [expiry] = await fixture.database
        .select({ expiredAt: build.table.expiredAt })
        .from(build.table)
        .where(eq(build.table.id, dropped.id));
    const isStoredWithinLease = await (await fixture.builds()).contains(dropped.manifest);
    const key = await (await fixture.index()).cacheKey(dropped.manifest);
    const cached = await (await fixture.builds()).cached(key);
    const attached = present(
        cached?.kind === "file" ? cached.file : undefined,
        "the dropped build's graph database",
    );
    const [isIndexedWithinLease] = await (await fixture.builds()).has([attached.digest]);
    await fixture.simulation.jump(start + LEASE_MILLISECONDS + RETAINED + 1);
    await fixture.simulation.jump(start + WEEK_MILLISECONDS);
    await fixture.simulation.jump(fixture.simulation.clock.now() + GRACE_MILLISECONDS);

    // wait a grace for the builds the dropped image alone kept, as a writer may still refer to them
    await fixture.simulation.jump(fixture.simulation.clock.now() + GRACE_MILLISECONDS);

    // expire it at once, retain it with its graph through its lease, then collect both
    const space = await fixture.builds();
    expect({
        expiredAt: present(expiry?.expiredAt, "the oldest build's expiry") - start,
        isStoredWithinLease,
        isStored: await space.contains(dropped.manifest),
        isIndexedWithinLease,
        isIndexed: (await space.has([attached.digest]))[0],
        retained: (
            await space.database.select({ digest: buildManifest.digest }).from(buildManifest)
        )
            .map((row) => row.digest)
            .toSorted(),
    }).toEqual({
        expiredAt: RETAINED,
        isStoredWithinLease: true,
        isStored: false,
        isIndexedWithinLease: true,
        isIndexed: false,
        retained: newer.flatMap((each) => [each.manifest, each.image]).toSorted(),
    });
});
