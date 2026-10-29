import { testSimulation } from "@destack/test/world";
import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
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
