import { buildManifest } from "@destack/build/table";
import { eq } from "@destack/db";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { PackageStore } from "@destack/build/store";
import { build } from "../../object/index.ts";
import { fixtureBuild, ids, TestSpace } from "../../test/index.ts";

/** The builds of a source a space retains by default, the build retention setting's default. */
const RETAINED = 10;

/** How long a build store leases what it stores for a root to retain: half an hour. */
const LEASE_MILLISECONDS = 30 * 60_000;

/** A week, far past every lease, in milliseconds. */
const WEEK_MILLISECONDS = 7 * 24 * 60 * 60_000;

test("expire a dropped build as builds change, and collect it with its graph once its lease ends, then rest", async () => {
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
        const { build: id } = await fixture.recordBuild(built);
        recorded.push({
            id,
            manifest: await PackageStore.digest(built.manifest, fixture.environment.crypto),
        });

        // read the first build's graph while it is live, attaching its index
        if (minor === 0) {
            await (await fixture.index()).open(id);
        }
        await simulation.clock.sleep(clock.now() + 1 - simulation.clock.now());
        await simulation.idle();
    }

    // read the expiry the last recorded build caused, then wake past the leases and a week on
    const [oldest, ...newer] = recorded;
    const dropped = present(oldest, "the oldest build");
    const [expiry] = await fixture.database
        .select({ expiredAt: build.table.expiredAt })
        .from(build.table)
        .where(eq(build.table.id, dropped.id));
    const isStoredWithinLease = await (await fixture.builds()).space.contains(dropped.manifest);
    const attached = present(
        await (await fixture.builds()).space.referrer(dropped.manifest, "graph"),
        "the dropped build's graph index",
    );
    const isIndexedWithinLease = await (await fixture.builds()).space.has(attached.digest);
    await fixture.simulation.jump(start + LEASE_MILLISECONDS + RETAINED + 1);
    await fixture.simulation.jump(start + WEEK_MILLISECONDS);

    // expire it at once, retain it with its graph through its lease, then collect both
    const { space } = await fixture.builds();
    expect({
        expiredAt: present(expiry?.expiredAt, "the oldest build's expiry") - start,
        isStoredWithinLease,
        isStored: await space.contains(dropped.manifest),
        isIndexedWithinLease,
        isIndexed: await space.has(attached.digest),
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
        retained: newer.map((each) => each.manifest).toSorted(),
    });
});
