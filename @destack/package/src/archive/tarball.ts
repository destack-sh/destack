import type { Compression, CompressionPair } from "@destack/environment";
import { PackageError } from "../error/index.ts";

/** The size of a tar block, with a header or a slice of a file. */
const BLOCK_BYTES = 512;

/** The longest file name that fits a ustar header's name field. */
const NAME_BYTES = 100;

/** The longest directory prefix that fits a ustar header's prefix field. */
const PREFIX_BYTES = 155;

/** The permissions of every archived file: readable by all, written by the owner. */
const FILE_MODE = 0o644;

/** The gzip member header: deflate, no flags, time or extra flags, unknown system (RFC 1952). */
const GZIP_HEADER = new Uint8Array([0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff]);

/** The offset of the gzip header's flags, which announce optional fields (RFC 1952). */
const FLAGS_OFFSET = 3;

/** One regular file of a tarball: its path, its size, and its bytes streamed in order. */
export interface TarballEntry {
    /** The path within the archive. */
    readonly path: string;
    /** The file's size in bytes, which its header records before its bytes. */
    readonly size: number;
    /** The file's bytes, read in order before the next entry's. */
    readonly body: AsyncIterable<Uint8Array>;
}

/** Make and read tarball entries. */
export const TarballEntry = {
    /** Make an entry of a file's whole bytes. */
    of(path: string, contents: Uint8Array): TarballEntry {
        return {
            path,
            size: contents.byteLength,
            body: (async function* () {
                yield contents;
            })(),
        };
    },

    /** Read an entry's whole bytes. */
    async bytes(entry: TarballEntry): Promise<Uint8Array<ArrayBuffer>> {
        // copy each chunk into one array of the entry's size
        const bytes = new Uint8Array(entry.size);
        let offset = 0;
        for await (const chunk of entry.body) {
            bytes.set(chunk, offset);
            offset += chunk.byteLength;
        }

        return bytes;
    },
};

/** A gzip-compressed ustar archive of regular files, equal entries always giving equal bytes. */
export const Tarball = {
    /** Stream entries as one gzip member: each file's ustar blocks, then the two end blocks. */
    stream(
        entries: AsyncIterable<TarballEntry>,
        compression: Compression,
    ): ReadableStream<Uint8Array<ArrayBuffer>> {
        // pull one chunk at a time, stopping the entries when the reader cancels
        const chunks = compress(entries, compression);

        return new ReadableStream<Uint8Array<ArrayBuffer>>({
            async pull(controller) {
                const next = await chunks.next();
                if (next.done === true) {
                    controller.close();
                } else {
                    controller.enqueue(next.value);
                }
            },
            async cancel() {
                await chunks.return(undefined);
            },
        });
    },

    /** Read a gzip-compressed ustar archive's regular files in order, refusing a malformed one. */
    async *read(
        stream: ReadableStream<Uint8Array>,
        compression: Compression,
    ): AsyncIterable<TarballEntry> {
        // read whole blocks of the decompressed archive
        const chunks = (async function* () {
            for await (const chunk of stream) {
                yield view(chunk);
            }
        })();
        const read = readBlocks(transform(chunks, compression.decompress("gzip")));
        let path: string | undefined;
        for (;;) {
            // end at the first zero block, and refuse a header whose checksum fails
            const head = await take(read, 1);
            if (head.every((byte) => byte === 0)) {
                return;
            }
            if (readOctal(head, 148, 8) !== checksum(head)) {
                throw PackageError.of("FILE_INVALID", "a tarball header's checksum fails");
            }

            // take a PAX path for the next file, skipping other entries' bytes
            const size = readOctal(head, 124, 12);
            const type = String.fromCharCode(head[156] ?? 0);
            const isFile = type === "0" || type === "\u0000";
            if (!isFile) {
                const record = (await take(read, Math.ceil(size / BLOCK_BYTES))).slice(0, size);
                path = type === "x" ? (readPaxPath(record) ?? path) : path;
                continue;
            }

            // yield a regular file whose body streams its blocks, draining what its reader skips
            const body = streamBody(read, size);
            yield { path: path ?? readName(head), size, body };
            path = undefined;
            const drained = body[Symbol.asyncIterator]();
            while ((await drained.next()).done !== true) {
                // drain what the reader left
            }
        }
    },
};

/** Stream a file's bytes from its blocks, cut to its size. */
async function* streamBody(
    read: AsyncIterator<Uint8Array<ArrayBuffer>>,
    size: number,
): AsyncGenerator<Uint8Array<ArrayBuffer>> {
    for (let remaining = size; remaining > 0; remaining -= BLOCK_BYTES) {
        const block = await take(read, 1);
        yield remaining < BLOCK_BYTES ? block.slice(0, remaining) : block;
    }
}

