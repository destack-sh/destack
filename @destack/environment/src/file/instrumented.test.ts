import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";

test("instrument a node's file reads and writes as spans by path, leaving status reads dark", async () => {
    // write a file, read it back and read its status on a node's files
    await using simulation = testSimulation();
    const { files } = simulation.environment("node");
    const directory = await files.makeTemporaryDirectory("instrumented-");
    const path = files.join(directory, "notes.txt");
    await files.writeTextFile(path, "pinned");
    await files.readTextFile(path);
    await files.stat(path);

    // read the file spans by name and path, after the temporary directory's
    const spans = simulation.telemetry.spans
        .filter(
            (span) => span.name.startsWith("destack.file.") && span.name !== "destack.file.mkdir",
        )
        .map((span) => [span.name, span.attributes["file.path"]]);
    expect(spans).toEqual([
        ["destack.file.write", path],
        ["destack.file.read", path],
    ]);
});
