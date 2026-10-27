import { eq } from "@destack/db";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { PackageStore } from "@destack/build/store";
import { build } from "../../object/index.ts";
import { fixtureBuild, ids, TestSpace } from "../../test/index.ts";

/** The builds of a source a space retains by default, the build retention setting's default. */
const RETAINED = 10;

/** The moment the simulated world starts at. */
const START = Date.UTC(2026, 9, 1);

/** How often a machine sweeps its stores: once per fifteen-minute write lease. */
const INTERVAL_MILLISECONDS = 900_000;

/** How long stored files stay safe from a sweep: two write leases. */
const GRACE_MILLISECONDS = 2 * INTERVAL_MILLISECONDS;

/** How long a machine stays down before it restarts, well within an interval: a minute. */
const RESTART_MILLISECONDS = 60_000;

test("expire dropped builds and sweep their indexes once due past the stored files' grace", async () => {
    // record one build more of the fixture's checkout than the space retains by default, a millisecond apart, in a simulated world
    const fixture = await TestSpace.open();
    const { simulation } = fixture;
    const { clock } = simulation;
    const sweeps: number[] = [];
    const graphs = {
        sweep: async () => {
            sweeps.push(clock.now());
        },
    };
    const served = await simulation.settle(fixture.serve({ graphs }));
    const recorded = [];
    for (let minor = 0; minor <= RETAINED; minor += 1) {
        const built = await fixtureBuild(
            ids.package,
            `2026.${minor + 1}.0`,
            fixture.environment.crypto,
        );
        const { build: id } = await simulation.settle(fixture.recordBuild(built));
        recorded.push({
            id,
            manifest: await PackageStore.digest(built.manifest, fixture.environment.crypto),
        });
        await simulation.run({ to: clock.now() + 1 });
    }

    // wake at each interval the retention sweeps at, as a machine waking from sleep
    for (
        let at = START + INTERVAL_MILLISECONDS;
        at <= START + GRACE_MILLISECONDS;
        at += INTERVAL_MILLISECONDS
    ) {
        await fixture.jump(served, at);
    }

    // wake half an interval past the stored files' grace
    await fixture.jump(served, START + GRACE_MILLISECONDS + INTERVAL_MILLISECONDS / 2);

    // retain the newer builds and the stack's, expire the oldest in the first interval's sweep, and sweep the indexes at each sweep
    const [oldest, ...newer] = recorded;
    const dropped = present(oldest, "the oldest build");
    const expired = await fixture.database
        .select({ expiredAt: build.table.expiredAt })
        .from(build.table)
        .where(eq(build.table.id, dropped.id));
    const retained = await (await fixture.builds()).packages.retained();
    const [expiry] = expired;
    const expiredAt = present(expiry?.expiredAt, "the oldest build's expiry");
    expect({
        expired: expired.length,
        interval: Math.floor((expiredAt - START) / INTERVAL_MILLISECONDS),
        sweeps: sweeps.map((at) => at - expiredAt),
        retained: [...retained].toSorted(),
    }).toEqual({
        expired: 1,
        interval: 1,
        sweeps: [0, INTERVAL_MILLISECONDS],
        retained: [
            ...newer.map((each) => each.manifest),
            fixture.releases.source.manifest,
        ].toSorted(),
    });
});

test("retain a space's builds once per interval across restarts, leaving a restart within it idle", async () => {
    // serve the fixture's space, then restart it within the interval
    const fixture = await TestSpace.open();
    const { simulation } = fixture;
    const { packages } = await fixture.builds();
    const served = await simulation.settle(fixture.serve());
    const before = await packages.rootedAt(fixture.id);
    await served[Symbol.asyncDispose]();
    await simulation.run({ to: simulation.clock.now() + RESTART_MILLISECONDS });
    const restarted = await simulation.settle(fixture.serve());
    const within = await packages.rootedAt(fixture.id);

    // wake once the interval passed, rooting the builds again
    await fixture.jump(restarted, START + INTERVAL_MILLISECONDS);
    expect({ before, within, later: await packages.rootedAt(fixture.id) }).toEqual({
        before: START,
        within: START,
        later: START + INTERVAL_MILLISECONDS,
    });
});
