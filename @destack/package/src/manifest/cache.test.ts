import { expect, test } from "@destack/test";
import { PackageFile } from "../file/file.ts";
import { Package } from "../definition/package.ts";
import { BuildCache } from "./cache.ts";
import { BuildReader } from "./reader.ts";

/** The package whose builds the cache reads. */
const app = Package.parse({
    id: "package-019f5530-8000-7000-8000-000000000002",
    name: "@example/app",
    version: "2026.9.0",
});

/** Open a reader of a build whose graph root has some text, reading no files. */
async function build(graph: string): Promise<BuildReader> {
    const root = await PackageFile.describe(
        "manifest/graph.json",
        "application/json",
        new TextEncoder().encode(graph),
    );

    return new BuildReader(
        {
            formatVersion: 1,
            package: app,
            language: "typescript",
            lists: { dependencies: root, files: root, sourceMaps: root, graph: root },
            outputs: {},
        },
        () => Promise.reject(new Error("no files")),
    );
}

test("read each graph once across readers, again after a failed read, and drop the least recently read beyond the capacity", async () => {
    // read values counting the reads of each graph, failing the first read of the third
    const reads: string[] = [];
    const cache = new BuildCache(async (reader) => {
        const digest = reader.manifest.lists.graph.digest;
        reads.push(digest);
        if (digest === third.manifest.lists.graph.digest && reads.length === 3) {
            throw new Error("unreadable graph");
        }

        return digest.slice(0, 8);
    }, 2);
    const first = await build("first");
    const second = await build("second");
    const third = await build("third");

    // read the first graph once through two readers and the second once
    expect(await cache.read(first)).toBe(first.manifest.lists.graph.digest.slice(0, 8));
    await cache.read(await build("first"));
    await cache.read(second);

    // read the third again after its failed read, dropping the first as the least recently read
    await expect(cache.read(third)).rejects.toThrow("unreadable graph");
    await cache.read(third);
    await cache.read(second);
    await cache.read(first);

    expect(reads).toEqual([
        first.manifest.lists.graph.digest,
        second.manifest.lists.graph.digest,
        third.manifest.lists.graph.digest,
        third.manifest.lists.graph.digest,
        first.manifest.lists.graph.digest,
    ]);
});
