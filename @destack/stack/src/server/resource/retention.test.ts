import { eq } from "@destack/db";
import { Identifier } from "@destack/schema";
import { setting } from "@destack/setting/object";
import { expect, test, TestClock } from "@destack/test";
import { build } from "../../object/index.ts";
import { buildRetention } from "../../setting/index.ts";
import { fixtureBuild, ids, StackFixture } from "../../test/index.ts";
import { PackageStore } from "@destack/build/store";
import { serveSpace } from "../server.ts";
import { RetentionController } from "./retention.ts";

test("expire dropped builds and sweep the pull cache, looking again after the interval", async () => {
    // keep one build of each source, with two succeeded builds of the fixture's checkout
    const fixture = await StackFixture.open();
    const clock = new TestClock(Date.UTC(2026, 9, 5));
    const objects = serveSpace(await fixture.options({ clock })).store;
    await objects.database.insert(setting.table).values({
        id: Identifier.create("setting"),
        scope: ids.space,
        packageId: buildRetention.package.id,
        name: buildRetention.name,
        mode: "set",
        value: 1,
        release: buildRetention.package.version,
        createdAt: clock.now(),
        updatedAt: clock.now(),
    });
    const older = await fixture.recordBuild(await fixtureBuild(ids.package, "2026.9.0"));
    const newerBuild = await fixtureBuild(ids.package, "2026.10.0");
    await fixture.recordBuild(newerBuild);
    const newer = await PackageStore.digest(newerBuild.manifest);
    const sweeps: [string[], number][] = [];
    const controller = new RetentionController(
        objects,
        await fixture.builds(),
        async (retained, before) => {
            sweeps.push([[...retained].toSorted(), before.getTime()]);
        },
        fixture.store(),
    );

    // sweep and look again after ten minutes
    expect(await controller.reconcile()).toBe(600_000);

    // keep the newer build and expire the older one at the sweep
    const [expired] = await objects.database
        .select({ expiredAt: build.table.expiredAt })
        .from(build.table)
        .where(eq(build.table.id, older.build));
    expect([sweeps, expired]).toEqual([
        [[[newer], Date.UTC(2026, 9, 5) - 3_600_000]],
        { expiredAt: Date.UTC(2026, 9, 5) },
    ]);
});
