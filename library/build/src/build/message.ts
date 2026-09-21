import { deserialize, serialize } from "node:v8";
import type { Writable } from "node:stream";
import { BuildError, type BuildErrorCode } from "../error/index.ts";
import type { PackageManifest } from "@destack/package/manifest";
import type { PackageInspection } from "@destack/package/inspect";
import type { BuildOptions } from "./build.ts";
import type { InspectOptions } from "../inspect/inspection.ts";

/** A compiler result transferred directly to its caller. */
export type BuildResponse =
    | {
          /** The requested compilation or inspection. */
          result: BuildResult;
          /** No compilation failure. */
          error?: never;
      }
    | {
          /** The compilation failure, preserving its public code. */
          error: { code: BuildErrorCode; message: string; cause?: unknown; stack?: string };
      };

/** Work accepted by the isolated compiler. */
export type BuildRequest =
    | { kind: "build"; options: Omit<BuildOptions, "directory" | "signal" | "timeout"> }
    | { kind: "inspect"; options: Omit<InspectOptions, "directory"> };

/** Completed compiler work. */
export type BuildResult =
    | {
          kind: "build";
          manifest: PackageManifest;
          files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>;
      }
    | { kind: "inspect"; inspection: PackageInspection };

/** Read length-prefixed compiler messages without copying accumulated chunks. */
export async function* readMessages(stream: AsyncIterable<Uint8Array>): AsyncGenerator<unknown> {
    let buffer = Buffer.allocUnsafe(4);
    let offset = 0;
    let isHeader = true;

    // fill each header and payload once, including messages split across stream chunks
    for await (const chunk of stream) {
        let position = 0;
        while (position < chunk.length) {
            const length = Math.min(buffer.length - offset, chunk.length - position);
            buffer.set(chunk.subarray(position, position + length), offset);
            offset += length;
            position += length;
            if (offset !== buffer.length) {
                continue;
            }

            // allocate the payload described by the completed header
            if (isHeader) {
                const size = buffer.readUInt32LE();
                if (!size) {
                    throw new BuildError("BUILD_FAILED", "Empty compiler message.");
                }
                buffer = Buffer.allocUnsafe(size);
                isHeader = false;
            } // release the complete payload before reading the next header
            else {
                yield deserialize(buffer);
                buffer = Buffer.allocUnsafe(4);
                isHeader = true;
            }
            offset = 0;
        }
    }

    if (offset || !isHeader) {
        throw new BuildError("BUILD_FAILED", "Incomplete compiler message.");
    }
}

/** Write a compiler message, preserving byte arrays and regular expressions. */
export async function writeMessage(stream: Writable, value: unknown): Promise<void> {
    const payload = serialize(value);
    const header = Buffer.allocUnsafe(4);
    header.writeUInt32LE(payload.length);
    stream.write(header);

    await new Promise<void>((resolve, reject) => {
        stream.write(payload, (error) => (error ? reject(error) : resolve()));
    });
}
