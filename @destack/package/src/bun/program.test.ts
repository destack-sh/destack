import { join } from "node:path";
import { testHost } from "@destack/test/host";
import { expect, test } from "@destack/test";

/** The program that links its own @destack/schema for the modules it loads. */
const PROGRAM = join(import.meta.dirname, "test", "linked.ts");

test("link a compiled program's own module instance for a module it loads from disk", async () => {
    // compile the program and write a module that imports @destack/schema outside any workspace
    const host = testHost();
    const directory = await host.files.makeTemporaryDirectory("link-");
    const outfile = join(directory, "program");
    const command = [
        host.process.executable,
        "build",
        "--compile",
        PROGRAM,
        `--outfile=${outfile}`,
    ];
    const compiled = await host.process.spawn(command).output();
    expect(compiled.code).toEqual(0);
    const loaded = join(directory, "loaded.js");
    await host.files.writeTextFile(loaded, 'export { schema } from "@destack/schema";\n');

    // run it: the loaded module resolves @destack/schema only through the link, to the program's instance
    const ran = await host.process
        .spawn([outfile, loaded], { workingDirectory: directory })
        .output();
    expect(new TextDecoder().decode(ran.stdout)).toEqual(
        `${JSON.stringify({ isShared: true, shares: [true, false] })}\n`,
    );
});
