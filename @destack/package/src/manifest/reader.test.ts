import { dirname, join } from "node:path";
import type { Crypto } from "@destack/environment";
import { Simulation } from "@destack/environment/simulation";
import { expect, refusal, test } from "@destack/test";
import { Digest } from "@destack/schema";
import { Package } from "../definition/package.ts";
import { PackageError } from "../error/error.ts";
import * as graph from "../graph/index.ts";
import { MemoryBuild } from "../test/memory.ts";
import { BuildReader } from "./reader.ts";
import { BuildWriter } from "./writer.ts";

/** The start of every test's world: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

/** The package whose builds the reader opens. */
const app = Package.parse({
    id: "package-019f5530-8000-7000-8000-000000000002",
    name: "@example/app",
    version: "2026.9.0",
});

/** Describe an empty module's graph file. */
async function module(
    path: string,
    source: Uint8Array<ArrayBuffer>,
    crypto: Crypto,
): Promise<graph.Module> {
    return {
        path,
        digest: await Digest.of(source, crypto),
        imports: [],
        exports: [],
        declarations: [],
        edges: [],
    };
}

test("refuse a module's graph file whose bytes differ from its digest", async () => {
    // build a module, then serve another module's graph file under every graph path
    await using simulation = new Simulation({ seed: 1, start: START });
    const { crypto } = simulation.environment("test");
    const source = new TextEncoder().encode("export {};\n");
    const build = await MemoryBuild.write(
        new Map([["src/notes.ts", source]]),
        {
            package: app,
            graph: [await module("src/notes.ts", source, crypto)],
        },
        crypto,
    );
    const other = await graph.Module.file(await module("src/other.ts", source, crypto), crypto);
    const tampered = new BuildReader(
        build.manifest,
        (path) =>
            path.startsWith("graph/")
                ? Promise.resolve(new Blob([other.bytes]).stream())
                : build.stream(path),
        crypto,
    );

    const [digest] = Object.values((await build.graph()).modules);
    expect(await refusal(tampered.modules())).toEqual([
        "BAD_REQUEST",
        "INVALID_FILE",
        `file digest mismatch: graph/${digest}.json`,
    ]);
});

test("refuse a distributed file whose bytes differ from its listing, and an unlisted path", async () => {
    // serve a build's file as other bytes of its length, then as longer bytes
    await using simulation = new Simulation({ seed: 1, start: START });
    const { crypto } = simulation.environment("test");
    const source = new TextEncoder().encode("export {};\n");
    const build = await MemoryBuild.write(
        new Map([["src/notes.js", source]]),
        { package: app },
        crypto,
    );
    const serving = (bytes: Uint8Array<ArrayBuffer>) =>
        new BuildReader(
            build.manifest,
            (path) =>
                path === "src/notes.js"
                    ? Promise.resolve(new Blob([bytes]).stream())
                    : build.stream(path),
            crypto,
        );
    const changed = new TextEncoder().encode("export {}!\n");
    const longer = new TextEncoder().encode("export {};\nexport {};\n");

    // refuse each, and a path the build does not list
    expect([
        await refusal(serving(changed).load("src/notes.js")),
        await refusal(serving(longer).load("src/notes.js")),
        await refusal(build.load("src/missing.js")),
    ]).toEqual([
        ["BAD_REQUEST", "INVALID_FILE", "file digest mismatch: src/notes.js"],
        ["BAD_REQUEST", "INVALID_FILE", "file size mismatch: src/notes.js"],
        ["BAD_REQUEST", "INVALID_FILE", "package file not listed: src/missing.js"],
    ]);
});

test("open a remote build by its manifest digest, and refuse other bytes and unsafe addresses", async () => {
    // serve a build's manifest and its distributed files over a fake endpoint
    await using simulation = new Simulation({ seed: 1, start: START });
    const { crypto } = simulation.environment("test");
    const content = new TextEncoder().encode("export {};");
    const build = await MemoryBuild.write(
        new Map([["src/index.js", content]]),
        { package: app },
        crypto,
    );
    const bytes = new Uint8Array(
        await new Response(await build.stream("manifest.json")).arrayBuffer(),
    );
    const files = await Promise.all(
        (await build.distributed()).map(
            async (file) =>
                [
                    `https://registry.example/build/files/${file.path}`,
                    await build.load(file.path),
                ] as const,
        ),
    );
    const served = new Map([["https://registry.example/build/manifest.json", bytes], ...files]);
    const fetch = async (input: URL) => {
        const body = served.get(input.href);

        return body === undefined ? new Response(null, { status: 404 }) : new Response(body);
    };
    const digest = await Digest.of(bytes, crypto);

    // read the manifest and a file below the base, which gains its trailing slash
    const reader = await BuildReader.open(
        { manifest: digest, url: "https://registry.example/build" },
        fetch,
        crypto,
    );
    expect([reader.manifest, await reader.load("src/index.js")]).toEqual([build.manifest, content]);

    // refuse a manifest of other bytes, an address with credentials and a failed read
    expect(
        await refusal(
            BuildReader.open(
                {
                    manifest: await Digest.of(content, crypto),
                    url: "https://registry.example/build/",
                },
                fetch,
                crypto,
            ),
        ),
    ).toEqual(["BAD_REQUEST", "INVALID_FILE", "file digest mismatch: manifest.json"]);
    expect(
        await refusal(
            BuildReader.open(
                { manifest: digest, url: "https://user:secret@registry.example/build/" },
                fetch,
                crypto,
            ),
        ),
    ).toEqual(["BAD_REQUEST", "INVALID_FILE", "invalid package URL"]);
    expect(
        await refusal(
            BuildReader.open(
                { manifest: digest, url: "https://registry.example/missing/" },
                fetch,
                crypto,
            ),
        ),
    ).toEqual(["BAD_REQUEST", "INVALID_FILE", "package read failed: HTTP 404"]);
});