/** Read a stream's bytes as an iterator of 512-byte blocks. */
async function* readBlocks(
    stream: AsyncIterable<Uint8Array>,
): AsyncGenerator<Uint8Array<ArrayBuffer>> {
    let pending = new Uint8Array(0);
    for await (const chunk of stream) {
        // join the chunk to the bytes left over, then hand out each whole block
        const joined = new Uint8Array(pending.byteLength + chunk.byteLength);
        joined.set(pending, 0);
        joined.set(chunk, pending.byteLength);
        let offset = 0;
        for (; offset + BLOCK_BYTES <= joined.byteLength; offset += BLOCK_BYTES) {
            yield joined.slice(offset, offset + BLOCK_BYTES);
        }
        pending = joined.slice(offset);
    }
    if (pending.byteLength > 0) {
        throw PackageError.of("FILE_INVALID", "a tarball ends inside a block");
    }
}

/** Take some blocks from an iterator as one array, refusing an archive that ends early. */
async function take(
    read: AsyncIterator<Uint8Array<ArrayBuffer>>,
    count: number,
): Promise<Uint8Array<ArrayBuffer>> {
    const taken = new Uint8Array(count * BLOCK_BYTES);
    for (let index = 0; index < count; index++) {
        const next = await read.next();
        if (next.done === true) {
            throw PackageError.of("FILE_INVALID", "a tarball ends before its last entry");
        }
        taken.set(next.value, index * BLOCK_BYTES);
    }

    return taken;
}

/** Read an octal field of a header, ending at its first NUL or space. */
function readOctal(head: Uint8Array, offset: number, length: number): number {
    const text = new TextDecoder().decode(head.subarray(offset, offset + length));

    return Number.parseInt(text.replace(/[\u0000 ].*$/su, "") || "0", 8);
}

/** Sum a header with its checksum field read as spaces, as ustar does. */
function checksum(head: Uint8Array): number {
    let sum = 0;
    for (let index = 0; index < BLOCK_BYTES; index++) {
        sum += index >= 148 && index < 156 ? 0x20 : (head[index] ?? 0);
    }

    return sum;
}

/** Read a header's path: its prefix and name joined by a slash, each ending at its first NUL. */
function readName(head: Uint8Array): string {
    // read each field up to its first NUL, then join the prefix and name
    const field = (offset: number, length: number) =>
        new TextDecoder().decode(head.subarray(offset, offset + length)).replace(/\u0000.*$/su, "");
    const prefix = field(345, PREFIX_BYTES);
    const name = field(0, NAME_BYTES);

    return prefix === "" ? name : `${prefix}/${name}`;
}

/** Read the path record of a PAX extended header, absent without one. */
function readPaxPath(records: Uint8Array): string | undefined {
    for (const record of new TextDecoder().decode(records).split("\n")) {
        const match = /^\d+ path=(.*)$/su.exec(record);
        if (match !== null) {
            return match[1];
        }
    }

    return undefined;
}

/** Write the gzip member of the entries' tar blocks under the fixed header. */
async function* compress(
    entries: AsyncIterable<TarballEntry>,
    compression: Compression,
): AsyncGenerator<Uint8Array<ArrayBuffer>> {
    // replace the runtime's header, which varies between runtimes, with the fixed one
    yield GZIP_HEADER.slice();
    const received: number[] = [];
    for await (const chunk of transform(archive(entries), compression.compress("gzip"))) {
        const skipped = Math.min(GZIP_HEADER.byteLength - received.length, chunk.byteLength);
        received.push(...chunk.subarray(0, skipped));
        if (received.length === GZIP_HEADER.byteLength && received[FLAGS_OFFSET] !== 0) {
            throw new TypeError("the runtime's gzip header has optional fields");
        }
        if (skipped < chunk.byteLength) {
            yield chunk.subarray(skipped);
        }
    }
}

/** Run chunks through a compression pair, writing them as read, never through the host's pipe. */
async function* transform(
    chunks: AsyncIterable<Uint8Array<ArrayBuffer>>,
    pair: CompressionPair,
): AsyncGenerator<Uint8Array<ArrayBuffer>> {
    // write the chunks as they come, failing the output with their failure
    const writer = pair.writable.getWriter();
    const writing = (async () => {
        try {
            for await (const chunk of chunks) {
                await writer.write(chunk);
            }
            await writer.close();
        } catch (error) {
            await writer.abort(error);
        }
    })();

    // read the output, then stop the writing and wait until the chunks stop
    const reader = pair.readable.getReader();
    try {
        for (let next = await reader.read(); !next.done; next = await reader.read()) {
            yield next.value;
        }
    } finally {
        await reader.cancel();
        await writing;
    }
}

/** View a chunk over its own buffer, copying only a chunk over a shared one. */
function view(chunk: Uint8Array): Uint8Array<ArrayBuffer> {
    const { buffer, byteOffset, byteLength } = chunk;

    return buffer instanceof ArrayBuffer
        ? new Uint8Array(buffer, byteOffset, byteLength)
        : new Uint8Array(chunk);
}

