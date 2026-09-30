import { createReadStream } from "node:fs";
import { mkdir, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { dirname, join } from "node:path";
import { describeFile, type PackageFile } from "@destack/package/file";
import { BuildError } from "../error/index.ts";
import { comparePath } from "./serialization.ts";

/** The media types of generated files by extension (IANA), octet streams otherwise. */
const MEDIA_TYPES: Readonly<Record<string, string>> = {
    json: "application/json",
    map: "application/json",
    js: "text/javascript",
    html: "text/html",
    css: "text/css",
    wasm: "application/wasm",
};

/** The files a build distributes: sources retained until written, and every written file by digest. */
export class BuildFiles {
    /** The build directory. */
    readonly directory: string;
    /** The source files retained before writing, by package path. */
    readonly #retained = new Map<string, Uint8Array<ArrayBuffer>>();
    /** The written files, by package path. */
    readonly #records = new Map<string, PackageFile>();

    /** Collect the files of a build directory. */
    constructor(directory: string) {
        this.directory = directory;
    }

    /** The source files retained before writing, by package path. */
    get retained(): ReadonlyMap<string, Uint8Array<ArrayBuffer>> {
        return this.#retained;
    }

    /** Retain a source file, refusing different bytes at the same path. */
    retain(path: string, bytes: Uint8Array<ArrayBuffer>): void {
        const previous = this.#retained.get(path);
        if (
            previous &&
            (previous.length !== bytes.length ||
                previous.some((value, index) => value !== bytes[index]))
        ) {
            throw new BuildError("BUILD_FAILED", `conflicting build file: ${path}`);
        }
        this.#retained.set(path, bytes);
    }

    /** Write a file once, accepting a second write of equal bytes. */
    async write(path: string, bytes: Uint8Array<ArrayBuffer>): Promise<void> {
        // accept an equal file written before
        const file = await describeFile(path, mediaType(path), bytes);
        const previous = this.#records.get(path);
        if (previous) {
            if (previous.digest !== file.digest) {
                throw new BuildError("BUILD_FAILED", `conflicting build file: ${path}`);
            }

            return;
        }

        // publish the bytes before recording the write
        const destination = join(this.directory, path);
        await mkdir(dirname(destination), { recursive: true });
        await writeFile(destination, bytes, { flag: "wx" });
        this.#records.set(path, file);
    }

    /** Record a file a compiler wrote, hashing it without reading it whole into memory. */
    async record(path: string): Promise<void> {
        // hash and measure the file
        const hash = createHash("sha256");
        let size = 0;
        for await (const bytes of createReadStream(join(this.directory, path))) {
            hash.update(bytes);
            size += bytes.length;
        }
        const digest = hash.digest("hex");

        // refuse a different file at the same path
        const previous = this.#records.get(path);
        if (previous && previous.digest !== digest) {
            throw new BuildError("BUILD_FAILED", `conflicting build file: ${path}`);
        }
        this.#records.set(path, { path, digest, size, mediaType: mediaType(path) });
    }

    /** Write every retained source file. */
    async flush(): Promise<void> {
        for (const [path, bytes] of this.#retained) {
            await this.write(path, bytes);
        }
        this.#retained.clear();
    }

    /** Report whether a file was written. */
    has(path: string): boolean {
        return this.#records.has(path);
    }

    /** List the written files by path. */
    list(): PackageFile[] {
        return [...this.#records.values()].sort(comparePath);
    }
}

/** Select the distributed media type from a generated path. */
function mediaType(path: string): string {
    return MEDIA_TYPES[path.split(".").at(-1)!] ?? "application/octet-stream";
}
