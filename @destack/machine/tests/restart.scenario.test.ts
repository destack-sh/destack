import { installation } from "@destack/stack/object";
import stackPackage from "@destack/stack/package";
import { createClient } from "@destack/object/client";
import { present } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { expect, test } from "@destack/test";
import { OWNER_EMAIL, TestComputer } from "../src/computer/test/index.ts";

/** The tag a change after the restart sets on the home's start installation. */
const RESUMED = { resumed: "yes" };

test("deliver a home change to a laptop replica soon after the home restarts", async () => {
    // run the owner's home in the cloud, and their laptop through the relay, signed in
    const { home, login } = await TestComputer.home();
    const { world } = home;
    const relay = await TestComputer.relay(world);
    const laptop = await TestComputer.open({ world, relay: relay.url });
    const signedIn = await laptop.signIn(OWNER_EMAIL);
    await laptop.enroll(signedIn, world.owner.account);

    // replicate the home's installations on the laptop as the owner
    const session = await laptop.device.launcher(signedIn.id);
    const space = `home--${world.owner.handle}`;
    await using stack = new AsyncDisposableStack();
    const replica = await laptop.device.open(session, { space }, { installation }, stack);
    const watched = replica.query.installation.findMany({}).watch(new AbortController().signal);
    const first = await world.simulation.wait(watched.next());
    const before = present(startOf(first), "the home's start installation");

    // restart the home's computer, then tag the start page as the owner once it serves the home again
    const restarted = await home.restart();
    await restarted.ready(world.owner.home);
    const ready = world.simulation.clock.now();
    const owner = await restarted.device.launcher(login);
    const objects = createClient(
        stackPackage,
        owner.endpoint(stackPackage.id, await restarted.device.locate(world.owner.home)),
        restarted.environment,
    );
    await objects.installation.update({
        scope: world.owner.home,
        id: before.id,
        requestId: RequestId.create(restarted.environment),
        tags: RESUMED,
    });

    // wait until the laptop's replica has the tag
    let elapsed: number | undefined;
    while (elapsed === undefined) {
        const next = await world.simulation.wait(watched.next());
        const row = startOf(next);
        if (row !== undefined && JSON.stringify(row.tags) === JSON.stringify(RESUMED)) {
            elapsed = world.simulation.clock.now() - ready;
        }
    }

    expect(elapsed).toBeLessThanOrEqual(5000);
});

/** Find the start installation among a watch's rows, absent once the watch ends. */
function startOf<Row extends { readonly name: string }>(
    result: IteratorResult<readonly Row[], void>,
): Row | undefined {
    return result.done === true ? undefined : result.value.find((row) => row.name === "start");
}
