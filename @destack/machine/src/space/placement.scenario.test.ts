import { UNIVERSE_NODE } from "@destack/control/test";
import accountPackage from "@destack/account/package";
import * as accountObject from "@destack/account/object";
import { BuildStore } from "@destack/build/store";
import { eq } from "@destack/db";
import { CONNECTION_PATH } from "@destack/object";
import { RequestId } from "@destack/sync";
import { expect, test } from "@destack/test";
import { fixtureBuild, ids, reconcile, TestSpace } from "../test/index.ts";

test("delete a space at its home once its installations are gone, clearing its builds", async () => {
    // serve the space its universe placed here, with an application installed and a build of its own
    const fixture = await TestSpace.open();
    await fixture.serve();
    const reader = await fixtureBuild(ids.package, "2026.9.0", fixture.environment.crypto);
    await fixture.install(reader);
    await fixture.recordBuild(reader);
    const manifest = await BuildStore.digest(reader.manifest, fixture.environment.crypto);

    // request the deletion at the universe as the owner stepped up, and settle the space removing itself
    const builds = await fixture.builds();
    const isStored = await builds.contains(manifest);
    const owner = await fixture.universe.elevate(fixture.owner);
    const accounts = owner.client(accountPackage);
    await accounts.space.delete({
        accountId: ids.account,
        id: ids.space,
        requestId: RequestId.create(fixture.universe.environment),
    });
    await fixture.simulation.idle();

    // remove the space's volume here, and its placement and row at the universe, the account's home left
    const volumes = await fixture.machine.volumes.list();
    const placed = await accountObject.Placement.find(fixture.machine.directory.snapshot, [
        ids.space,
    ]);
    const listed = await accounts.space.list({ accountId: ids.account, limit: 10 });
    const isCleared = !(await fixture.machine.builds.roots()).has(`space:${ids.space}`);
    expect({
        removed: [volumes.includes(ids.space), placed, listed.items.map((item) => item.id)],
        builds: [isStored, isCleared],
    }).toEqual({
        removed: [false, [], [accountObject.Account.home(ids.account)]],
        builds: [true, true],
    });
});

test("finish removing a deleted space whose universe row is already gone", async () => {
    // serve the space, its replicas following the universe until paused
    const fixture = await TestSpace.open();
    await fixture.serve();

    // request the deletion as the owner stepped up, and pause the universe's pages on its connection once it arrives here
    const owner = await fixture.universe.elevate(fixture.owner);
    await owner.client(accountPackage).space.delete({
        accountId: ids.account,
        id: ids.space,
        requestId: RequestId.create(fixture.universe.environment),
    });
    const replicated = async () => {
        const [row] = await fixture.database
            .select({ deletedAt: accountObject.space.table.deletedAt })
            .from(accountObject.space.table)
            .where(eq(accountObject.space.table.id, ids.space));

        return row;
    };
    await fixture.database.log.waitUntil(
        async () => (await replicated())?.deletedAt !== null,
        new AbortController().signal,
    );
    const paused = fixture.simulation.network.pause(
        UNIVERSE_NODE,
        fixture.machine.id,
        (sent) => sent.includes(CONNECTION_PATH) && sent.includes('"changes":'),
    );

    // remove the space and its row at the universe, then look again while its replica still has the row
    await fixture.simulation.idle();
    const again = await reconcile(await fixture.store(), "destack.space.limit", {
        id: ids.space,
    }).then(
        () => "removed",
        (error: unknown) => (error instanceof Error ? error.message : "unknown"),
    );
    const homed = await fixture.universe.storage.database
        .select({ id: accountObject.space.table.id })
        .from(accountObject.space.table)
        .where(eq(accountObject.space.table.id, ids.space));
    const replica = await replicated();
    paused.resume();

    expect({ again, homed, isReplicated: replica !== undefined }).toEqual({
        again: "removed",
        homed: [],
        isReplicated: true,
    });
});
