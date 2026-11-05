import type { Select } from "@destack/db";
import { installation } from "@destack/object";
import objectPackage from "@destack/object/package";
import { createClient } from "@destack/object/client";
import { present } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { expect, test } from "@destack/test";
import { OWNER_EMAIL, SCENARIO_MILLISECONDS, TestComputer } from "../src/computer/test/index.ts";

/** The tag a change after the restart sets on the home's start installation. */
const RESUMED = { resumed: "yes" };

test(
    "resume a person's replica of their home on their laptop once the home's computer restarts, receiving a change made after it at once",
    async () => {
        // run the owner's home in the cloud, and their laptop through the relay, signed in
        const { home, login } = await TestComputer.home();
        const { world } = home;
        const relay = await TestComputer.relay(world);
        const laptop = await TestComputer.open({ world, relay: relay.url });
        const signedIn = await laptop.signIn(OWNER_EMAIL);
        await laptop.enroll(signedIn, world.owner.account);

        // replicate the home's installations on the laptop as the owner
        const session = await laptop.device.session(signedIn.id);
        const space = `home--${world.owner.handle}`;
        await using stack = new AsyncDisposableStack();
        const replica = await session.open({ space }, { installation }, stack);
        const watched = replica.query.installation.findMany({}).watch(new AbortController().signal);
        const first = await world.simulation.wait(watched.next());
        const before = present(startOf(first), "the home's start installation");

        // restart the home's computer, then tag the start page as the owner once it serves the home again
        const restarted = await home.restart();
        await restarted.ready(world.owner.home);
        const ready = world.simulation.clock.now();
        const owner = await restarted.device.session(login);
        const objects = createClient(
            objectPackage,
            owner.endpoint(objectPackage.id, await restarted.device.locate(world.owner.home)),
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
    },
    SCENARIO_MILLISECONDS,
);

/** Find the start installation among a watch's rows, absent once the watch ends. */
function startOf(
    result: IteratorResult<readonly Select<typeof installation.table>[], void>,
): Select<typeof installation.table> | undefined {
    return result.done === true ? undefined : result.value.find((row) => row.name === "start");
}
