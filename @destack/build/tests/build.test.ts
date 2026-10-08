import { VERSION_HEADER } from "@destack/service/request";
import { test } from "@destack/test";
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { PackageBuild } from "../src/index.ts";
import {
    Fixture,
    expectFiles,
    expectManifest,
    exported,
    importBuilt,
    isHandlerModule,
    isNoteModule,
    isServiceModule,
    property,
    readBuildFiles,
} from "./fixture.ts";
import * as library from "./fixture/library/request.ts";
import * as service from "./fixture/service/request.ts";
import * as resource from "./fixture/resource/request.ts";
import * as stack from "./fixture/stack/request.ts";
import { requests } from "./fixture/web/request.ts";
import { BuildReader } from "@destack/package/manifest";
import { WorkloadInstance } from "@destack/service/workload";
import { ResourceContext } from "@destack/resource/context";
import { testCallKey } from "@destack/service/test";
import { Catalog } from "@destack/locale";
import { PackageDefinition } from "@destack/package";

/** Complete build scenarios, a web application in each rendering mode. */
const fixtures = [
    { name: "library", request: library.request, mode: "" },
    { name: "service", request: service.request, mode: "" },
    { name: "resource", request: resource.request, mode: "" },
    { name: "stack", request: stack.request, mode: "" },
    ...Object.entries(requests).map(([mode, application]) => ({
        name: "web",
        mode,
        request: { outputs: { website: application } },
    })),
];

