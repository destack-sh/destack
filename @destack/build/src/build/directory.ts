import { dirname, join } from "node:path";
import type { Environment, FileSystem } from "@destack/environment";
import type { PackageFile } from "@destack/package/file";
import { type BuildContents, BuildWriter, type PackageManifest } from "@destack/package/manifest";
import { BuildError } from "../error/index.ts";
import { Digest, Text } from "@destack/schema";

/** The directory a build writes into: its sources retained until written, then its files. */
export class BuildDirectory {
    /** The build directory. */
    readonly directory: string;
    /** The files the build reads and writes, and the crypto digesting them. */
    readonly environment: Environment<"files">;
    /** The writer of the build's format into the directory. */
    readonly #writer: BuildWriter;
    /** The source files retained before writing, by package path. */
    readonly #retained = new Map<string, Uint8Array<ArrayBuffer>>();

    /** Collect the files of a build directory. */
    constructor(directory: string, environment: Environment<"files">) {
        // write each file once into the directory, digesting it with the environment's crypto
        const { files, crypto } = environment;
        this.directory = directory;
        this.environment = environment;
        this.#writer = new BuildWriter(
            {
                write: async (path, bytes) => {
                    const destination = join(directory, path);
                    await files.mkdir(dirname(destination), { recursive: true });
                    await files.writeFile(destination, bytes, { createNew: true });
                },
            },
            crypto,
        );
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
            throw BuildError.of("INVARIANT_VIOLATED", `conflicting build file: ${path}`);
        }
        this.#retained.set(path, bytes);
    }

    /** Write a file once, accepting a second write of equal bytes. */
    async write(path: string, bytes: Uint8Array<ArrayBuffer>): Promise<void> {
        await this.#writer.write(path, bytes);
    }

    /** Record a file a compiler wrote, hashing it without reading it whole into memory. */
    async record(path: string): Promise<void> {
        const { digest, size } = await digestFile(join(this.directory, path), this.environment);
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
export async function digestFile(
    path: string,
    environment: Environment<"files">,
): Promise<{ digest: Digest; size: number }> {
    // hash the file's chunks as they stream, counting their bytes
    const tally = { size: 0 };
    await using file = await environment.files.open(path, { read: true });
    const digest = await Digest.of(tallied(file.readable(), tally), environment.crypto);

    return { digest, size: tally.size };
}

/** Pass chunks on, adding their lengths to a tally. */
async function* tallied(
    chunks: AsyncIterable<Uint8Array>,
    tally: { size: number },
): AsyncGenerator<Uint8Array> {
    for await (const chunk of chunks) {
        tally.size += chunk.length;
        yield chunk;
    }
}

/** Read regular files in deterministic order and reject symbolic links. */
export async function readDirectory(
    directory: string,
    files: FileSystem,
): Promise<Map<string, Uint8Array<ArrayBuffer>>> {
    // walk directories breadth first and collect regular files
    const contents = new Map<string, Uint8Array<ArrayBuffer>>();
    const pending = [""];
    for (const parent of pending) {
        const entries = await files.readDirectory(join(directory, parent));
        for (const entry of entries) {
            const name = parent ? `${parent}/${entry.name}` : entry.name;
            if (entry.kind === "directory") {
                pending.push(name);
            } else if (entry.kind === "file") {
                contents.set(name, await files.readFile(join(directory, name)));
            } else {
                throw BuildError.of(
                    "ASSET_INVALID",
                    `unsupported directory entry: ${name} in ${directory}`,
                );
            }
        }
    }

    return new Map([...contents].toSorted(([left], [right]) => Text.compare(left, right)));
}