/** Write each entry's tar blocks followed by the two zero blocks ending the archive. */
async function* archive(
    entries: AsyncIterable<TarballEntry>,
): AsyncGenerator<Uint8Array<ArrayBuffer>> {
    for await (const entry of entries) {
        yield* blocks(entry);
    }
    yield new Uint8Array(BLOCK_BYTES * 2);
}

/** Write one regular file as ustar blocks: its header, a PAX path record, and its padded bytes. */
async function* blocks(entry: TarballEntry): AsyncGenerator<Uint8Array<ArrayBuffer>> {
    // split the path into a ustar prefix and name, or record it in a PAX header
    const encoded = new TextEncoder().encode(entry.path);
    const split = splitPath(encoded);
    if (split === undefined) {
        const record = paxRecord("path", entry.path);
        yield header("PaxHeader", new Uint8Array(0), record.byteLength, "x");
        yield* pad(record);
    }

    // write the file's header, then its bytes, refusing a body of another size
    const { prefix, name } = split ?? {
        prefix: new Uint8Array(0),
        name: encoded.slice(0, NAME_BYTES),
    };
    yield header(name, prefix, entry.size, "0");
    let written = 0;
    for await (const chunk of entry.body) {
        written += chunk.byteLength;
        yield view(chunk);
    }
    if (written !== entry.size) {
        throw PackageError.of(
            "FILE_INVALID",
            `a tarball entry of ${entry.size} bytes has ${written}`,
        );
    }

    // pad the bytes to whole blocks
    const remainder = written % BLOCK_BYTES;
    if (remainder !== 0) {
        yield new Uint8Array(BLOCK_BYTES - remainder);
    }
}

/** Split an encoded path at a slash into a prefix and name that fit, absent when none fits. */
function splitPath(
    path: Uint8Array<ArrayBuffer>,
): { prefix: Uint8Array<ArrayBuffer>; name: Uint8Array<ArrayBuffer> } | undefined {
    // keep a short path whole
    if (path.byteLength <= NAME_BYTES) {
        return { prefix: new Uint8Array(0), name: path };
    }

    // take the first slash leaving a name short enough, with a prefix short enough
    const slash = "/".charCodeAt(0);
    for (let index = path.byteLength - NAME_BYTES - 1; index < path.byteLength; index++) {
        if (index > 0 && index <= PREFIX_BYTES && path[index] === slash) {
            return { prefix: path.slice(0, index), name: path.slice(index + 1) };
        }
    }

    return undefined;
}

/** Write a ustar header block for a name and prefix, a size and an entry type. */
function header(
    name: string | Uint8Array<ArrayBuffer>,
    prefix: Uint8Array<ArrayBuffer>,
    size: number,
    type: "0" | "x",
): Uint8Array<ArrayBuffer> {
    // fill the fields with fixed ownership and time
    const block = new Uint8Array(BLOCK_BYTES);
    const encoder = new TextEncoder();
    block.set(typeof name === "string" ? encoder.encode(name) : name, 0);
    block.set(encoder.encode(octal(FILE_MODE, 7)), 100);
    block.set(encoder.encode(octal(0, 7)), 108);
    block.set(encoder.encode(octal(0, 7)), 116);
    block.set(encoder.encode(octal(size, 11)), 124);
    block.set(encoder.encode(octal(0, 11)), 136);
    block.set(encoder.encode(type), 156);
    block.set(encoder.encode("ustar\u000000"), 257);
    block.set(prefix, 345);

    // sum the header with the checksum field read as spaces
    block.fill(0x20, 148, 156);
    const sum = block.reduce((total, byte) => total + byte, 0);
    block.set(encoder.encode(`${sum.toString(8).padStart(6, "0")}\u0000 `), 148);

    return block;
}

/** Write a PAX extended header record: its length, key and value. */
function paxRecord(key: string, value: string): Uint8Array<ArrayBuffer> {
    // count the record's own length digits into its length
    const body = ` ${key}=${value}\n`;
    const bytes = new TextEncoder().encode(body).byteLength;
    let length = bytes + String(bytes).length;
    if (String(length).length !== String(bytes).length) {
        length += 1;
    }

    return new TextEncoder().encode(`${length}${body}`);
}

/** Pad contents with zeros to whole blocks. */
function pad(contents: Uint8Array<ArrayBuffer>): Uint8Array<ArrayBuffer>[] {
    const remainder = contents.byteLength % BLOCK_BYTES;

    return remainder === 0 ? [contents] : [contents, new Uint8Array(BLOCK_BYTES - remainder)];
}

/** Write a number in zero-padded octal of a width, followed by a NUL. */
function octal(value: number, width: number): string {
    return `${value.toString(8).padStart(width, "0")}\u0000`;
}