test.concurrent.for(fixtures)(
    "relocate the $name $mode build and run its outputs",
    async (fixture, { expect }) => {
        await using input = await Fixture.open(fixture.name);
        await using build = await input.build(fixture.request);

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
                const [first = 0] = bytes;
                bytes[0] = first ^ 1;

                return new Blob([bytes]).stream();
            });
            await expect(corrupt.files()).rejects.toMatchObject({
                code: "INVALID_FILE",
                message: `file digest mismatch: ${build.manifest.lists.files.path}`,
            });

            const module = await importBuilt(
                destination,
                exported(build.manifest.outputs["library"], "."),
            );
            if (!isNoteModule(module)) {
                throw new TypeError("the library exports no createNote function");
            }
            expect(module.createNote("A note")).toEqual({ title: "A note", complete: false });
        }
        // run each output's declared workload
        else if (fixture.name === "service") {
            for (const output of Object.values(build.manifest.outputs)) {
                expect(await callNotes(build, destination, output)).toEqual([
                    200,
                    { path: "/notes" },
                ]);
            }
        }
        // read the declared database from the bundle
        else if (fixture.name === "resource") {
            const module = await importBuilt(
                destination,
                exported(build.manifest.outputs["library"], "."),
            );

            // distribute the declared tables with the database, so the bundle plans the database's migration
            const database = property(module, "database");
            expect(property(database, "tables")).toEqual([property(module, "note")]);
        }
        // read the declared stack from the bundle
        else if (fixture.name === "stack") {
            const module = await importBuilt(
                destination,
                exported(build.manifest.outputs["stack"], "."),
            );

            // retain the shared database's description in the space definition
            const database = property(module, "database");
            const owner = property(database, "package");
            const ownerId = property(owner, "id");
            if (typeof ownerId !== "string") {
                throw new TypeError("the database's package has no identifier");
            }
            const declaration = Object.fromEntries(
                ["name", "kind", "version", "spec"].map((key) => [key, property(database, key)]),
            );
            const definition = property(property(module, "personal"), "definition");
            expect(property(definition, "resources")).toEqual({
                main: {
                    declaration: { package: owner, ...declaration },
                    retention: { within: { days: 30 } },
                    tags: {},
                },
            });
            expect(property(definition, "installations")).toEqual({
                notes: {
                    package: {
                        id: property(property(module, "stack"), "id"),
                        name: "@example/stack",
                        version: "2026.9.0",
                    },
                    status: "enabled",
                    alias: "stack",
                    bindings: {
                        [ownerId]: {
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
            const distributed = await readBuildFiles(build);
            const server = build.manifest.outputs["website-server"];
            if (server?.emit === true) {
                const module = await importBuilt(destination, exported(server, "."));
                if (!isHandlerModule(module)) {
                    throw new TypeError("the server exports no request handler");
                }
                const response = await module.default.fetch(new Request("https://example.test/"));
                const document = await response.text();
                expect([response.status, headings(document)]).toEqual([200, ["Hello Destack"]]);
                expect(missingAssets(document, distributed)).toEqual([]);
            }

            // resolve every generated page's asset references to distributed files
            for (const [path, bytes] of distributed) {
                if (path.endsWith(".html")) {
                    expect(
                        missingAssets(new TextDecoder().decode(bytes), distributed),
                        path,
                    ).toEqual([]);
                }
            }
        }
    },
);

test.concurrent("release a build at a prerelease version its manifest, package.json and modules carry", async ({
    expect,
}) => {
    // build the service at a dev prerelease newer than its declared version, after removing its source checkout
    await using input = await Fixture.open("service");
    await using build = await input.build({ ...service.request, version: "2026.10.0-dev.3" });
    const destination = join(input.directory, "build");
    await build.write(destination);
    await rm(input.source, { recursive: true });

    // call its workload naming the prerelease, which its service refuses unless its modules carry it, and read its package.json
    const output = build.manifest.outputs["bun"];
    const called = output === undefined ? undefined : await callNotes(build, destination, output);
    const manifest: unknown = JSON.parse(await readFile(join(destination, "package.json"), "utf8"));
    expect({
        release: build.manifest.package.version,
        manifest: property(manifest, "version"),
        called,
    }).toEqual({
        release: "2026.10.0-dev.3",
        manifest: "2026.10.0-dev.3",
        called: [200, { path: "/notes" }],
    });
});

/** Start a written build's declared workload from an output's package export and call its notes route with the build's version. */
async function callNotes(
    build: PackageBuild,
    destination: string,
    output: NonNullable<PackageBuild["manifest"]["outputs"][string]>,
): Promise<[number, unknown]> {
    // start the declared workload from its package export
    const module = await importBuilt(destination, exported(output, "./server"));
    if (!isServiceModule(module)) {
        throw new TypeError("the server exports no workload and service");
    }
    await using instance = await WorkloadInstance.start(module.web, {
        callKey: testCallKey,
        report: (error) => {
            throw error;
        },
        resources: new ResourceContext(),
        history: { ingest: async () => 0 },
        runs: {
            send: async () => {
                throw new TypeError("the fixture workload records no runs");
            },
        },
        service: () => ({
            audience: build.manifest.package.id,
            scope: "fixture",
            drainTimeout: 1000,
            authenticate: async () => null,
            authorizeMachine: async () => {},
        }),
    });

    // call the notes route with the build's version
    const response = await instance.fetch(
        module.service,
        new Request("https://example.test/notes", {
            headers: { [VERSION_HEADER]: build.manifest.package.version },
        }),
    );

    return [response.status, await response.json()];
}

/** List the text of a page's headings. */
function headings(document: string): string[] {
    return [...document.matchAll(/<h1[^>]*>([^<]*)<\/h1>/gu)].map((match) => match[1] ?? "");
}

/** List the root-relative asset references of a page that the browser output does not distribute. */
function missingAssets(document: string, distributed: ReadonlyMap<string, Uint8Array>): string[] {
    return [...document.matchAll(/(?:src|href)="\/([^"]+)"/gu)]
        .map((match) => match[1] ?? "")
        .filter((asset) => !distributed.has(`output/website-browser/${asset}`));
}

test.concurrent("refuse a server entry reading a browser global before framework compilation", async ({
    expect,
}) => {
    // write a server entry reading a browser global, which the fixture's DOM library types
    await using input = await Fixture.open("web");
    await writeFile(
        join(input.source, "server.ts"),
        "/** Render the page title. */\nexport function render(): string {\n    return document.title;\n}\n",
    );
    await writeFile(join(input.source, "client.ts"), "export {};\n");

    // refuse the build before framework compilation or rendering
    const application = { ...requests.static, entryServer: "server.ts", entryClient: "client.ts" };
    await expect(input.build({ outputs: { website: application } })).rejects.toMatchObject({
        code: "BUILD_FAILED",
        message: "unsupported bun API: document.title at server.ts:76",
    });
});

test.concurrent("ship a package's own catalogs in its build, refusing a misnamed one and another package's", async ({
    expect,
}) => {
    // write a German catalog of the library's own messages
    await using input = await Fixture.open("library");
    const owner = PackageDefinition.read(
        await readFile(join(input.source, "destack.json"), "utf8"),
    ).id;
    const german = { package: owner, locale: "de", messages: { a1: "Notiz" }, drafts: ["a1"] };
    const write = (name: string, catalog: object) =>
        writeFile(join(input.source, "locale", name), JSON.stringify(catalog));
    await mkdir(join(input.source, "locale"));
    await write("de.json", german);

    // read the shipped catalog back from the build
    await using build = await input.build(library.request);
    const shipped = await Catalog.read(build.reader);

    // refuse a catalog named for another locale, then one of another package
    await write("de.json", { ...german, locale: "de-AT" });
    const misnamed = input.build(library.request);
    await expect(misnamed).rejects.toMatchObject({ message: "invalid catalog: locale/de.json" });
    await write("de.json", { ...german, package: "package-01996ab0-0000-7000-8000-0000000000aa" });
    const foreign = input.build(library.request);
    await expect(foreign).rejects.toMatchObject({ message: "invalid catalog: locale/de.json" });
    expect(shipped).toEqual([german]);
});
