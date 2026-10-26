import { accessInvitation, INVITATION_LIFETIME_MILLISECONDS } from "@destack/access";
import accountPackage, { OAUTH_PATH } from "@destack/account/package";
import { PackageMount } from "@destack/object";
import { schema } from "@destack/schema";
import { Snapshot } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { testSimulation } from "@destack/object/test";
import { expect, onTestFinished, test } from "@destack/test";
import { TestUniverse, type TestUniverseOptions, UNIVERSE_NODE } from "../src/test/index.ts";
import { apply, universeDatabase } from "../src/universe/index.ts";

test("start a universe over its definition, answering the account package below its mount and nothing elsewhere", async () => {
    // start the universe with a cloud machine serving its residency
    await using started = await TestUniverse.open();
    const { universe, origin } = started;

    // answer the issuer's discovery below the account package's mount, and refuse other paths
    const issuer = `${PackageMount.url(origin, accountPackage.id)}${OAUTH_PATH}`;
    const discovery = await universe.fetch(
        new Request(`${issuer}/.well-known/openid-configuration`),
    );
    const { issuer: discovered } = schema
        .looseObject({ issuer: schema.string() })
        .parse(await discovery.json());
    const outside = await universe.fetch(new Request(`${origin}/auth/sign-in`));
    await started.settle();
    expect([discovery.status, discovered, outside.status]).toEqual([200, issuer, 404]);
});

test("apply a definition once across starts, again once the operator's invitation lapses, and again once it changes", async () => {
    // store the universe's database across starts in one flowing world
    const simulation = testSimulation();
    simulation.clock.resume();
    onTestFinished(() => simulation[Symbol.asyncDispose]());
    const environment = simulation.environment(UNIVERSE_NODE);
    const storage = await TestDatabase.create(universeDatabase, { environment, isMigrated: true });
    onTestFinished(() => storage.close());
    const counts = async () => ({
        runs: (await Snapshot.live(storage.database).read(apply.run.table).findMany()).length,
        invitations: (await Snapshot.live(storage.database).read(accessInvitation).findMany())
            .length,
    });
    const start = async (options: Pick<TestUniverseOptions, "residencies"> = {}) =>
        (await TestUniverse.open({ simulation, storage, world, ...options }))[
            Symbol.asyncDispose
        ]();

    // start again, then once the operator's invitation lapsed, then over a changed definition
    const first = await TestUniverse.open({ simulation, storage });
    const world = await first.world();
    await first[Symbol.asyncDispose]();
    await start();
    const again = await counts();
    simulation.clock.advance(INVITATION_LIFETIME_MILLISECONDS);
    await start();
    const later = await counts();
    await start({
        residencies: [
            { code: "eu", name: "Europe" },
            { code: "us", name: "United States" },
        ],
    });
    expect({ again, later, changed: await counts() }).toEqual({
        again: { runs: 1, invitations: 1 },
        later: { runs: 2, invitations: 2 },
        changed: { runs: 3, invitations: 2 },
    });
});
