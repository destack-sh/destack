import { testSimulation } from "@destack/test/world";
import { join } from "node:path";
import type { Environment } from "@destack/environment";
import { expect, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { Tarball, TarballEntry } from "./tarball.ts";

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
    await using simulation = testSimulation();
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
                        yield TarballEntry.of(path, new TextEncoder().encode(text));
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

test("read back every file a tarball holds in order, with long paths and multi-block contents, and refuse a corrupted header", async () => {
    await using simulation = testSimulation();
    const { compression } = simulation.environment("test");

    // archive a short path, a prefixed path, an extended path, an empty file and contents spanning blocks
    const files: { readonly path: string; readonly contents: Uint8Array<ArrayBuffer> }[] = [
        { path: "manifest.json", contents: new TextEncoder().encode("{}") },
        { path: `${"nested/".repeat(20)}file.txt`, contents: new TextEncoder().encode("prefixed") },
        { path: `long/${"x".repeat(200)}.txt`, contents: new TextEncoder().encode("extended") },
        { path: "empty.txt", contents: new Uint8Array(0) },
        { path: "blocks.bin", contents: new Uint8Array(1300).map((_, index) => index % 251) },
    ];
    const archived = await read(
        Tarball.stream(
            (async function* () {
                for (const file of files) {
                    yield TarballEntry.of(file.path, file.contents);
                }
            })(),
            compression,
        ),
    );

    // read the written back, then corrupt the first header's name and read again
    const restored: { readonly path: string; readonly contents: Uint8Array<ArrayBuffer> }[] = [];
    for await (const entry of Tarball.read(new Blob([archived]).stream(), compression)) {
        restored.push({ path: entry.path, contents: await TarballEntry.bytes(entry) });
    }
    const decompressed = await read(
        new Blob([archived]).stream().pipeThrough(compression.decompress("gzip")),
    );
    decompressed[0] = "M".charCodeAt(0) + 1;
    const corrupted = new Blob([decompressed]).stream().pipeThrough(compression.compress("gzip"));
    const refused = await Array.fromAsync(Tarball.read(corrupted, compression)).then(
        () => "read",
        (error: unknown) => (error instanceof Error ? error.message : String(error)),
    );
    expect([restored, refused]).toEqual([files, "a tarball header's checksum fails"]);
});

test("write equal entries to identical tarball bytes with a fixed gzip header, a checked trailer and a pinned digest", async () => {
    // stream the same entries twice in a world, and once through the host's compression streams
    await using simulation = testSimulation();
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
    yield TarballEntry.of("package/package.json", new TextEncoder().encode('{"name":"a"}'));
    yield TarballEntry.of("package/build/index.js", new TextEncoder().encode("export {};\n"));
}
