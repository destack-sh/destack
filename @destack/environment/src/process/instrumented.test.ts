import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";

test("instrument a command a node runs as a span by its executable, timing it", async () => {
    // run a command that exits at once on a node's process
    await using simulation = testSimulation();
    const { process } = simulation.environment("node");
    await simulation.wait(process.run([process.executable, "--version"]));

    // read the command's span by its executable, and its duration's count
    const runs = simulation.telemetry.spans
        .filter((span) => span.name === "destack.process.run")
        .map((span) => span.attributes["process.executable.name"]);
    const counts = await simulation.telemetry.counts();
    const executable = process.executable.split("/").at(-1);
    expect({ runs, timed: counts["destack.process.run.duration.count"] }).toEqual({
        runs: [executable],
        timed: 1,
    });
});
