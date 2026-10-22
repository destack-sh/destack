import { CompilerPool } from "@destack/build";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import type { PackageBuild } from "@destack/build";
import { PackageCopy, testStore } from "@destack/build/test";
import { Clock } from "@destack/environment";
import { ResourceContext } from "@destack/resource/context";
import type { PackageHandle } from "@destack/object";
import { testCallKey } from "@destack/object/test";
import { type Workload, WorkloadInstance } from "@destack/object/workload";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import * as resource from "./fixture/resource/request.ts";
import * as stack from "./fixture/stack/request.ts";
import * as workload from "./fixture/workload/request.ts";

/** The resource fixture, a package declaring a database. */
const RESOURCE = new URL("./fixture/resource/source/", import.meta.url);

/** The stack fixture, a stack importing declarations from another package. */
const STACK = new URL("./fixture/stack/source/", import.meta.url);

/** The workload fixture, a workload compiled for Bun and workerd. */
const WORKLOAD = new URL("./fixture/workload/source/", import.meta.url);

/** The longest a fixture's build on its stored closure takes. */
const BUILD_MILLISECONDS = 60_000;

/** A built server module exporting its workload and the package it serves. */
interface WorkloadModule {
    /** The workload serving the package. */
    readonly web: Workload;
    /** The package's handle. */
    readonly notes: PackageHandle;
}

/** Report whether a module exports a workload and a package handle. */
function isWorkloadModule(module: unknown): module is WorkloadModule {
    const served =
        typeof module === "object" && module !== null && "web" in module ? module.web : undefined;
    const handle =
        typeof module === "object" && module !== null && "notes" in module
            ? module.notes
            : undefined;

    return (
        typeof served === "object" &&
        served !== null &&
        "start" in served &&
        typeof served.start === "function" &&
        typeof handle === "object" &&
        handle !== null &&
        "router" in handle
    );
}

/** Read a property of a built object. */
function property(value: unknown, name: string): unknown {
    if (typeof value !== "object" || value === null) {
        throw new TypeError(`the built value holds no object with ${name}`);
    }
    const read: unknown = Reflect.get(value, name);

    return read;
}

/** Import an output's export of a deployed build. */
async function importExport(
    destination: string,
    build: PackageBuild,
    output: string,
    path: string,
): Promise<unknown> {
    const file = present(build.manifest.outputs[output]?.exports[path], `${output} ${path}`);

    return await import(pathToFileURL(join(destination, file)).href);
}

/** Start a deployed build's workload from an output's server export and call its feed. */
async function callFeed(
    destination: string,
    build: PackageBuild,
    output: string,
): Promise<[number, unknown]> {
    // start the declared workload from its package export
    const module = await importExport(destination, build, output, "./server");
    if (!isWorkloadModule(module)) {
        throw new TypeError("the server exports no workload or package handle");
    }
    await using instance = await WorkloadInstance.start(module.web, {
        clock: Clock.system,
        callKey: testCallKey,
        report: (error) => {
            throw error;
        },
        resources: new ResourceContext(),
        serverOptions: () => ({
            scope: "fixture",
            drainTimeout: 1000,
            authenticate: async () => undefined,
            authorizeMachine: async () => {},
        }),
    });

    // call the package's feed
    const response = await instance.fetch(module.notes, new Request("https://example.test/feed"));

    return [response.status, await response.json()];
}

test(
    "run the workload's Bun and workerd builds from their placed trees, at a dev version too",
    { timeout: BUILD_MILLISECONDS },
    async () => {
        // build the workload at its release and at a dev version, and place both as a machine does
        await using store = await testStore();
        await using compilers = await CompilerPool.open(store);
        await using source = await PackageCopy.open(WORKLOAD, compilers);
        await using released = await source.build(workload.request);
        await using dev = await source.build({
            ...workload.request,
            versions: { "@destack/build-workload-fixture": "2026.9.0-dev.3" },
        });
        await source.deploy(released, join(source.directory, "released"));
        await source.deploy(dev, join(source.directory, "dev"));

        // call each output's feed, which answers the version its modules carry, and read the dev package.json
        const manifest: unknown = JSON.parse(
            await readFile(join(source.directory, "dev", "package.json"), "utf8"),
        );
        expect({
            bun: await callFeed(join(source.directory, "released"), released, "bun"),
            worker: await callFeed(join(source.directory, "released"), released, "worker"),
            dev: await callFeed(join(source.directory, "dev"), dev, "bun"),
            devRelease: dev.manifest.package.version,
            devManifest: property(manifest, "version"),
        }).toEqual({
            bun: [200, { path: "/feed", version: "2026.9.0" }],
            worker: [200, { path: "/feed", version: "2026.9.0" }],
            dev: [200, { path: "/feed", version: "2026.9.0-dev.3" }],
            devRelease: "2026.9.0-dev.3",
            devManifest: "2026.9.0-dev.3",
        });
    },
);

test(
    "distribute the declared tables with the resource's database",
    { timeout: BUILD_MILLISECONDS },
    async () => {
        // build the resource and read its database from the placed build
        await using store = await testStore();
        await using compilers = await CompilerPool.open(store);
        await using source = await PackageCopy.open(RESOURCE, compilers);
        await using build = await source.build(resource.request);
        const destination = join(source.directory, "deployed");
        await source.deploy(build, destination);
        const module = await importExport(destination, build, "library", ".");

        // distribute the tables with the database, so the build plans the database's migration
        const database = property(module, "database");
        expect(property(database, "tables")).toEqual([property(module, "note")]);
    },
);

test(
    "retain the shared bucket's description and the installation in the stack's definition",
    { timeout: BUILD_MILLISECONDS },
    async () => {
        // build the stack and read its definition from the placed build
        await using store = await testStore();
        await using compilers = await CompilerPool.open(store);
        await using source = await PackageCopy.open(STACK, compilers);
        await using build = await source.build(stack.request);
        const destination = join(source.directory, "deployed");
        await source.deploy(build, destination);
        const module = await importExport(destination, build, "stack", ".");

        // read the bucket's owner and declaration, and the stack's personal definition
        const files = property(module, "files");
        const owner = property(files, "package");
        const ownerId = property(owner, "id");
        if (typeof ownerId !== "string") {
            throw new TypeError("the bucket's package has no identifier");
        }
        const declaration = Object.fromEntries(
            ["name", "kind", "version", "spec"].map((key) => [key, property(files, key)]),
        );
        const definition = property(property(module, "personal"), "definition");
        expect({
            resources: property(definition, "resources"),
            installations: property(definition, "installations"),
        }).toEqual({
            resources: {
                files: {
                    declaration: { package: owner, ...declaration },
                    retention: { within: { days: 30 } },
                    tags: {},
                },
            },
            installations: {
                notes: {
                    package: {
                        id: property(property(module, "stack"), "id"),
                        name: "@example/stack",
                        version: "2026.9.0",
                    },
                    status: "enabled",
                    name: "stack",
                    bindings: {
                        [ownerId]: {
                            files: { target: { type: "resource", name: "files" }, state: {} },
                        },
                    },
                    compute: {},
                    capabilities: [],
                    directories: [],
                    tags: {},
                },
            },
        });
    },
);
