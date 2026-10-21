import { expect, onTestFinished, test } from "@destack/test";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { Package } from "../definition/index.ts";
import { BuildWriter } from "../manifest/writer.ts";
import { readBuild } from "./build.ts";

test("read a build a directory keeps: its manifest, its file list and each file's bytes", async () => {
    // write a build with one file into a directory
    const directory = await mkdtemp(join(tmpdir(), "destack-read-build-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    const writer = new BuildWriter({
        write: async (path, bytes) => {
            await mkdir(dirname(join(directory, path)), { recursive: true });
            await writeFile(join(directory, path), bytes);
        },
    });
    await writer.write("output/bun/index.js", new TextEncoder().encode("export {};\n"));
    const written = await writer.finish({
        package: Package.parse({
            id: "package-01996ab0-0000-7000-8000-0000000000a1",
            name: "@example/notes",
            version: "2026.9.0",
        }),
    });

    // read it back through the directory
    const build = await readBuild(directory);
    expect({
        manifest: build.manifest,
        files: (await build.files()).map((file) => file.path),
        bytes: new TextDecoder().decode(await build.load("output/bun/index.js")),
    }).toEqual({ manifest: written, files: ["output/bun/index.js"], bytes: "export {};\n" });
});
