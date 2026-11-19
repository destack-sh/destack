import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { SimulatedProcess } from "./simulated.ts";

/** A process with no arguments in the root, as the simulation's runtime runs it. */
const OPTIONS = {
    arguments: [],
    variables: {},
    workingDirectory: "/",
    id: 1,
    userId: undefined,
    executable: "/bin/bun",
    script: "/main.ts",
    runtime: { revision: "0".repeat(40), isStandalone: false },
} as const;

test("report a terminal's size and take raw mode on it, and report no size and refuse raw mode on a pipe", async () => {
    // run one process on a terminal and one on a pipe
    await using simulation = testSimulation();
    const terminal = new SimulatedProcess({ ...OPTIONS, isTerminal: true }, simulation.scheduler);
    const piped = new SimulatedProcess(OPTIONS, simulation.scheduler);

    // switch the terminal to raw mode, and refuse it on the pipe
    terminal.stdin.setRawMode(true);
    expect(() => piped.stdin.setRawMode(true)).toThrow(
        new TypeError("standard input is invalid: it is no terminal"),
    );
    expect({
        terminal: [terminal.stdout.columns, terminal.stdout.rows],
        piped: [piped.stdout.columns, piped.stdout.rows],
    }).toEqual({ terminal: [80, 24], piped: [undefined, undefined] });
});
