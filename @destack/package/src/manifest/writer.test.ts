import { testSimulation } from "@destack/test/world";
import { expect, test } from "@destack/test";
import { Digest, found, schema } from "@destack/schema";
import { Package } from "../definition/package.ts";
import { PackageFile } from "../file/file.ts";
import { BuildReader } from "./reader.ts";
import { BuildWriter } from "./writer.ts";

/** The package whose build the writer writes. */
const app = Package.parse({
    id: "package-01996ab0-0000-7000-8000-000000000001",
    name: "@example/notes",
    version: "2026.9.0",
});

test("write a file's equal bytes once, a concurrent writer waiting for them, and forget a failed write", async () => {
    // keep bytes in memory only once released, failing a path marked to fail
    await using simulation = testSimulation();
    const { crypto } = simulation.environment("test");
    const stored = new Map<string, Uint8Array<ArrayBuffer>>();
    const release = Promise.withResolvers<void>();
    const writer = new BuildWriter(
        {
            write: async (path, bytes) => {
                await release.promise;
                if (path === "src/failing.js") {
                    throw new TypeError("disk full");
                }
                stored.set(path, bytes);
            },
        },
        crypto,
    );
    const bytes = new TextEncoder().encode("export {};\n");

    // write one path twice at once, noting when each write answers, and a failing path
    const answered: string[] = [];
    const first = writer.write("src/notes.js", bytes).then(() => answered.push("first"));
    const second = writer.write("src/notes.js", bytes).then(() => answered.push("second"));
    const failing = writer.write("src/failing.js", bytes).catch((error: unknown) => String(error));
    await Promise.resolve();
    const before = [...answered];
    release.resolve();
    await Promise.all([first, second]);
    const failure = await failing;
    const manifest = await writer.finish({ package: app });
    const reader = new BuildReader(
        manifest,
        (path) => Promise.resolve(new Blob([stored.get(path) ?? new Uint8Array()]).stream()),
        crypto,
    );
    const listed = await reader.read(manifest.lists.files, schema.array(PackageFile));

    // answer neither before the bytes are kept, keep them once, and list no failed file
    expect({
        before,
        answered: answered.toSorted(),
        failure,
        source: [...stored.keys()].filter((path) => path.startsWith("src/")),
        listed: listed.map((file) => file.path),
    }).toEqual({
        before: [],
        answered: ["first", "second"],
        failure: "TypeError: disk full",
        source: ["src/notes.js"],
        listed: ["src/notes.js"],
    });
});

test("write one manifest for one closure listed in any order", async () => {
    // write the same two dependencies in both orders
    await using simulation = testSimulation();
    const { crypto } = simulation.environment("test");
    const schemaRelease = {
        package: Package.parse({
            ...app,
            name: "@example/schema",
            id: "package-01996ab0-0000-7000-8000-000000000002",
        }),
        manifest: Digest.parse("1".repeat(64)),
        kind: "destack" as const,
    };
    const dbRelease = {
        package: Package.parse({
            ...app,
            name: "@example/db",
            id: "package-01996ab0-0000-7000-8000-000000000003",
        }),
        manifest: Digest.parse("2".repeat(64)),
        kind: "destack" as const,
    };
    const finish = (dependencies: Record<string, typeof schemaRelease>) =>
        new BuildWriter({ write: () => Promise.resolve() }, crypto).finish({
            package: app,
            dependencies,
        });
    const forward = await finish({
        "@example/schema@2026.9.0": schemaRelease,
        "@example/db@2026.9.0": dbRelease,
    });
    const backward = await finish({
        "@example/db@2026.9.0": dbRelease,
        "@example/schema@2026.9.0": schemaRelease,
    });

    // digest the dependency list and the manifest alike, as a store keys builds by them
    expect(backward).toEqual(forward);
});

test("list each output's loaded files: its own but maps and types, the package.json and the catalogs", async () => {
    // write a build with a Bun and a browser output beside its sources, a catalog and a readme
    await using simulation = testSimulation();
    const { crypto } = simulation.environment("test");
    const stored = new Map<string, Uint8Array<ArrayBuffer>>();
    const writer = new BuildWriter(
        { write: async (path, bytes) => void stored.set(path, bytes) },
        crypto,
    );
    const paths = [
        "package.json",
        "README.md",
        "src/index.ts",
        "locale/de.json",
        "bun/index.js",
        "bun/index.js.map",
        "bun/index.d.ts",
        "browser/index.js",
    ];
    for (const path of paths) {
        await writer.write(path, new TextEncoder().encode(path));
    }
    const manifest = await writer.finish({
        package: app,
        outputs: { bun: output("bun"), browser: output("browser") },
    });

    // read each output's list, and the maps the Bun output carries beside it
    const reader = new BuildReader(
        manifest,
        async (path) => new Blob([found(stored, path)]).stream(),
        crypto,
    );
    expect({
        bun: pathsOf(await reader.loaded("bun")),
        browser: pathsOf(await reader.loaded("browser")),
        maps: pathsOf(await reader.maps("bun")),
    }).toEqual({
        bun: ["bun/index.js", "locale/de.json", "package.json"],
        browser: ["browser/index.js", "locale/de.json", "package.json"],
        maps: ["bun/index.js.map"],
    });
});

/** Describe an output of a runtime at its directory, exporting and depending on nothing. */
function output(runtime: "bun" | "browser") {
    return {
        runtime,
        emit: true,
        workloads: {},
        views: {},
        directory: runtime,
        exports: {},
        dependencies: {},
    };
}

/** List files' paths in order. */
function pathsOf(files: readonly PackageFile[]): string[] {
    return files.map((file) => file.path);
}
