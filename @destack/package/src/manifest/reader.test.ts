import { expect, test } from "@destack/test";
import { Digest } from "@destack/schema";
import { Package } from "../definition/package.ts";
import { PackageError } from "../error/error.ts";
import * as graph from "../graph/index.ts";
import { MemoryBuild } from "../test/memory.ts";
import { BuildReader } from "./reader.ts";

/** The package whose builds the reader opens. */
const app = Package.parse({
    id: "package-019f5530-8000-7000-8000-000000000002",
    name: "@example/app",
    version: "2026.9.0",
});

/** Describe an empty module's graph file. */
async function module(path: string, source: Uint8Array<ArrayBuffer>): Promise<graph.Module> {
    return {
        path,
        digest: await Digest.of(source),
        imports: [],
        exports: [],
        symbols: [],
        declarations: [],
        edges: [],
    };
}

test("refuse a module's graph file whose bytes differ from its digest", async () => {
    // build a module, then serve another module's graph file under every graph path
    const source = new TextEncoder().encode("export {};\n");
    const build = await MemoryBuild.write(new Map([["src/notes.ts", source]]), {
        package: app,
        graph: [await module("src/notes.ts", source)],
    });
    const other = await graph.Module.file(await module("src/other.ts", source));
    const tampered = new BuildReader(build.manifest, (path) =>
        path.startsWith("graph/") ? Promise.resolve(other.bytes) : build.reader.load(path),
    );

    const [digest] = Object.values((await build.reader.graph()).modules);
    await expect(tampered.modules()).rejects.toThrow(
        new PackageError("INVALID_FILE", `file digest mismatch: graph/${digest}.json`),
    );
});

test("open a remote build by its manifest digest, and refuse other bytes and unsafe addresses", async () => {
    // serve a build's manifest and one file over a fake endpoint
    const content = new TextEncoder().encode("export {};");
    const build = await MemoryBuild.write(new Map([["src/index.js", content]]), { package: app });
    const bytes = await new Response(await build.open("manifest.json")).bytes();
    const served = new Map([
        ["https://registry.example/build/manifest.json", bytes],
        ["https://registry.example/build/files/src/index.js", content],
    ]);
    const fetch = async (input: URL) => {
        const body = served.get(input.href);

        return body === undefined ? new Response(null, { status: 404 }) : new Response(body);
    };
    const digest = await Digest.of(bytes);

    // read the manifest and a file below the base, which gains its trailing slash
    const reader = await BuildReader.open(
        { manifest: digest, url: "https://registry.example/build" },
        fetch,
    );
    expect([reader.manifest, await reader.load("src/index.js")]).toEqual([build.manifest, content]);

    // refuse a manifest of other bytes, an address with credentials and a failed read
    await expect(
        BuildReader.open(
            { manifest: await Digest.of(content), url: "https://registry.example/build/" },
            fetch,
        ),
    ).rejects.toThrow(new PackageError("INVALID_FILE", "file digest mismatch: manifest.json"));
    await expect(
        BuildReader.open(
            { manifest: digest, url: "https://user:secret@registry.example/build/" },
            fetch,
        ),
    ).rejects.toThrow(new PackageError("INVALID_FILE", "invalid package URL"));
    await expect(
        BuildReader.open({ manifest: digest, url: "https://registry.example/missing/" }, fetch),
    ).rejects.toThrow(new PackageError("INVALID_FILE", "package read failed: HTTP 404"));
});
