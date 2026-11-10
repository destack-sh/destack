import { UniverseOrigin } from "@destack/account/address";
import { accessInvitation, INVITATION_LIFETIME_MILLISECONDS } from "@destack/access";
import { schema } from "@destack/schema";
import { Snapshot } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { testSimulation } from "@destack/object/test";
import { expect, onTestFinished, test } from "@destack/test";
import { TestControl, type TestControlOptions, UNIVERSE_NODE } from "../src/test/index.ts";
import { apply, Control } from "../src/control/index.ts";

test("start a universe over its definition, serving its issuer at the issuer's path and nothing at its root or elsewhere", async () => {
    // start the universe with a cloud machine serving its residency
    await using started = await TestControl.open();
    const { control, origin } = started;

    // read the issuer's discovery at its path
    const issuer = UniverseOrigin.issuer(origin);
    const discovery = await control.fetch(
        new Request(`${issuer}/.well-known/openid-configuration`),
    );
    const { issuer: discovered } = schema
        .looseObject({ issuer: schema.string() })
        .parse(await discovery.json());

    // refuse the root and any other path, which the routes of the universe's hostname serve
    const outside = await control.fetch(new Request(`${origin}/sign-in`));
    const root = await control.fetch(new Request(`${origin}/`));
    await started.simulation.idle();
    expect([discovery.status, discovered, outside.status, root.status]).toEqual([
        200,
        issuer,
        404,
        404,
    ]);
});

test("apply a definition once across starts, again once the operator's invitation lapses, and again once it changes", async () => {
    // store the universe's database across starts in one world
    const simulation = testSimulation();
    onTestFinished(() => simulation[Symbol.asyncDispose]());
    const environment = simulation.environment(UNIVERSE_NODE);
    const storage = await TestDatabase.create(Control.database, { environment, isMigrated: true });
    onTestFinished(() => storage.close());
    const counts = async () => ({
        runs: (await Snapshot.live(storage.database).read(apply.run.table).findMany()).length,
        invitations: (await Snapshot.live(storage.database).read(accessInvitation).findMany())
            .length,
    });
    const start = async (options: Pick<TestControlOptions, "residencies"> = {}) =>
        (await TestControl.open({ simulation, storage, world, ...options }))[Symbol.asyncDispose]();

    // start again, then once the operator's invitation lapsed, then over a changed definition
    const first = await TestControl.open({ simulation, storage });
    const world = await first.world();
    await first[Symbol.asyncDispose]();
    await start();
    const again = await counts();
    await simulation.jump(simulation.clock.now() + INVITATION_LIFETIME_MILLISECONDS);
    await start();
    const later = await counts();
    await start({
        residencies: [
            {
                code: "eu",
                name: "Europe",
                relay: "https://relay-eu.machine.test",
                countries: ["DE"],
            },
            {
                code: "us",
                name: "United States",
                relay: "https://relay-us.machine.test",
                countries: ["US"],
            },
        ],
    });
    expect({ again, later, changed: await counts() }).toEqual({
        again: { runs: 1, invitations: 1 },
        later: { runs: 2, invitations: 2 },
        changed: { runs: 3, invitations: 2 },
    });
});
