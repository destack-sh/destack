import { WORKERD_COMPATIBILITY_DATE } from "@destack/package/runtime";
import { Miniflare } from "miniflare";
import { fileURLToPath } from "node:url";
import { eq } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import type { DurableObjectNamespace } from "@destack/db/cloudflare";
import { Package } from "@destack/package";
import { modulePlugin } from "@destack/package/vite";
import { ModuleTransformer } from "@destack/package/transform";
import { rolldown } from "rolldown";
import { testHost } from "@destack/test/host";
import { testSimulation } from "@destack/object/test";
import { MemoryBuild } from "@destack/package/test";
import { Digest, type Identifier, schema } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { build, declaration } from "../object/index.ts";
import { spaceTables } from "../stack/index.ts";
import { ids } from "../test/index.ts";
import { DurableObjectGraphStorage } from "./graph.ts";

/** How long the index objects keep an index past its last view in the test: 300 ms. */
const LEASE_MILLISECONDS = 300;

/** The two builds of one release the test reads, the second finished later. */
const builds = [
    schema.identifier("build").parse("build-01996ab0-0000-7000-8000-0000000000a1"),
    schema.identifier("build").parse("build-01996ab0-0000-7000-8000-0000000000a2"),
] as const;

test("index a manifest once in its object, show each build of it the rows in its own scope, and delete the object a lease after its last view", async () => {
    // record a release declaring a notification as two builds of the space, in a world of its own
    const simulation = testSimulation();
    const world = simulation.environment("test");
    const reader = await MemoryBuild.of(
        Package.parse({ id: ids.package, name: "@example/notes", version: "2026.9.0" }),
        [{ kind: "notification", kindPackageId: ids.package, name: "mention", description: {} }],
        world.crypto,
    );
    const manifest = await Digest.json(reader.manifest, world.crypto);
    const storage = await TestDatabase.create(spaceTables({}).tables, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    for (const [index, id] of builds.entries()) {
        await storage.database.transaction(
            async (transaction) =>
                await transaction.insert(build.table).values({
                    id,
                    scope: ids.space,
                    source: {
                        kind: "release",
                        package: ids.package,
                        version: "2026.9.0",
                        manifest,
                    },
                    outputs: [],
                    state: "succeeded",
                    manifest,
                    startedAt: 1000,
                    finishedAt: 1000 + index,
                    createdAt: 1000,
                    updatedAt: 1000,
                }),
        );
    }

    // read each build's notification through the index objects, counting the builds read
    let opened = 0;
    const namespace = await indexes();
    const graphs = () =>
        new DurableObjectGraphStorage(
            storage.database,
            {
                contains: async () => true,
                contents: async () => {
                    opened += 1;

                    return reader;
                },
                referrer: async () => undefined,
                file: () => Promise.reject(new TypeError("the builds carry no code index")),
            },
            namespace,
            LEASE_MILLISECONDS,
        );
    const first = graphs();
    const rows = [
        await readNotification(first, builds[0]),
        await readNotification(first, builds[1]),
    ];

    // keep the index through a read within the lease, then let the lease end unviewed
    await readNotification(first, builds[0]);
    const kept = opened;
    await testHost().clock.sleep(LEASE_MILLISECONDS * 2);

    // index it again once a build views the deleted index after its renewal is due
    simulation.clock.advance(LEASE_MILLISECONDS);
    await readNotification(first, builds[0]);

    expect({ rows, kept, opened }).toEqual({
        // show each build the one stored row in its own scope and finish
        rows: builds.map((id, index) => ({
            scope: id,
            kind: "notification",
            name: "mention",
            createdAt: 1000 + index,
        })),
        // read the build once while kept
        kept: 1,
        // read the build once for the first build of the manifest, and again once its object was deleted
        opened: 2,
    });
});

/** Read a build's notification declaration through its index object. */
async function readNotification(graph: DurableObjectGraphStorage, id: Identifier<"build">) {
    const database = await graph.open(id);
    const [row] = await database
        .select({
            scope: declaration.table.scope,
            kind: declaration.table.kind,
            name: declaration.table.name,
            createdAt: declaration.table.createdAt,
        })
        .from(declaration.table)
        .where(eq(declaration.table.kind, "notification"));

    return row;
}

/** Run the index objects in the Workers runtime, answered from here, until the test ends. */
async function indexes(): Promise<DurableObjectNamespace> {
    // bundle the scenario Worker exporting the index object for workerd
    const bundle = await rolldown({
        input: fileURLToPath(new URL("./test/graph.ts", import.meta.url)),
        platform: "browser",
        resolve: { conditionNames: ["workerd", "worker", "browser"] },
        external: [/^node:/u, /^cloudflare:/u],
        plugins: [modulePlugin(new ModuleTransformer(testHost().files))],
    });
    const { output } = await bundle.generate({ format: "esm" });
    const [chunk] = output;

    // run it with the index objects
    const worker = new Miniflare({
        modules: true,
        script: chunk.code,
        compatibilityDate: WORKERD_COMPATIBILITY_DATE,
        compatibilityFlags: ["nodejs_compat"],
        durableObjects: { GRAPH: { className: "GraphIndexObject", useSQLite: true } },
        bindings: { DESTACK_GRAPH_LEASE: String(LEASE_MILLISECONDS) },
    });
    onTestFinished(() => worker.dispose());
    const namespace = await worker.getDurableObjectNamespace("GRAPH");

    // send each request to its object, in this runtime's request and response
    const objects: DurableObjectNamespace = {
        idFromName: (name) => name,
        get: (name) => {
            const stub = namespace.get(namespace.idFromName(schema.string().parse(name)));

            return {
                fetch: async (request) => {
                    const answered = await stub.fetch(request.url, {
                        method: request.method,
                        body: await request.text(),
                    });

                    return new Response(await answered.text(), { status: answered.status });
                },
            };
        },
        jurisdiction: () => objects,
    };

    return objects;
}
