import { testSimulation } from "../test/world.ts";
import { expect, refusal, test } from "@destack/test";
import { join } from "node:path";
import { BuildDirectory } from "./directory.ts";

test("copy and write one path at once, keeping one file and refusing different bytes", async () => {
    // write a source file, and a build directory that copies and writes it from two outputs at once
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const source = await files.makeTemporaryDirectory("directory-source-");
    const stylesheet = join(source, "stack.css");
    const bytes = new TextEncoder().encode(".stack { display: flex; }\n");
    await files.writeFile(stylesheet, bytes);
    const directory = new BuildDirectory(
        await files.makeTemporaryDirectory("directory-build-"),
        environment,
    );
    await Promise.all([
        directory.copy("src/stack/stack.css", stylesheet),
        directory.write("src/stack/stack.css", bytes),
    ]);

    // refuse other bytes at the same path
    const different = await refusal(() =>
        directory.write("src/stack/stack.css", new TextEncoder().encode(".stack {}\n")),
    );

    // expect the file once, as the source has it, and the other bytes refused
    expect({
        file: directory.file("src/stack/stack.css").size,
        bytes: await files.readFile(join(directory.directory, "src/stack/stack.css")),
        different,
    }).toEqual({
        file: bytes.length,
        bytes,
        different: [
            "BAD_REQUEST",
            "FILE_INVALID",
            "build file 'src/stack/stack.css' is invalid: it conflicts with another file at its path",
        ],
    });
});
