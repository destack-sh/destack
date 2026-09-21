import { test } from "@destack/test";
import { readFile, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { PackageBuild } from "../src/index.ts";
import { Fixture, expectBuild, expectFiles } from "./fixture.ts";
import { request } from "./fixture/library/request.ts";
import { request as serviceRequest } from "./fixture/service/request.ts";
import { request as resourceRequest } from "./fixture/resource/request.ts";
import { request as stackRequest } from "./fixture/stack/request.ts";
import { requests } from "./fixture/web/request.ts";

/** Complete build scenarios and their expected output directories. */
const fixtures = [
    { name: "library", request, expected: "expected/" },
    { name: "service", request: serviceRequest, expected: "expected/" },
    { name: "resource", request: resourceRequest, expected: "expected/" },
    { name: "stack", request: stackRequest, expected: "expected/" },
    ...Object.entries(requests).map(([mode, application]) => ({
        name: "web",
        mode,
        request: { outputs: { website: application } },
        expected: `expected/${mode}/`,
    })),
];

test.concurrent.for(fixtures)("build $name $expected", async (fixture, { expect }) => {
    await using input = await Fixture.open(fixture.name);
    const build = await input.build(fixture.request);
    await expectBuild(build, new URL(fixture.expected, input.fixture));

    // load the complete distribution after removing its source checkout
    const destination = join(input.directory, "build");
    await build.write(destination);
    await rm(input.source, { recursive: true });
    const restored = await PackageBuild.read(destination);
    expect(restored.manifest).toEqual(build.manifest);
    expectFiles(restored.files, build.files);

    // exercise the exported package API from the relocated build
    if (fixture.name === "library") {
        const module = await import(
            pathToFileURL(join(destination, build.manifest.outputs.library.exports["."])).href
        );
        expect(module.createNote("A note")).toEqual({ title: "A note", complete: false });
    } else if (fixture.name === "service") {
        for (const output of Object.values(build.manifest.outputs)) {
            const module = await import(
                pathToFileURL(join(destination, output.exports["./server"])).href
            );
            const response = await module.fetch(new Request("https://example.test/notes"));
            expect([response.status, await response.json()]).toEqual([200, { path: "/notes" }]);
            expect(module.remind({ id: "occurrence-1" })).toBe("occurrence-1");
        }
    } else if (fixture.name === "resource") {
        const module = await import(
            pathToFileURL(join(destination, build.manifest.outputs.library.exports["."])).href
        );
        expect(module.database.spec).toEqual({ dialect: "sqlite" });
        for (const dialect of ["sqlite", "postgresql"]) {
            const migration = new URL(
                `${dialect}/20260920000000_note/migration.sql`,
                module.notes.migrations,
            );
            expect(await readFile(migration, "utf8")).toBe(
                "CREATE TABLE note (id INTEGER PRIMARY KEY, title TEXT NOT NULL);\n",
            );
        }
    } else if (fixture.name === "stack") {
        const module = await import(
            pathToFileURL(join(destination, build.manifest.outputs.stack.exports["."])).href
        );
        expect(module.personal.resources).toEqual({
            main: { declaration: module.database, retention: "retain", tags: {} },
        });
    } else if (fixture.name === "web") {
        const server = build.manifest.outputs["website-server"];
        if (server) {
            const module = await import(pathToFileURL(join(destination, server.exports["."])).href);
            const response = await module.default.fetch(new Request("https://example.test/"));
            expect(response.status).toBe(200);
            await expect(await response.text()).toMatchFileSnapshot(
                fileURLToPath(
                    new URL(
                        `expected/${"mode" in fixture ? fixture.mode : ""}.html`,
                        input.fixture,
                    ),
                ),
            );
        }

        // check every generated HTML asset reference against distributed files
        for (const [path, bytes] of build.files) {
            if (!path.endsWith(".html")) {
                continue;
            }
            const document = new TextDecoder().decode(bytes);
            const assets = [...document.matchAll(/(?:src|href)="\/([^"]+)"/g)].map(
                (match) => match[1],
            );
            expect(
                assets.filter((asset) => !build.files.has(`output/website-browser/${asset}`)),
            ).toEqual([]);
        }
    }
});

/** Invalid declarations rejected before framework compilation or rendering. */
const invalid = [
    {
        file: "src/app.test.ts",
        source: 'import { test } from "@destack/test";\ntest(String("renders"), () => {});\n',
        code: "INSPECTION_FAILED",
        message: "Test declaration requires a literal title: src/app.test.ts:38",
        application: requests.browser,
    },
    {
        file: "src/app.test.ts",
        source: 'import { test } from "@destack/test";\nif (true) { test("renders", () => {}); }\n',
        code: "INSPECTION_FAILED",
        message: "Test declaration requires module or suite scope: src/app.test.ts:50",
        application: requests.browser,
    },
    {
        file: "server.ts",
        source: "export function render(): string {\n    return document.title;\n}\n",
        code: "BUILD_FAILED",
        message: "Unsupported deno API: document.title at server.ts:46",
        application: { ...requests.static, entryServer: "server.ts", entryClient: "client.ts" },
    },
];

test.concurrent.for(invalid)("reject $message", async (fixture, { expect }) => {
    await using input = await Fixture.open("web");
    await writeFile(join(input.source, fixture.file), fixture.source);
    if ("entryServer" in fixture.application) {
        await writeFile(join(input.source, "client.ts"), "export {};\n");
        await writeFile(
            join(input.source, "tsconfig.json"),
            JSON.stringify({
                compilerOptions: {
                    target: "ESNext",
                    module: "ESNext",
                    moduleResolution: "bundler",
                    allowImportingTsExtensions: true,
                    noEmit: true,
                    strict: true,
                    skipLibCheck: true,
                    jsx: "preserve",
                    jsxImportSource: "@destack/view",
                },
                include: ["src/**/*.ts", "src/**/*.tsx"],
            }),
        );
    }
    const actual = await input.build({ outputs: { website: fixture.application } }).then(
        () => {
            throw new Error("expected fixture rejection");
        },
        (error) => ({ code: error.code, message: error.message }),
    );
    expect(actual).toEqual({
        code: fixture.code,
        message: fixture.message,
    });
});
