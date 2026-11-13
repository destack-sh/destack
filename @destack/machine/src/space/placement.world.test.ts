import { UNIVERSE_NODE } from "@destack/control/test";
import accountPackage from "@destack/account/package";
import { identityKey } from "@destack/account/identity";
import * as accountObject from "@destack/account/object";
import { BuildStore } from "@destack/build/store";
import { eq } from "@destack/db";
import { RequestId } from "@destack/sync";
import { expect, test } from "@destack/test";
import * as object from "@destack/stack/object";
import { fixtureBuild, ids, reconcile, TestSpace } from "../test/index.ts";

test("delete a space at its home once its installations are gone, clearing its builds", async () => {
    // serve the space its universe placed here, with an application installed and a build of its own
    const fixture = await TestSpace.open();
    const served = await fixture.serve();
    const reader = await fixtureBuild(ids.package, "2026.9.0", fixture.environment.crypto);
    await fixture.install(served, reader);
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

    // remove the installations, the space's replica here and its keys, and its placement and row at the universe, the account's home left
    const installations = await fixture.database
        .select({ id: object.rollout.table.installationId })
        .from(object.rollout.table);
    const keys = await fixture.database
        .select({ scope: identityKey.scope })
        .from(identityKey)
        .where(eq(identityKey.scope, ids.space));
    const replicas = await fixture.database
        .select({ id: accountObject.space.table.id })
        .from(accountObject.space.table);
    const placed = await accountObject.Placement.find(fixture.machine.directory.snapshot, [
        ids.space,
    ]);
    const listed = await accounts.space.list({ accountId: ids.account, limit: 10 });
    const isCleared = !(await builds.contains(manifest));
    expect({
        removed: [installations, keys, replicas, placed, listed.items.map((item) => item.id)],
        builds: [isStored, isCleared],
    }).toEqual({
        removed: [[], [], [], [], [accountObject.Account.home(ids.account)]],
        builds: [true, true],
    });
});

test("finish removing a deleted space whose row an earlier look already finalized at its home", async () => {
    // serve the space, its replicas following the universe until paused
    const fixture = await TestSpace.open();
    const served = await fixture.serve();

    // request the deletion as the owner stepped up, and pause the universe's pages once it arrives here
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
    const paused = fixture.simulation.network.pause(UNIVERSE_NODE, fixture.machine.id, (sent) =>
        sent.endsWith("/store/stream"),
    );

    // remove the space and its row at the universe, then look again while its replica still has the row
    await fixture.simulation.idle();
    const again = await reconcile(served.store, "space", { id: ids.space }).then(
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
