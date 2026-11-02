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
    const worker = workers.start((Worker) => new Worker(MODULE, { type: "module", name: "echo" }));
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
    const missing = new URL("https://app.test/missing.js");
    expect(() => workers.start((Worker) => new Worker(missing, { type: "module" }))).toThrow(
        new TypeError("this world defines no worker body for https://app.test/missing.js"),
    );
});

test("tell the parent of a worker whose body fails to run", async () => {
    // define a body failing as it starts, as a worker failing to load does
    await using simulation = new Simulation({ seed: 1, start: START });
    const failing = new URL("https://app.test/failing.worker.js");
    simulation.workers.define(failing, () => {
        throw new Error("the database cannot open");
    });

    // start it and take its failure
    const { workers } = simulation.environment("tab");
    const worker = workers.start((Worker) => new Worker(failing, { type: "module" }));
    const failures: string[] = [];
    worker.addEventListener("error", (event) => failures.push(event.message));
    await simulation.idle();

    expect(failures).toEqual(["the database cannot open"]);
});
