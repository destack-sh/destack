import { testWorld } from "../test/browser.ts";
import type { BrowserEnvironment } from "@destack/environment/browser";
import { Simulation } from "@destack/environment/simulation";
import { expect, test } from "@destack/test";
import { flush } from "solid-js";
import { createPageRoot } from "../test/dom.ts";
import { createIdleTimer, type IdleTimerOptions } from "./idle.ts";

/** Make an idle timer in a root. */
function idleTimer(
    options: IdleTimerOptions,
    environment: BrowserEnvironment,
): ReturnType<typeof createIdleTimer> & { dispose: () => void } {
    return createPageRoot(
        (disposeRoot) => ({ ...createIdleTimer(options), dispose: disposeRoot }),
        environment,
    );
}

/** Let time pass and commit the writes. */
async function wait(milliseconds: number, simulation: Simulation): Promise<void> {
    await simulation.clock.sleep(milliseconds);
    await simulation.idle();
    flush();
}

test("prompt after the idle timeout and go idle after the prompt timeout", async () => {
    const { simulation, tab } = testWorld();
    const timer = idleTimer({ idleTimeout: 5, promptTimeout: 5 }, tab);
    flush();
    const phases: boolean[][] = [];
    for (const step of [2, 5, 5]) {
        await wait(step, simulation);
        phases.push([timer.isPrompted(), timer.isIdle()]);
    }
    timer.dispose();

    expect(phases).toEqual([
        [false, false],
        [true, false],
        [false, true],
    ]);
});

test("start, reset and stop the timers by hand", async () => {
    const { simulation, tab } = testWorld();
    // start, go idle, reset and go idle again
    const timer = idleTimer({ idleTimeout: 5, startManually: true }, tab);
    await wait(50, simulation);
    const unstarted = timer.isIdle();
    timer.start();
    await wait(50, simulation);
    const started = timer.isIdle();
    timer.reset();
    await wait(4, simulation);
    const reset = timer.isIdle();
    await wait(50, simulation);
    const again = timer.isIdle();

    // stop and wait
    timer.stop();
    await wait(50, simulation);
    const stopped = timer.isIdle();
    timer.dispose();

    expect([unstarted, started, reset, again, stopped]).toEqual([false, true, false, true, false]);
});

test("go idle by hand, then report activity and run the timers as before", async () => {
    const { simulation, tab } = testWorld();
    // go idle on an element watched for clicks
    const element = document.createElement("div");
    const seen: string[] = [];
    const timer = idleTimer(
        {
            element,
            events: ["click"],
            idleTimeout: 10,
            onActive: () => seen.push("active"),
            onIdle: (event) => seen.push(`idle ${event.type}`),
            onPrompt: () => seen.push("prompt"),
            startManually: true,
        },
        tab,
    );
    timer.triggerIdle();
    flush();
    const idle = timer.isIdle();

    // click, then wait out the timers
    element.click();
    flush();
    const active = timer.isIdle();
    await wait(15, simulation);
    const idleAgain = timer.isIdle();
    timer.dispose();

    expect({ idle, active, idleAgain, seen }).toEqual({
        idle: true,
        active: false,
        idleAgain: true,
        seen: ["idle manualidle", "active", "prompt", "idle click"],
    });
});

test("watch only the chosen events on the chosen element", async () => {
    const { simulation, tab } = testWorld();
    // run the timers to idle
    const element = document.createElement("div");
    let status = "initial";
    const timer = idleTimer(
        {
            promptTimeout: 30,
            idleTimeout: 30,
            startManually: true,
            onActive: () => (status = "active"),
            onIdle: () => (status = "idle"),
            onPrompt: () => (status = "prompted"),
            element,
            events: ["click"],
        },
        tab,
    );
    await wait(10, simulation);
    const unstarted = status;
    timer.start();
    await wait(50, simulation);
    const prompted = status;
    await wait(60, simulation);
    const idle = status;

    // press a key, then click
    element.dispatchEvent(new KeyboardEvent("keydown", { key: "a" }));
    const afterKey = status;
    element.click();
    timer.dispose();

    expect([unstarted, prompted, idle, afterKey, status]).toEqual([
        "initial",
        "prompted",
        "idle",
        "idle",
        "active",
    ]);
});
