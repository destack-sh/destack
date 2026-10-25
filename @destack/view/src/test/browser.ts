import {
    BrowserSimulation,
    type SimulatedBrowserEnvironment,
} from "@destack/environment/browser/simulation";
import type { Simulation } from "@destack/environment/simulation";
import { onTestFinished } from "@destack/test";
import { testSimulation } from "@destack/test/world";

/** The node the test's tab runs as. */
const TAB_NODE = "tab";

/** The running test's world and tab, made on first use and disposed once the test finishes. */
let current:
    | { readonly simulation: Simulation; readonly tab: SimulatedBrowserEnvironment }
    | undefined;

/** Read the running test's simulated browser tab, which the DOM tests render in. */
export function testBrowser(): SimulatedBrowserEnvironment {
    return testWorld().tab;
}

/** Read the running test's world and tab, made fresh for each test. */
export function testWorld(): {
    readonly simulation: Simulation;
    readonly tab: SimulatedBrowserEnvironment;
} {
    // make the world and its tab once per test, dropping both once the test finishes
    if (current === undefined) {
        const simulation = testSimulation();
        onTestFinished(async () => {
            current = undefined;
            await simulation[Symbol.asyncDispose]();
        });
        current = { simulation, tab: new BrowserSimulation(simulation).environment(TAB_NODE) };
    }

    return current;
}
