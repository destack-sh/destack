import { VERSION_HEADER } from "@destack/service/request";
import { test } from "@destack/test";
import { readFile, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { PackageBuild } from "../src/index.ts";
import { Fixture, expectBuild, expectFiles, expectManifest, readBuildFiles } from "./fixture.ts";
import * as library from "./fixture/library/request.ts";
import * as service from "./fixture/service/request.ts";
import * as resource from "./fixture/resource/request.ts";
import * as stack from "./fixture/stack/request.ts";
import { requests } from "./fixture/web/request.ts";
import { BuildReader } from "@destack/package/manifest";
import { WorkloadInstance } from "@destack/service/workload";
import { ResourceContext } from "@destack/resource/context";

/** Complete build scenarios and their expected output directories. */
const fixtures = [
    { name: "library", request: library.request, expected: "expected/" },
    { name: "service", request: service.request, expected: "expected/" },
    { name: "resource", request: resource.request, expected: "expected/" },
    { name: "stack", request: stack.request, expected: "expected/" },
    ...Object.entries(requests).map(([mode, application]) => ({
        name: "web",
        mode,
        request: { outputs: { website: application } },
        expected: `expected/${mode}/`,
    })),
];

test.concurrent.for(fixtures)("build $name $expected", async (fixture, { expect }) => {
    await using input = await Fixture.open(fixture.name);
    await using build = await input.build(fixture.request);
    await expectBuild(build, new URL(fixture.expected, input.fixture));

    // load the complete distribution after removing its source checkout
    const destination = join(input.directory, "build");
    await build.write(destination);
    await rm(input.source, { recursive: true });
    const restored = await PackageBuild.read(destination);
    expect(restored.manifest).toEqual(build.manifest);
    await expectFiles(restored, build);

    await expectManifest(build, destination);

    // reject corrupted description bytes, then call the library's exports from the relocated build
    if (fixture.name === "library") {
        const corrupt = new BuildReader(build.manifest, async (path) => {
            const bytes = new Uint8Array(await readFile(join(destination, path)));
            bytes[0] ^= 1;

            return bytes;
        });
        await expect(corrupt.files()).rejects.toMatchObject({
            code: "INVALID_FILE",
            message: `file digest mismatch: ${build.manifest.files.path}`,
        });

        const module = await import(
            pathToFileURL(join(destination, build.manifest.outputs.library.exports["."])).href
        );
        expect(module.createNote("A note")).toEqual({ title: "A note", complete: false });
    }
    // run each output's declared workload
    else if (fixture.name === "service") {
        for (const output of Object.values(build.manifest.outputs)) {
            // start the declared workload from its package export
            const module = await import(
                pathToFileURL(join(destination, output.exports["./server"])).href
            );
            await using instance = await WorkloadInstance.start(module.web, {
                resources: new ResourceContext(),
                history: { ingest: async () => 0 },
                replicas: {
                    scope: "fixture",
                    source: {
                        stream: () => {
                            throw new TypeError("the fixture workload keeps no copies");
                        },
                    },
                },
                service: () => ({
                    audience: build.manifest.package.id,
                    scope: "fixture",
                    drainTimeout: 1000,
                    authenticate: async () => null,
                    authorizeHost: async () => {},
                }),
            });
            const response = await instance.fetch(
                module.service,
                new Request("https://example.test/notes", {
                    headers: { [VERSION_HEADER]: build.manifest.package.version },
                }),
            );
            expect([response.status, await response.json()]).toEqual([200, { path: "/notes" }]);
            await instance.deliver(
                module.reminders,
                { scheduledAt: Date.now() },
                AbortSignal.timeout(1000),
            );
        }
    }
    // read the declared database from the bundle
    else if (fixture.name === "resource") {
        const module = await import(
            pathToFileURL(join(destination, build.manifest.outputs.library.exports["."])).href
        );

        // distribute the declared tables with the database, so the bundle plans its own migration
        expect(module.database.tables).toEqual([module.note]);
    }
    // read the declared stack from the bundle
    else if (fixture.name === "stack") {
        const module = await import(
            pathToFileURL(join(destination, build.manifest.outputs.stack.exports["."])).href
        );

        // retain the shared database's description in the space definition
        const { package: owner, name, kind, version, spec } = module.database;
        expect(module.personal.definition.resources).toEqual({
            main: {
                declaration: { package: owner, name, kind, version, spec },
                retention: "retain",
                tags: {},
            },
        });
        expect(module.personal.definition.installations).toEqual({
            notes: {
                package: {
                    id: module.stack.id,
                    name: "@example/stack",
                    version: "2026.9.0",
                },
                status: "enabled",
                alias: "stack",
                bindings: {
                    [module.database.package.id]: {
                        main: {
                            target: { type: "resource", name: "main" },
                            state: { tables: { sqlite: [], postgresql: [] } },
                        },
                    },
                },
                compute: {},
                tags: {},
            },
        });
    } else if (fixture.name === "web") {
        const server = build.manifest.outputs["website-server"];
        if (server?.emit) {
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
        const distributed = await readBuildFiles(build);
        for (const [path, bytes] of distributed) {
            if (!path.endsWith(".html")) {
                continue;
            }
            const document = new TextDecoder().decode(bytes);
            const assets = [...document.matchAll(/(?:src|href)="\/([^"]+)"/g)].map(
                (match) => match[1],
            );
            expect(
                assets.filter((asset) => !distributed.has(`output/website-browser/${asset}`)),
            ).toEqual([]);
        }
    }
});

test.concurrent("reject unsupported bun API: document.title at server.ts:76", async ({
    expect,
}) => {
    // write a server entry reading a browser global, beside the configuration compiling it
    await using input = await Fixture.open("web");
    await writeFile(
        join(input.source, "server.ts"),
        "/** Render the page title. */\nexport function render(): string {\n    return document.title;\n}\n",
    );
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

    // refuse the build before framework compilation or rendering
    const application = { ...requests.static, entryServer: "server.ts", entryClient: "client.ts" };
    const actual = await input.build({ outputs: { website: application } }).then(
        () => {
            throw new Error("expected fixture rejection");
        },
        (error) => ({ code: error.code, message: error.message }),
    );
    expect(actual).toEqual({
        code: "BUILD_FAILED",
        message: "unsupported bun API: document.title at server.ts:76",
    });
});
