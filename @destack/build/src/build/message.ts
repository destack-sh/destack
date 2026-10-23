import { graph } from "@destack/package";
import { PackageInspection } from "@destack/package/code";
import { deserialize, serialize } from "node:v8";
import type { Writable } from "node:stream";
import { DependencyResolution, PackageName } from "@destack/package";
import { OutputRequest } from "@destack/package/build";
import { Runtime } from "@destack/package/runtime";
import { History } from "@destack/resource";
import { Commit, Digest, schema, Version } from "@destack/schema";
import { Failure } from "@destack/error";
import { BuildError } from "../error/index.ts";
import { CachedOutput, OutputKeys } from "../cache/entry.ts";
import { ModuleOptions } from "./build.ts";

/** Completed compiler work. */
export const BuildResult = schema.discriminatedUnion("kind", [
    schema.object({
        kind: schema.literal("plan"),
        /** The build's cache keys. */
        keys: OutputKeys,
    }),
    schema.object({
        kind: schema.literal("build"),
        /** The requested outputs the build compiled, to cache under their keys. */
        outputs: schema.record(schema.string(), CachedOutput),
    }),
    schema.object({
        kind: schema.literal("inspect"),
        /** The inspected package. */
        inspection: PackageInspection,
    }),
    schema.object({
        kind: schema.literal("index"),
        /** The code of each module of the indexed package. */
        code: schema.array(graph.Document),
    }),
    schema.object({
        kind: schema.literal("convert"),
        /** The installed packages the converted modules import. */
        imported: schema.array(schema.string()),
    }),
]);
/** Completed compiler work. */
export type BuildResult = schema.Infer<typeof BuildResult>;

/** A compiler result transferred directly to its caller. */
export const BuildResponse = schema.union([
    schema.object({
        kind: schema.literal("result"),
        /** The requested compilation or inspection. */
        result: BuildResult,
        /** No compilation failure. */
        error: schema.never().exactOptional(),
    }),
    schema.object({
        kind: schema.literal("result"),
        /** The compilation failure, keeping its call error. */
        error: schema.object({
            failure: Failure,
            cause: schema.unknown().exactOptional(),
            stack: schema.string().exactOptional(),
        }),
    }),
]);
/** A compiler result transferred directly to its caller. */
export type BuildResponse = schema.Infer<typeof BuildResponse>;

/** The most bytes of one compiler message, bounding what package code makes the host allocate. */
const MAX_MESSAGE_BYTES = 256 * 1024 * 1024;

/** The options of a build of one source package. */
export const RequestedBuild = schema.object({
    /** The source package directory. */
    directory: schema.string(),
    /** Named module outputs, and outputs of kinds the dependency closure's extensions compile. */
    outputs: schema
        .record(schema.string(), schema.union([ModuleOptions, OutputRequest]))
        .readonly(),
    /** Exact dependency releases selected by the package resolver. */
    dependencies: schema.record(schema.string(), DependencyResolution).readonly(),
    /** The TypeScript configuration, absent for the package defaults. */
    configuration: schema.string().exactOptional(),
    /** What the package has published, to plan the upgrade from. */
    history: History.exactOptional(),
    /** The commit of the source directory, absent for a working tree with changes. */
    commit: Commit.exactOptional(),
    /** The versions the build releases its source packages at, by package name. */
    versions: schema.record(PackageName, Version).readonly().exactOptional(),
    /** The directory the host materialized each Destack dependency's build in, by package name. */
    tree: schema.record(PackageName, schema.string()).readonly(),
});

/** The options of a build of one source package. */
export type RequestedBuild = schema.Infer<typeof RequestedBuild>;

/** Work accepted by the isolated compiler. */
export const BuildRequest = schema.discriminatedUnion("kind", [
    schema.object({
        kind: schema.literal("plan"),
        /** The build's options. */
        options: RequestedBuild,
    }),
    schema.object({
        kind: schema.literal("build"),
        /** The directory the build writes into, with the files of the reused outputs. */
        destination: schema.string(),
        /** The build's options. */
        options: RequestedBuild,
        /** The requested outputs to reuse instead of compiling them, by name. */
        reuse: schema.record(schema.string(), CachedOutput),
    }),
    schema.object({
        kind: schema.literal("convert"),
        /** The installed copy of the npm package. */
        source: schema.string(),
        /** The directory the converted copy is written into. */
        destination: schema.string(),
        /** The package's name. */
        name: schema.string(),
        /** The installed packages it requires. */
        dependencies: schema.array(schema.string()),
        /** The peers it declares. */
        peers: schema.array(schema.string()),
    }),
    schema.object({
        kind: schema.enum(["inspect", "index"]),
        /** The inspection's options. */
        options: schema.object({
            directory: RequestedBuild.shape.directory,
            runtime: Runtime,
            configuration: schema.string().exactOptional(),
            tree: RequestedBuild.shape.tree,
        }),
    }),
]);
/** Work accepted by the isolated compiler. */
export type BuildRequest = schema.Infer<typeof BuildRequest>;

/** A transform's cache access the compiler asks its host for while it builds. */
export const CacheRequest = schema.discriminatedUnion("kind", [
    schema.object({
        /** Read the bytes stored under a key. */
        kind: schema.literal("read"),
        /** The request's number, which the answer repeats. */
        id: schema.number().int(),
        /** The key. */
        key: Digest,
    }),
    schema.object({
        /** Store bytes under a key. */
        kind: schema.literal("write"),
        /** The request's number, which the answer repeats. */
        id: schema.number().int(),
        /** The key. */
        key: Digest,
        /** The bytes. */
        bytes: schema.instanceof(Uint8Array<ArrayBuffer>),
    }),
]);
/** A transform's cache access the compiler asks its host for while it builds. */
export type CacheRequest = schema.Infer<typeof CacheRequest>;

/** The host's answer to a cache request. */
export const CacheResponse = schema.object({
    kind: schema.literal("cached"),
    /** The number of the request it answers. */
    id: schema.number().int(),
    /** The bytes a read found, null on a miss and for a write. */
    bytes: schema.instanceof(Uint8Array<ArrayBuffer>).nullable(),
});
/** The host's answer to a cache request. */
export type CacheResponse = schema.Infer<typeof CacheResponse>;

/** What the host sends the compiler: work, or the answer to a cache request. */
export const HostMessage = schema.union([BuildRequest, CacheResponse]);

/** What the compiler sends its host: a result, or a cache request. */
export const CompilerMessage = schema.union([BuildResponse, CacheRequest]);

/** Read and parse length-prefixed messages from writeMessage without copying chunks. */
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
                    throw BuildError.of("COMPILER_FAILED", "empty compiler message");
                } else if (size > MAX_MESSAGE_BYTES) {
                    throw BuildError.of(
                        "COMPILER_FAILED",
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
        throw BuildError.of("COMPILER_FAILED", "incomplete compiler message");
    }
}

/** Write a compiler message, preserving byte arrays and regular expressions. */
export async function writeMessage(
    stream: Writable,
    value: BuildRequest | BuildResponse | CacheRequest | CacheResponse,
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
