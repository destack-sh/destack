import { Simulation } from "@destack/environment/simulation";
import { expect, test, refusal } from "@destack/test";
import { join } from "node:path";
import { rolldown } from "rolldown";
import { HostModules } from "./host.ts";

/** The start of every test's world: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

test("classify host modules a runtime keeps, a package replaces with nothing, and the rest as unavailable", async () => {
    // write a package whose browser field replaces node:fs with nothing
    await using simulation = new Simulation({ seed: 1, start: START });
    const { files } = simulation.environment("test");
    const directory = await files.makeTemporaryDirectory("host-");
    await files.writeTextFile(
        join(directory, "package.json"),
        JSON.stringify({ browser: { "node:fs": false } }),
    );
    const importer = join(directory, "index.js");

    // classify on Bun, then in a browser through that package
    const bun = new HostModules("bun", files);
    const browser = new HostModules("browser", files);
    expect([
        await bun.classify("node:fs", undefined),
        await browser.classify("node:fs", importer),
        await browser.classify("node:path", importer),
        await browser.classify("@example/other", importer),
    ]).toEqual(["external", "empty", "unavailable", undefined]);
});

test("refuse a host module the runtime lacks and the importing package keeps", async () => {
    // resolve a host module the browser lacks for a module of no package
    await using simulation = new Simulation({ seed: 1, start: START });
    const { files } = simulation.environment("test");
    const hosts = new HostModules("browser", files);

    expect(await refusal(hosts.resolve("node:path", "/app/index.js"))).toEqual([
        "UNPROCESSABLE_CONTENT",
        "HOST_MODULE_UNAVAILABLE",
        "host module is unavailable on browser: node:path, imported by /app/index.js",
    ]);
});

test("bundle a host module the runtime keeps as an import, and one its package replaces as empty", async () => {
    // write a module importing node:fs, in a package whose browser field replaces it with nothing
    await using simulation = new Simulation({ seed: 1, start: START });
    const { files } = simulation.environment("test");
    const directory = await files.makeTemporaryDirectory("host-");
    await files.writeTextFile(
        join(directory, "package.json"),
        JSON.stringify({ browser: { "node:fs": false } }),
    );
    await files.writeTextFile(
        join(directory, "index.js"),
        'import fs from "node:fs";\nexport const read = fs;\n',
    );

    // bundle it for Bun, then for a browser
    const bundle = async (runtime: "bun" | "browser") => {
        const build = await rolldown({
            cwd: directory,
            input: "index.js",
            plugins: [new HostModules(runtime, files).plugin()],
        });
        const { output } = await build.generate({ format: "esm" });

        return output[0].code;
    };

    expect([await bundle("bun"), await bundle("browser")]).toEqual([
        'import fs from "node:fs";\n//#region index.js\nconst read = fs;\n//#endregion\nexport { read };\n',
        "//#endregion\n//#region index.js\nconst read = {};\n//#endregion\nexport { read };\n",
    ]);
});
