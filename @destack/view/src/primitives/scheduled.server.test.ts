import { TEST_START, testSimulation } from "@destack/test/world";
import { BrowserSimulation } from "@destack/environment/browser/simulation";
import { expect, test } from "@destack/test";
import { debounce, leading, leadingAndTrailing, scheduleIdle, throttle } from "./scheduled.ts";

test("run nothing scheduled on the server, and only the first leading call", async () => {
    // trigger every schedule twice around a second of the server's clock
    await using simulation = testSimulation();
    const { clock } = simulation;
    const { display } = new BrowserSimulation(simulation).environment("tab");
    const called: string[] = [];
    const triggers = [
        debounce((value: string) => called.push(`debounce ${value}`), 100, clock),
        throttle((value: string) => called.push(`throttle ${value}`), 100, clock),
        scheduleIdle((value: string) => called.push(`idle ${value}`), 100, display),
        leading(debounce, (value: string) => called.push(`leading ${value}`), 100, clock),
        leadingAndTrailing(debounce, (value: string) => called.push(`both ${value}`), 100, clock),
    ];
    for (const trigger of triggers) {
        trigger("foo");
    }
    await simulation.clock.sleep(TEST_START + 1000 - simulation.clock.now());
    await simulation.idle();
    for (const trigger of triggers) {
        trigger("baz");
    }

    expect(called).toEqual(["leading foo", "both foo"]);
});
