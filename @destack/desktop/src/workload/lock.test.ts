import { testSimulation } from "@destack/object/test";
import { expect, refusal, test } from "@destack/test";
import { DesktopLock } from "./lock.ts";

test("hold a home's lock while one desktop runs, refusing a second desktop until it quits", async () => {
    // take the lock as a running desktop
    const environment = testSimulation().environment("desktop");
    const { home } = environment.host;
    const running = new DesktopLock(home, environment);
    const held = await running.acquire();

    // probe it and start a second desktop, then quit the first and probe again
    const second = new DesktopLock(home, environment);
    const whileRunning = await second.isHeld();
    const refused = await refusal(() => second.acquire());
    await held[Symbol.asyncDispose]();

    // expect the lock held and the second refused, then free
    expect({ whileRunning, refused, afterQuit: await second.isHeld() }).toEqual({
        whileRunning: true,
        refused: ["CONFLICT", "DESKTOP_RUNNING", "another Destack desktop runs for this user"],
        afterQuit: false,
    });
});
