import { expect, onTestFinished, test } from "@destack/test";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Tarball, type TarballEntry } from "./tarball.ts";

/** Create a temporary directory removed when the test finishes. */
async function temporary(): Promise<string> {
    const directory = await mkdtemp(join(tmpdir(), "destack-tarball-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));

    return directory;
}

/** Read a stream's bytes. */
async function read(stream: ReadableStream<Uint8Array>): Promise<Uint8Array<ArrayBuffer>> {
    return new Uint8Array(await new Response(stream).arrayBuffer());
}

/** Extract a gzip tarball with the system tar into a directory. */
async function extract(archive: Uint8Array, directory: string): Promise<void> {
    await writeFile(join(directory, "archive.tgz"), archive);
    await mkdir(join(directory, "extracted"));
    const child = Bun.spawn(["tar", "-xzf", "archive.tgz", "-C", "extracted"], {
        cwd: directory,
        stderr: "pipe",
    });
    const [code, errors] = await Promise.all([child.exited, new Response(child.stderr).text()]);
    if (code !== 0) {
        throw new Error(`tar failed with ${code}: ${errors}`);
    }
}

test("roundtrip files of short, prefixed and extended paths through a tarball the system tar extracts", async () => {
    const directory = await temporary();

    // archive a short path, a path split into prefix and name, a path only an extended header holds, and an empty file
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
            ),
        ),
        directory,
    );

    // extract every file with its exact contents
    const extracted = Object.fromEntries(
        await Promise.all(
            Object.keys(files).map(async (path): Promise<[string, string]> => [
                path,
                await readFile(join(directory, "extracted", path), "utf8"),
            ]),
        ),
    );
    expect(extracted).toEqual(files);
});

test("write equal entries to identical tarball bytes with a fixed gzip header, a checked trailer and a pinned digest", async () => {
    // stream the same entries twice
    const first = await read(Tarball.stream(entries()));
    const second = await read(Tarball.stream(entries()));

    // name no time and an unknown system, and inflate through the runtime's gzip, which checks the CRC and size
    const inflated = await read(
        new Blob([first]).stream().pipeThrough(new DecompressionStream("gzip")),
    );
    const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", first)).toHex();
    expect([
        second,
        [...first.subarray(0, 10)],
        inflated.byteLength,
        new TextDecoder().decode(inflated.subarray(0, 20)),
        digest,
    ]).toEqual([
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
