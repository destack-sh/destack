import { graph } from "@destack/package";
import { PackageInspection } from "@destack/package/code";
import { deserialize, serialize } from "node:v8";
import { DependencyResolution, PackageName } from "@destack/package";
import { OutputRequest } from "@destack/package/build";
import { Runtime } from "@destack/package/runtime";
import { Commit, Digest, schema, Version } from "@destack/schema";
import { Failure } from "@destack/error";
import { BuildError } from "../error/index.ts";
import { ModuleOptions } from "./build.ts";
import { EvaluatedDeclaration } from "../declaration/evaluation.ts";

/** Completed compiler work. */
export const BuildResult = schema.discriminatedUnion("kind", [
    schema.object({
        kind: schema.literal("build"),
        /** The npm packages each output imports, which the host's link pins, by output. */
        npm: schema.record(schema.string(), schema.array(schema.string())),
    }),
    schema.object({
        kind: schema.literal("evaluate"),
        /** The package's declarations, each with the module and export it has. */
        declarations: schema.array(EvaluatedDeclaration),
    }),
    schema.object({
        /** The package's outputs passed their type and runtime checks. */
        kind: schema.literal("check"),
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
        /** The module graph: each module's exports, declarations and edges. */
        modules: schema.array(graph.Module),
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
            /** Whether the failure is for callers or for the host alone, as its family declares. */
            audience: schema.enum(["caller", "host"]),
            cause: schema.unknown().exactOptional(),
            stack: schema.string().exactOptional(),
        }),
    }),
]);
/** A compiler result transferred directly to its caller. */
export type BuildResponse = schema.Infer<typeof BuildResponse>;

/** The most bytes of one compiler message, bounding what package code makes the host allocate. */
const MAX_MESSAGE_BYTES = 256 * 1024 * 1024;

/** The bytes of a message's length header, a little-endian 32-bit integer. */
const HEADER_BYTES = 4;

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
        kind: schema.literal("build"),
        /** The directory the build writes into. */
        destination: schema.string(),
        /** The build's options. */
        options: RequestedBuild,
        /** The package's evaluated declarations. */
        declarations: schema.array(EvaluatedDeclaration),
    }),
    schema.object({
        kind: schema.literal("evaluate"),
        /** The build's options. */
        options: RequestedBuild,
    }),
    schema.object({
        /** Check the outputs' types and the runtime globals their modules use. */
        kind: schema.literal("check"),
        /** The build's options. */
        options: RequestedBuild,
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
/** What the host sends the compiler: work, or the answer to a cache request. */
export type HostMessage = schema.Infer<typeof HostMessage>;

/** What the compiler sends its host: a result, or a cache request. */
export const CompilerMessage = schema.union([BuildResponse, CacheRequest]);

/** Where compiler messages are written, such as a stream's writer or a process's output. */
export interface MessageOutput {
    /** Write a chunk, settling once the output took it. */
    readonly write: (chunk: Uint8Array) => Promise<void> | void;
}

/** Read length-prefixed messages from writeMessage without copying chunks. */
export async function* readMessages<Message>(
    stream: AsyncIterable<Uint8Array>,
    read: (value: unknown) => Message,
): AsyncGenerator<Message> {
    // read message headers and bodies from the stream
    let buffer = Buffer.allocUnsafe(HEADER_BYTES);
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
                yield read(deserialize(buffer));
                buffer = Buffer.allocUnsafe(HEADER_BYTES);
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
    output: MessageOutput,
    value: BuildRequest | BuildResponse | CacheRequest | CacheResponse,
): Promise<void> {
    // write the length header and the payload as one chunk, which concurrent writes cannot split
    const payload = serialize(value);
    const message = new Uint8Array(HEADER_BYTES + payload.length);
    new DataView(message.buffer).setUint32(0, payload.length, true);
    message.set(payload, HEADER_BYTES);
    await output.write(message);
}
