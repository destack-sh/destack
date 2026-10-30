import { expect, test } from "@destack/test";
import { fixtureBuild } from "./build.ts";
import { ids, TestSpace } from "./fixture.ts";

/** The moment the simulated world starts at, the stack's template's: 2026-10-01T00:00:00Z. */
const START = Date.UTC(2026, 9, 1);

test("open a space in a simulated world from its template without moving its clock or losing a commit race", async () => {
    // open the fixture's space in a stopped fork of its template's world
    const fixture = await TestSpace.open();
    const { simulation } = fixture;

    // leave the world at its start, its controllers acting once the space's replicas arrived
    const { database } = fixture;
    expect({
        now: simulation.clock.now(),
        lostRaces: [database, fixture.universe.storage.database].map(
            (each) => each.state.lostRaces,
        ),
    }).toEqual({ now: START, lostRaces: [0, 0] });
});

test("serve a space and install a build in a simulated world without moving its clock", async () => {
    // open, serve and install on a stopped fork, settling the work at its start
    const fixture = await TestSpace.open();
    const { simulation } = fixture;
    const build = await fixtureBuild(
        ids.package,
        "2026.9.0",
        simulation.environment("test").crypto,
    );
    await (async () => {
        const served = await fixture.serve();
        await fixture.install(served, build);
    })();

    // leave the world at its start, as serving and installing wait only on tracked work
    expect(simulation.clock.now()).toBe(START);
});
