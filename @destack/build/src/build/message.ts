import { deserialize, serialize } from "node:v8";
import type { Writable } from "node:stream";
import { DependencyResolution } from "@destack/package";
import { OutputRequest } from "@destack/package/build";
import { PackageInspection } from "@destack/package/inspect";
import { PackageManifest } from "@destack/package/manifest";
import { Runtime } from "@destack/package/runtime";
import { History } from "@destack/resource";
import { Commit, schema } from "@destack/schema";
import { BuildError, BuildErrorCode } from "../error/index.ts";
import { BuildKeys, CachedOutput } from "../cache/index.ts";
import { ModuleOptions } from "./build.ts";

/** Completed compiler work. */
export const BuildResult = schema.discriminatedUnion("kind", [
    schema.object({
        kind: schema.literal("plan"),
        /** The build's cache keys. */
        keys: BuildKeys,
    }),
    schema.object({
        kind: schema.literal("build"),
        /** The built package's manifest. */
        manifest: PackageManifest,
        /** The requested outputs the build compiled, to cache under their keys. */
        outputs: schema.record(schema.string(), CachedOutput),
    }),
    schema.object({
        kind: schema.literal("inspect"),
        /** The inspected package. */
        inspection: PackageInspection,
    }),
]);
/** Completed compiler work. */
export type BuildResult = schema.Infer<typeof BuildResult>;

/** A compiler result transferred directly to its caller. */
export const BuildResponse = schema.union([
    schema.object({
        /** The requested compilation or inspection. */
        result: BuildResult,
        /** No compilation failure. */
        error: schema.never().exactOptional(),
    }),
    schema.object({
        /** The compilation failure, preserving its public code. */
        error: schema.object({
            code: BuildErrorCode,
            message: schema.string(),
            cause: schema.unknown().exactOptional(),
            stack: schema.string().exactOptional(),
        }),
    }),
]);
/** A compiler result transferred directly to its caller. */
export type BuildResponse = schema.Infer<typeof BuildResponse>;

/** The most bytes one compiler message has, bounding what a compiler running package code makes the host allocate. */
const MAX_MESSAGE_BYTES = 256 * 1024 * 1024;

/** The options of a build beside its package directory. */
export const RequestedBuild = schema.object({
    /** Named module outputs, and outputs of kinds the extensions of the package's dependency closure compile. */
    outputs: schema
        .record(schema.string(), schema.union([ModuleOptions, OutputRequest]))
        .readonly(),
    /** Exact dependency releases selected by the package resolver. */
    dependencies: schema.record(schema.string(), DependencyResolution).readonly(),
    /** The TypeScript configuration, absent for the package defaults. */
    configuration: schema.string().exactOptional(),
    /** What the package has published, to plan the upgrade from. */
    history: History.exactOptional(),
    /** The commit the source directory holds, absent for a working tree with uncommitted changes. */
    commit: Commit.exactOptional(),
});

/** The options of a build beside its package directory. */
export type RequestedBuild = schema.Infer<typeof RequestedBuild>;

/** Work accepted by the isolated compiler. */
export const BuildRequest = schema.discriminatedUnion("kind", [
    schema.object({
        kind: schema.literal("plan"),
        /** The build's options beside the package directory. */
        options: RequestedBuild,
    }),
    schema.object({
        kind: schema.literal("build"),
        /** The directory the build writes into, holding the files of the reused outputs. */
        destination: schema.string(),
        /** The build's options beside the package directory. */
        options: RequestedBuild,
        /** The requested outputs to reuse instead of compiling them, by name. */
        reuse: schema.record(schema.string(), CachedOutput),
    }),
    schema.object({
        kind: schema.literal("inspect"),
        /** The inspection's options beside the package directory. */
        options: schema.object({
            runtime: Runtime,
            configuration: schema.string().exactOptional(),
        }),
    }),
]);
/** Work accepted by the isolated compiler. */
export type BuildRequest = schema.Infer<typeof BuildRequest>;

/** Read length-prefixed compiler messages the peer wrote with writeMessage, parsing each, without copying accumulated chunks. */
export async function* readMessages<Message>(
    stream: AsyncIterable<Uint8Array>,
    message: schema.Schema<Message>,
): AsyncGenerator<Message> {
    // read message headers and bodies from the stream
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
                    throw new BuildError("BUILD_FAILED", "empty compiler message");
                } else if (size > MAX_MESSAGE_BYTES) {
                    throw new BuildError(
                        "BUILD_FAILED",
                        `compiler message of ${size} bytes exceeds ${MAX_MESSAGE_BYTES}`,
                    );
                }
                buffer = Buffer.allocUnsafe(size);
                isHeader = false;
            }
            // release the complete payload before reading the next header
            else {
                yield message.parse(deserialize(buffer));
                buffer = Buffer.allocUnsafe(4);
                isHeader = true;
            }
            offset = 0;
        }
    }

    // reject a truncated stream
    if (offset || !isHeader) {
        throw new BuildError("BUILD_FAILED", "incomplete compiler message");
    }
}

/** Write a compiler message, preserving byte arrays and regular expressions. */
export async function writeMessage(
    stream: Writable,
    value: BuildRequest | BuildResponse,
): Promise<void> {
    // write the length header
    const payload = serialize(value);
    const header = Buffer.allocUnsafe(4);
    header.writeUInt32LE(payload.length);
    stream.write(header);

    // write the payload and wait for the flush
    await new Promise<void>((resolve, reject) => {
        stream.write(payload, (error) => (error ? reject(error) : resolve()));
    });
}
