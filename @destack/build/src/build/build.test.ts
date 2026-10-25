import { testSimulation } from "../test/world.ts";
import { expect, test } from "@destack/test";
import { join } from "node:path";
import { outputsOf } from "./build.ts";
import { readPackageJson } from "../source/index.ts";

test("read one module per runtime the shipped exports compile for", async () => {
    // declare packages with every runtime, with runtimes per export, with a test layer and without runtimes
    await using simulation = testSimulation();
    const { files } = simulation.environment("test");
    const directory = await files.makeTemporaryDirectory("outputs-");
    const read = async (exports: object, definition: object) => {
        await files.writeTextFile(
            join(directory, "package.json"),
            JSON.stringify({ name: "@example/outputs", version: "2026.9.0", exports }),
        );
        await files.writeTextFile(
            join(directory, "destack.json"),
            JSON.stringify({
                $schema: "https://destack.app/schemas/2026.9.0/destack.json",
                ...definition,
            }),
        );
        try {
            return outputsOf(await readPackageJson(directory, files));
        } catch (error) {
            if (!(error instanceof Error)) {
                throw error;
            }

            return error.message;
        }
    };
    const id = "package-01996ab0-0000-7000-8000-00000000000c";
    const exports = { ".": "./src/index.ts", "./server": "./src/server.ts" };
    try {
        expect([
            await read(exports, {
                id,
                language: "typescript",
                runtimes: ["browser", "bun", "workerd"],
            }),
            await read(exports, {
                id,
                language: "typescript",
                runtimes: ["browser"],
                exports: { "./server": { runtimes: ["workerd"] } },
            }),
            await read(
                { ...exports, "./test": "./src/test.ts" },
                {
                    id,
                    language: "typescript",
                    runtimes: ["browser"],
                    exports: { "./test": { runtimes: ["bun"] } },
                },
            ),
            await read(exports, { id, language: "typescript" }),
        ]).toEqual([
            {
                browser: { kind: "module", runtime: "browser" },
                bun: { kind: "module", runtime: "bun" },
                workerd: { kind: "module", runtime: "workerd" },
            },
            {
                browser: { kind: "module", runtime: "browser" },
                workerd: { kind: "module", runtime: "workerd" },
            },
            { browser: { kind: "module", runtime: "browser" } },
            "no runtimes declared for export: .",
        ]);
    } finally {
        await files.remove(directory, { recursive: true });
    }
});
