import { expect, onTestFinished, test, refusal } from "@destack/test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { HostModules } from "./host.ts";

test("classify host modules a runtime keeps, a package replaces with nothing, and the rest as unavailable", async () => {
    // write a package whose browser field replaces node:fs with nothing
    const directory = await mkdtemp(join(tmpdir(), "destack-host-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    await writeFile(
        join(directory, "package.json"),
        JSON.stringify({ browser: { "node:fs": false } }),
    );
    const importer = join(directory, "index.js");

    // classify on Bun, then in a browser through that package
    const bun = new HostModules("bun");
    const browser = new HostModules("browser");
    expect([
        bun.classify("node:fs", undefined),
        browser.classify("node:fs", importer),
        browser.classify("node:path", importer),
        browser.classify("@example/other", importer),
    ]).toEqual(["external", "empty", "unavailable", undefined]);
});

test("refuse a host module the runtime lacks and the importing package keeps", async () => {
    expect(
        await refusal(() => new HostModules("browser").resolve("node:path", "/app/index.js")),
    ).toEqual([
        "UNPROCESSABLE_CONTENT",
        "HOST_MODULE_UNAVAILABLE",
        "host module is unavailable on browser: node:path, imported by /app/index.js",
    ]);
});
