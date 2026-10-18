import { Miniflare } from "miniflare";
import { fileURLToPath } from "node:url";
import { eq } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import type { DurableObjectNamespace } from "@destack/db/cloudflare";
import { Package } from "@destack/package";
import { modulePlugin } from "@destack/package/bun";
import { MemoryBuild } from "@destack/package/test";
import { Digest, schema } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { build, symbol } from "../object/index.ts";
import { stackTables } from "../stack/index.ts";
import { ids } from "../test/index.ts";
import { DurableObjectGraphStorage } from "./graph.ts";

/** The two builds of one release the test reads, the second finished later. */
const builds = [
    schema.identifier("build").parse("build-01996ab0-0000-7000-8000-0000000000a1"),
    schema.identifier("build").parse("build-01996ab0-0000-7000-8000-0000000000a2"),
] as const;

test("index a manifest once in its object, and show each build of it the rows in its own scope", async () => {
    // keep a release declaring a notification as two builds of the space
    const reader = await MemoryBuild.of(
        Package.parse({ id: ids.package, name: "@example/notes", version: "2026.9.0" }),
        [{ kind: "notification", kindPackageId: ids.package, name: "mention", description: {} }],
    );
    const manifest = await Digest.json(reader.manifest);
    const storage = await TestDatabase.create("sqlite", stackTables, { isMigrated: true });
    onTestFinished(() => storage.close());
    for (const [index, id] of builds.entries()) {
        await storage.database.insert(build.table).values({
            id,
            scope: ids.space,
            source: { kind: "release", package: ids.package, version: "2026.9.0", manifest },
            outputs: [],
            state: "succeeded",
            manifest,
            startedAt: 1000,
            finishedAt: 1000 + index,
            createdAt: 1000,
            updatedAt: 1000,
        });
    }

    // read each build's notification through the index objects, counting the builds read
    let opened = 0;
    const graphs = new DurableObjectGraphStorage(
        storage.database,
        async () => {
            opened += 1;

            return reader;
        },
        await indexes(),
    );
    const read = async (id: (typeof builds)[number]) => {
        const database = await graphs.open(id);
        const [row] = await database
            .select({
                scope: symbol.table.scope,
                kind: symbol.table.kind,
                name: symbol.table.name,
                createdAt: symbol.table.createdAt,
            })
            .from(symbol.table)
            .where(eq(symbol.table.kind, "notification"));

        return row;
    };

    expect({ rows: [await read(builds[0]), await read(builds[1])], opened }).toEqual({
        // show each build the one stored row in its own scope and finish
        rows: builds.map((id, index) => ({
            scope: id,
            kind: "notification",
            name: "mention",
            createdAt: 1000 + index,
        })),
        // read the build once, for the first build of the manifest
        opened: 1,
    });
});

/** Run the index objects in the Workers runtime, answered from here, until the test ends. */
async function indexes(): Promise<DurableObjectNamespace> {
    // bundle the scenario Worker exporting the index object
    const compiled = await Bun.build({
        entrypoints: [fileURLToPath(new URL("./test/graph.ts", import.meta.url))],
        format: "esm",
        target: "browser",
        conditions: ["workerd", "worker", "browser"],
        external: ["node:*", "cloudflare:*"],
        plugins: [modulePlugin],
    });
    const [output] = compiled.outputs;
    if (output === undefined) {
        throw new Error("the scenario bundled into no file");
    }

    // run it with the index objects
    const worker = new Miniflare({
        modules: true,
        script: await output.text(),
        compatibilityDate: "2026-07-30",
        compatibilityFlags: ["nodejs_compat"],
        durableObjects: { GRAPH: { className: "GraphIndexObject", useSQLite: true } },
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
