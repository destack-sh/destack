import { testSimulation } from "@destack/test/world";
import { expect, test } from "@destack/test";
import { found } from "@destack/schema";
import { PackageVersion } from "../definition/package.ts";
import { PackageFile } from "../file/file.ts";
import { BuildReader } from "./reader.ts";
import { BuildWriter } from "./writer.ts";

/** The package whose build the writer writes. */
const app = PackageVersion.parse({
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
    const manifest = await writer.finish({ destack: "2026.11.0", package: app, role: "app" });
    const reader = new BuildReader(
        manifest,
        (path) => Promise.resolve(new Blob([stored.get(path) ?? new Uint8Array()]).stream()),
        crypto,
    );
    const listed = await reader.files();

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

test("write one manifest for one set of requirements listed in any order", async () => {
    // finish the same build requiring two packages
    await using simulation = testSimulation();
    const { crypto } = simulation.environment("test");
    const finish = (requires: Record<string, string>) =>
        new BuildWriter({ write: () => Promise.resolve() }, crypto).finish({
            destack: "2026.11.0",
            package: app,
            role: "app",
            requires,
        });
    const forward = await finish({ "@example/schema": "^2026.9.0", zod: "^4.6.0" });
    const backward = await finish({ zod: "^4.6.0", "@example/schema": "^2026.9.0" });

    // write one manifest
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
        destack: "2026.11.0",
        package: app,
        role: "app",
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
    };
}

/** List files' paths in order. */
function pathsOf(files: readonly PackageFile[]): string[] {
    return files.map((file) => file.path);
}

test("record each file's media type by its extension, as browsers require for module scripts, styles, images, fonts and media", async () => {
    // write one file of each served kind and one of an unknown extension
    await using simulation = testSimulation();
    const { crypto } = simulation.environment("test");
    const writer = new BuildWriter({ write: async () => undefined }, crypto);
    const paths = [
        "dist/index.mjs",
        "dist/index.cjs",
        "dist/view.js",
        "dist/view.css",
        "public/logo.svg",
        "public/icon.png",
        "public/font.woff2",
        "public/site.webmanifest",
        "public/intro.mp4",
        "public/chime.mp3",
        "public/captions.vtt",
        "public/model.glb",
        "public/report.pdf",
        "public/data.bin",
    ];
    const bytes = new TextEncoder().encode("x");
    const written = await Promise.all(paths.map((path) => writer.write(path, bytes)));

    // expect JavaScript for every module kind, each asset's IANA type, and an octet stream otherwise
    expect(written.map((file) => [file.path, file.mediaType])).toEqual([
        ["dist/index.mjs", "text/javascript"],
        ["dist/index.cjs", "text/javascript"],
        ["dist/view.js", "text/javascript"],
        ["dist/view.css", "text/css"],
        ["public/logo.svg", "image/svg+xml"],
        ["public/icon.png", "image/png"],
        ["public/font.woff2", "font/woff2"],
        ["public/site.webmanifest", "application/manifest+json"],
        ["public/intro.mp4", "video/mp4"],
        ["public/chime.mp3", "audio/mpeg"],
        ["public/captions.vtt", "text/vtt"],
        ["public/model.glb", "model/gltf-binary"],
        ["public/report.pdf", "application/pdf"],
        ["public/data.bin", "application/octet-stream"],
    ]);
});
