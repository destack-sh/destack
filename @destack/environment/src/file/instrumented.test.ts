import { SpanRunner } from "../telemetry/runner.ts";
import { SpanPath } from "../telemetry/span.ts";
import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";

test("count a node's quick file reads and writes in the running span, spanning its slow verbs by path", async () => {
    // write a file, read it back and read its status in a span, then list the directory
    await using simulation = testSimulation();
    const node = simulation.environment("node");
    const { files } = node;
    const directory = await files.makeTemporaryDirectory("instrumented-");
    const path = files.join(directory, "notes.txt");
    const { telemetry } = node;
    const run = SpanRunner.of(
        telemetry.tracers.getTracer("test"),
        telemetry.meters.getMeter("test"),
        telemetry,
    );
    const totals = await run("work", {}, async () => {
        await files.writeTextFile(path, "pinned");
        await files.readTextFile(path);
        await files.stat(path);

        return SpanPath.totals(telemetry.context.active());
    });
    await files.readDirectory(directory);

    // expect the quick verbs in the totals and only the listing as a file span
    const spans = simulation.telemetry.spans
        .filter((span) => span.name.startsWith("destack.file."))
        .map((span) => [span.name, span.attributes["file.path"]]);
    expect({ totals, spans }).toEqual({
        totals: { "destack.file.read.count": 1, "destack.file.write.count": 1 },
        spans: [["destack.file.list", directory]],
    });
});
