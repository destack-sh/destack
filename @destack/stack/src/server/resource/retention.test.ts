import { eq } from "@destack/db";
import { Identifier } from "@destack/schema";
import { setting } from "@destack/setting/object";
import { expect, test } from "@destack/test";
import { build } from "../../object/index.ts";
import { buildRetention } from "../../setting/index.ts";
import { fixtureBuild, ids, StackFixture } from "../../test/index.ts";
import { PackageStore } from "@destack/build/store";
import { serveSpace } from "../server.ts";
import { RetentionController } from "./retention.ts";
import { VirtualClock } from "@destack/environment/simulation";

test("expire dropped builds and sweep their indexes once due, looking again after the interval", async () => {
    // keep one build of each source, with two succeeded builds of the fixture's checkout
    const fixture = await StackFixture.open();
    const clock = new VirtualClock(Date.UTC(2026, 9, 5));
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
    let sweeps = 0;
    const controller = new RetentionController(
        objects,
        await fixture.builds(),
        {
            sweep: async () => {
                sweeps += 1;
            },
        },
        fixture.store(),
    );

    // sweep and look again after one write lease's fifteen minutes
    expect(await controller.reconcile()).toBe(900_000);

    // keep the newer build, expire the older one at the sweep, and sweep the indexes once
    const [expired] = await objects.database
        .select({ expiredAt: build.table.expiredAt })
        .from(build.table)
        .where(eq(build.table.id, older.build));
    const retained = await (await fixture.builds()).packages.retained();
    expect([sweeps, [...retained], expired]).toEqual([
        1,
        [newer],
        { expiredAt: Date.UTC(2026, 9, 5) },
    ]);
});
