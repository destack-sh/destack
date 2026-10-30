import { expect, test } from "@destack/test";
import { Simulation } from "../simulation/simulation.ts";

/** The start of the world, in UTC epoch milliseconds. */
const START = Date.UTC(2026, 9, 1);

/** The module the world defines a worker body for. */
const MODULE = new URL("https://app.test/echo.worker.js");

test("run a worker's body on a node of its own, answering messages as the world's work until terminated", async () => {
    // define a body echoing each message with its node's time
    await using simulation = new Simulation({ seed: 1, start: START });
    simulation.workers.define(MODULE, (scope, environment) => {
        scope.addEventListener("message", (event) => {
            const echo: unknown = event.data;
            scope.postMessage({ echo, at: environment.clock.now() - START });
        });
    });

    // start it from a node, send a message and wait for the world to settle
    const { workers } = simulation.environment("tab");
    const worker = workers.start(MODULE, { name: "echo" });
    const answers: unknown[] = [];
    worker.addEventListener("message", (event) => answers.push(event.data));
    worker.postMessage("first");
    await simulation.idle();

    // terminate it, then send another message, which nothing answers
    worker.terminate();
    worker.postMessage("second");
    await simulation.idle();

    expect(answers).toEqual([{ echo: "first", at: 0 }]);

    // refuse a module the world defines no body for
    expect(() => workers.start(new URL("https://app.test/missing.js"))).toThrow(
        new TypeError("this world defines no worker body for https://app.test/missing.js"),
    );
});
