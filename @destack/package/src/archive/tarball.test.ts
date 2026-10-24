import { join } from "node:path";
import type { Environment } from "@destack/environment";
import { Simulation } from "@destack/environment/simulation";
import { expect, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { Tarball, type TarballEntry } from "./tarball.ts";

/** The start of every test's world: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

/** Read a stream's bytes. */
async function read(stream: ReadableStream<Uint8Array>): Promise<Uint8Array<ArrayBuffer>> {
    return new Uint8Array(await new Response(stream).arrayBuffer());
}

/** Extract a gzip tarball with the system tar into a directory. */
async function extract(
    archive: Uint8Array,
    directory: string,
    environment: Environment<"files" | "process">,
): Promise<void> {
    const { files } = environment;
    await files.writeFile(join(directory, "archive.tgz"), archive);
    await files.mkdir(join(directory, "extracted"));
    const child = environment.process.spawn(["tar", "-xzf", "archive.tgz", "-C", "extracted"], {
        workingDirectory: directory,
        stderr: "pipe",
    });
    const [code, errors] = await Promise.all([child.exited, new Response(child.stderr).text()]);
    if (code !== 0) {
        throw new Error(`tar failed with ${code}: ${errors}`);
    }
}

test("roundtrip files of short, prefixed and extended paths through a tarball the system tar extracts", async () => {
    await using simulation = new Simulation({ seed: 1, start: START });
    const environment = simulation.environment("test");
    const directory = await environment.files.makeTemporaryDirectory("tarball-");

    // archive a short path, a path split into prefix and name, a path that fits only an extended header, and an empty file
    const files: Record<string, string> = {
        "manifest.json": "{}",
        [`${"nested/".repeat(20)}file.txt`]: "prefixed",
        [`long/${"x".repeat(200)}.txt`]: "extended",
        "empty.txt": "",
    };
    await extract(
        await read(
            Tarball.stream(
                (async function* () {
                    for (const [path, text] of Object.entries(files)) {
                        yield { path, contents: new TextEncoder().encode(text) };
                    }
                })(),
                environment.compression,
            ),
        ),
        directory,
        environment,
    );

    // extract every file with its exact contents
    const extracted = Object.fromEntries(
        await Promise.all(
            Object.keys(files).map(async (path): Promise<[string, string]> => [
                path,
                await environment.files.readTextFile(join(directory, "extracted", path)),
            ]),
        ),
    );
    expect(extracted).toEqual(files);
});

test("write equal entries to identical tarball bytes with a fixed gzip header, a checked trailer and a pinned digest", async () => {
    // stream the same entries twice in a world, and once through the host's compression streams
    await using simulation = new Simulation({ seed: 1, start: START });
    const { compression, crypto } = simulation.environment("test");
    const first = await read(Tarball.stream(entries(), compression));
    const second = await read(Tarball.stream(entries(), compression));
    const hosted = await read(Tarball.stream(entries(), testHost().compression));

    // set no time and an unknown system, and inflate through the world's gzip, which checks the CRC and size
    const inflated = await read(
        new Blob([first]).stream().pipeThrough(compression.decompress("gzip")),
    );
    const digest = new Uint8Array(await crypto.digest("SHA-256", first)).toHex();
    expect([
        second,
        hosted,
        [...first.subarray(0, 10)],
        inflated.byteLength,
        new TextDecoder().decode(inflated.subarray(0, 20)),
        digest,
    ]).toEqual([
        first,
        first,
        [0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff],
        512 * 6,
        "package/package.json",
        "e80c2cacdc3ba483c0c5ec3978fc9c16c6827c5d79e83e8cc560974878e427db",
    ]);
});

/** Stream a package manifest and an empty module. */
async function* entries(): AsyncGenerator<TarballEntry> {
    yield { path: "package/package.json", contents: new TextEncoder().encode('{"name":"a"}') };
    yield {
        path: "package/build/index.js",
        contents: new TextEncoder().encode("export {};\n"),
    };
}
