import { createReadStream } from "node:fs";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { PackagePath } from "../file/file.ts";
import { PackageManifest } from "../manifest/manifest.ts";
import { BuildReader, MANIFEST_PATH } from "../manifest/reader.ts";

/** Read the build a directory keeps: its manifest at the root and its files below. */
export async function readBuild(directory: string): Promise<BuildReader> {
    const bytes = await readFile(resolve(directory, MANIFEST_PATH));
    const manifest = PackageManifest.parse(
        JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)),
    );

    return new BuildReader(manifest, async (path, signal) =>
        streamFile(resolve(directory, PackagePath.parse(path)), signal),
    );
}

/** Stream a file's bytes, stopping once a signal aborts. */
function streamFile(file: string, signal: AbortSignal | undefined): ReadableStream<Uint8Array> {
    const chunks = createReadStream(file, signal === undefined ? {} : { signal })[
        Symbol.asyncIterator
    ]();

    return new ReadableStream<Uint8Array>({
        pull: async (controller) => {
            // close at the file's end
            const next = await chunks.next();
            if (next.done === true) {
                controller.close();
            }
            // pass each chunk on
            else if (next.value instanceof Uint8Array) {
                controller.enqueue(next.value);
            }
            // refuse a stream of anything but bytes
            else {
                throw new TypeError(`file stream of ${file} yielded no bytes`);
            }
        },
        cancel: async () => {
            await chunks.return?.();
        },
    });
}
