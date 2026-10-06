import { openAsBlob } from "node:fs";
import { mkdir, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { dirname, join } from "node:path";
import type { PackageFile } from "@destack/package/file";
import { type BuildContents, BuildWriter, type PackageManifest } from "@destack/package/manifest";
import { BuildError } from "../error/index.ts";

/** The directory a build writes into: its sources retained until written, then every file and the build format through its writer. */
export class BuildDirectory {
    /** The build directory. */
    readonly directory: string;
    /** The writer of the build's format into the directory. */
    readonly #writer: BuildWriter;
    /** The source files retained before writing, by package path. */
    readonly #retained = new Map<string, Uint8Array<ArrayBuffer>>();

    /** Collect the files of a build directory. */
    constructor(directory: string) {
        this.directory = directory;
        this.#writer = new BuildWriter({
            write: async (path, bytes) => {
                const destination = join(directory, path);
                await mkdir(dirname(destination), { recursive: true });
                await writeFile(destination, bytes, { flag: "wx" });
            },
        });
    }

    /** The source files retained before writing, by package path. */
    get retained(): ReadonlyMap<string, Uint8Array<ArrayBuffer>> {
        return this.#retained;
    }

    /** Retain a source file, refusing different bytes at the same path. */
    retain(path: string, bytes: Uint8Array<ArrayBuffer>): void {
        // refuse different bytes at a retained path
        const previous = this.#retained.get(path);
        const isEqual =
            previous?.length === bytes.length &&
            previous.every((value, index) => value === bytes[index]);
        if (previous && !isEqual) {
            throw new BuildError("BUILD_FAILED", `conflicting build file: ${path}`);
        }
        this.#retained.set(path, bytes);
    }

    /** Write a file once, accepting a second write of equal bytes. */
    async write(path: string, bytes: Uint8Array<ArrayBuffer>): Promise<void> {
        await this.#writer.write(path, bytes);
    }

    /** Record a file a compiler wrote, hashing it without reading it whole into memory. */
    async record(path: string): Promise<void> {
        const { digest, size } = await digestFile(join(this.directory, path));
        this.#writer.add({ path, digest, size, mediaType: BuildWriter.mediaType(path) });
    }

    /** Read the description of a written file. */
    file(path: string): PackageFile {
        return this.#writer.file(path);
    }

    /** Write every retained source, then the build's graph, lists, upgrade and manifest. */
    async finish(contents: BuildContents): Promise<PackageManifest> {
        for (const [path, bytes] of this.#retained) {
            await this.#writer.write(path, bytes);
        }
        this.#retained.clear();

        return await this.#writer.finish(contents);
    }
}

/** Digest and measure a file without reading it whole into memory. */
export async function digestFile(path: string): Promise<{ digest: string; size: number }> {
    // hash the file's chunks as they stream
    const hash = createHash("sha256");
    let size = 0;
    const file = await openAsBlob(path);
    for await (const bytes of file.stream()) {
        hash.update(bytes);
        size += bytes.length;
    }

    return { digest: hash.digest("hex"), size };
}