test("list the Destack releases a build resolved, leaving its npm dependencies out", async () => {
    // build the app against a Destack release and an npm package
    await using simulation = new Simulation({ seed: 1, start: START });
    const { crypto } = simulation.environment("test");
    const vault = {
        kind: "destack" as const,
        package: Package.parse({
            id: "package-019f5530-8000-7000-8000-000000000003",
            name: "@example/vault",
            version: "2026.9.0",
        }),
        manifest: Digest.parse("a".repeat(64)),
    };
    const jose = {
        kind: "npm" as const,
        package: { name: "jose", version: "6.2.12" },
        registry: "https://registry.npmjs.org/",
        integrity: "sha512-fixture",
        manifest: Digest.parse("b".repeat(64)),
    };
    const build = await MemoryBuild.of(app, [], crypto, {
        dependencies: { "@example/vault": vault, jose },
    });

    expect(await build.releases()).toEqual([{ package: vault.package, manifest: vault.manifest }]);
});

test("read each list and the graph's modules once, and read a list again after a failed read", async () => {
    // build an app declaring one declaration, and fail its first read of the file list
    await using simulation = new Simulation({ seed: 1, start: START });
    const { crypto } = simulation.environment("test");
    const build = await MemoryBuild.of(
        app,
        [
            {
                kind: "feature",
                kindPackageId: app.id,
                name: "export",
                description: { label: "Export" },
            },
        ],
        crypto,
    );
    const loaded: string[] = [];
    let isFailing = true;
    const reader = new BuildReader(
        build.manifest,
        async (path) => {
            loaded.push(path);
            if (path === build.manifest.lists.files.path && isFailing) {
                isFailing = false;
                throw PackageError.of("INVALID_FILE", `package read failed: ${path}`);
            }

            return await build.stream(path);
        },
        crypto,
    );
    const { lists } = build.manifest;
    expect(await refusal(reader.files())).toEqual([
        "BAD_REQUEST",
        "INVALID_FILE",
        `package read failed: ${lists.files.path}`,
    ]);

    // read every list and the declarations twice, loading each file once after the failed read
    const read = () =>
        Promise.all([
            reader.dependencies(),
            reader.files(),
            reader.sourceMaps(),
            reader.graph(),
            reader.declarations(),
        ]);
    const first = await read();
    const second = await read();
    const [digest] = Object.values(first[3].modules);
    expect(loaded.toSorted()).toEqual(
        [
            lists.files.path,
            lists.dependencies.path,
            lists.files.path,
            lists.sourceMaps.path,
            lists.graph.path,
            `graph/${digest}.json`,
        ].toSorted(),
    );

    // answer the second reads with the first reads' lists, deriving the same declarations
    const isReused = second.slice(0, 4).map((value, index) => value === first[index]);
    expect([isReused, second[4]]).toEqual([[true, true, true, true], first[4]]);
});

test("read a build a directory keeps: its manifest, its file list and each file's bytes", async () => {
    // write a build with one file into a directory
    await using simulation = new Simulation({ seed: 1, start: START });
    const world = simulation.environment("test");
    const { files } = world;
    const directory = await files.makeTemporaryDirectory("read-build-");
    const writer = new BuildWriter(
        {
            write: async (path, bytes) => {
                await files.mkdir(dirname(join(directory, path)), { recursive: true });
                await files.writeFile(join(directory, path), bytes);
            },
        },
        world.crypto,
    );
    await writer.write("output/bun/index.js", new TextEncoder().encode("export {};\n"));
    const written = await writer.finish({
        package: Package.parse({
            id: "package-01996ab0-0000-7000-8000-0000000000a1",
            name: "@example/notes",
            version: "2026.9.0",
        }),
    });

    // read it back through the directory
    const build = await BuildReader.openDirectory(directory, files, world.crypto);
    expect({
        manifest: build.manifest,
        files: (await build.files()).map((file) => file.path),
        bytes: new TextDecoder().decode(await build.load("output/bun/index.js")),
    }).toEqual({ manifest: written, files: ["output/bun/index.js"], bytes: "export {};\n" });
});
