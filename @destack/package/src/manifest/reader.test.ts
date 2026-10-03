import { expect, test } from "@destack/test";
import { Digest, found, schema } from "@destack/schema";
import { PackageFile } from "../file/file.ts";
import { PackageError } from "../error/error.ts";
import { Package } from "../definition/package.ts";
import * as graph from "../graph/index.ts";
import { BuildReader } from "./reader.ts";
import type { PackageManifest } from "./manifest.ts";

/** The declaring package's first release. */
const owner = Package.parse({
    id: "package-019f5530-8000-7000-8000-000000000001",
    name: "@destack/setting",
    version: "2026.9.0",
});

/** The package whose build the reader opens. */
const app = Package.parse({
    id: "package-019f5530-8000-7000-8000-000000000002",
    name: "@example/app",
    version: "2026.9.0",
});

/** A package declaring the procedure kind. */
const service = Package.parse({
    id: "package-019f5530-8000-7000-8000-000000000003",
    name: "@destack/service",
    version: "2026.9.0",
});

/** An empty list file of a build. */
const EMPTY = await PackageFile.describe(
    "manifest/empty.json",
    "application/json",
    new Uint8Array(),
);

/** Declare a kind of a package at an export of the app's module. */
function declaration(
    declaring: Package,
    kind: string,
    module: string,
    name: string,
    member?: string,
): graph.Declaration {
    const symbol = graph.Moniker.of({ packageId: app.id, module, name });
    const derived = member === undefined ? {} : { member };

    return {
        moniker: graph.Moniker.of({ packageId: app.id, module, name, ...derived, kind }),
        symbol,
        kind,
        package: declaring.id,
        name: member ?? name,
        description: { name: member ?? name },
    };
}

/** Store a graph file of each module's declarations and the root naming them, beside an upgrade. */
async function store(modules: Readonly<Record<string, graph.Declaration[]>>) {
    // write each module's graph file by its digest
    const files = new Map<string, Uint8Array<ArrayBuffer>>();
    const root: graph.Root = { modules: {} };
    for (const [path, declarations] of Object.entries(modules)) {
        const encoded = await graph.Module.file({
            path,
            digest: await Digest.of(new TextEncoder().encode(path)),
            imports: [],
            exports: [],
            symbols: [],
            declarations,
            edges: [],
        });
        files.set(`graph/${encoded.digest}.json`, encoded.bytes);
        root.modules[path] = encoded.digest;
    }

    // describe the root and the upgrade
    const file = async (path: string, value: unknown) => {
        const bytes = new TextEncoder().encode(JSON.stringify(value));
        files.set(path, bytes);

        return PackageFile.describe(path, "application/json", bytes);
    };
    const manifest: PackageManifest = {
        formatVersion: 1,
        package: app,
        language: "typescript",
        lists: {
            dependencies: EMPTY,
            files: EMPTY,
            sourceMaps: EMPTY,
            graph: await file("manifest/graph.json", root),
        },
        outputs: {},
        upgrade: {
            package: owner,
            file: await file("manifest/upgrade.json", { from: "2026.8.0", steps: [] }),
        },
    };

    return new BuildReader(manifest, async (path) => found(files, path));
}

test("read the declarations of one kind a package declares across the graph's modules", async () => {
    // declare settings in two modules beside a schedule, a service and the procedure it derives
    const reader = await store({
        "src/index.ts": [
            declaration(owner, "setting", "src/index.ts", "language"),
            declaration(owner, "schedule", "src/index.ts", "nightly"),
        ],
        "src/notes.ts": [
            declaration(owner, "setting", "src/notes.ts", "theme"),
            declaration(service, "service", "src/notes.ts", "notes"),
            declaration(service, "procedure", "src/notes.ts", "notes", "list"),
        ],
    });

    // read the settings of both modules, the service without its procedure, and nothing of another package
    const item = schema.object({ name: schema.string() }).strict();
    const read = async (id: typeof owner.id, kind: string) =>
        (await reader.declared(id, kind, item)).map((declared) => declared.description);
    expect([
        await read(owner.id, "setting"),
        await read(service.id, "service"),
        await read(service.id, "procedure"),
        await read(app.id, "setting"),
    ]).toEqual([[{ name: "language" }, { name: "theme" }], [{ name: "notes" }], [], []]);
});

test("list a build's declarations without derived members, and reference its graph root and upgrade", async () => {
    // declare a service and its procedure
    const notes = declaration(service, "service", "src/notes.ts", "notes");
    const reader = await store({
        "src/notes.ts": [notes, declaration(service, "procedure", "src/notes.ts", "notes", "list")],
    });

    // keep the service alone, and reference the root before the upgrade
    expect([await reader.declarations(), reader.references().map((file) => file.path)]).toEqual([
        [notes],
        [
            "manifest/empty.json",
            "manifest/empty.json",
            "manifest/empty.json",
            "manifest/graph.json",
            "manifest/upgrade.json",
        ],
    ]);
});

test("refuse a module's graph file whose bytes differ from its digest", async () => {
    // serve another module's bytes under each graph file's digest
    const reader = await store({ "src/notes.ts": [] });
    const other = await graph.Module.file({
        path: "src/other.ts",
        digest: await Digest.of(new TextEncoder().encode("src/other.ts")),
        imports: [],
        exports: [],
        symbols: [],
        declarations: [],
        edges: [],
    });
    const tampered = new BuildReader(reader.manifest, (path) =>
        path.startsWith("graph/") ? Promise.resolve(other.bytes) : reader.load(path),
    );

    const [digest] = Object.values((await reader.graph()).modules);
    await expect(tampered.modules()).rejects.toThrow(
        new PackageError("INVALID_FILE", `file digest mismatch: graph/${digest}.json`),
    );
});

test("open a remote build by its manifest digest, and refuse other bytes and unsafe addresses", async () => {
    // serve a build's manifest and one file over a fake endpoint
    const content = new TextEncoder().encode("export {};");
    const manifest: PackageManifest = {
        formatVersion: 1,
        package: owner,
        language: "typescript",
        lists: { dependencies: EMPTY, files: EMPTY, sourceMaps: EMPTY, graph: EMPTY },
        outputs: {},
    };
    const bytes = new TextEncoder().encode(JSON.stringify(manifest));
    const served = new Map([
        ["https://registry.example/build/manifest.json", bytes],
        ["https://registry.example/build/files/src/index.js", content],
    ]);
    const fetch = async (input: URL) => {
        const body = served.get(input.href);

        return body === undefined ? new Response(null, { status: 404 }) : new Response(body);
    };
    const digest = (await PackageFile.describe("manifest.json", "application/json", bytes)).digest;

    // read the manifest and a file below the base, which gains its trailing slash
    const reader = await BuildReader.open(
        { manifest: digest, url: "https://registry.example/build" },
        fetch,
    );
    expect(reader.manifest).toEqual(manifest);
    expect(await reader.load("src/index.js")).toEqual(content);

    // refuse a manifest of other bytes, an address with credentials and a failed read
    const forged = (await PackageFile.describe("manifest.json", "application/json", content))
        .digest;
    await expect(
        BuildReader.open({ manifest: forged, url: "https://registry.example/build/" }, fetch),
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
