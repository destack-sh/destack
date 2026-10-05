/** The size of a tar block, which holds a header or a slice of a file. */
const BLOCK_BYTES = 512;

/** The longest file name a ustar header holds in its name field. */
const NAME_BYTES = 100;

/** The longest directory prefix a ustar header holds in its prefix field. */
const PREFIX_BYTES = 155;

/** The permissions of every archived file: readable by all, written by the owner. */
const FILE_MODE = 0o644;

/** The gzip member header: deflate, no flags, no time, no extra flags, unknown system (RFC 1952). */
const GZIP_HEADER = new Uint8Array([0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff]);

/** The offset of the gzip header's flags, which announce optional fields (RFC 1952). */
const FLAGS_OFFSET = 3;

/** One regular file of a tarball. */
export interface TarballEntry {
    /** The path within the archive. */
    readonly path: string;
    /** The file's bytes. */
    readonly contents: Uint8Array<ArrayBuffer>;
}

/** A gzip-compressed ustar archive of regular files, equal entries always giving equal bytes. */
export const Tarball = {
    /** Stream entries as one gzip member: each file's ustar blocks followed by the two end blocks. */
    stream(entries: AsyncIterable<TarballEntry>): ReadableStream<Uint8Array<ArrayBuffer>> {
        // pull one chunk at a time, stopping the entries when the reader cancels
        const chunks = compress(entries);

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
};

/** Write the gzip member of the entries' tar blocks under the fixed header, the runtime computing the deflated blocks, CRC-32 and size. */
async function* compress(
    entries: AsyncIterable<TarballEntry>,
): AsyncGenerator<Uint8Array<ArrayBuffer>> {
    // compress the blocks as they are written, and fail the output with their failure
    const gzip = new CompressionStream("gzip");
    const writer = gzip.writable.getWriter();
    const writing = (async () => {
        try {
            for await (const block of archive(entries)) {
                await writer.write(block);
            }
            await writer.close();
        } catch (error) {
            await writer.abort(error);
        }
    })();

    // replace the runtime's header, which varies between runtimes, with the fixed one
    const reader = gzip.readable.getReader();
    try {
        yield GZIP_HEADER.slice();
        const received: number[] = [];
        for (let next = await reader.read(); !next.done; next = await reader.read()) {
            const skipped = Math.min(
                GZIP_HEADER.byteLength - received.length,
                next.value.byteLength,
            );
            received.push(...next.value.subarray(0, skipped));
            if (received.length === GZIP_HEADER.byteLength && received[FLAGS_OFFSET] !== 0) {
                throw new TypeError("the runtime's gzip header carries optional fields");
            }
            if (skipped < next.value.byteLength) {
                yield next.value.subarray(skipped);
            }
        }
    } finally {
        // stop the writing, and wait until the entries stop
        await reader.cancel();
        await writing;
    }
}

/** Write each entry's tar blocks followed by the two zero blocks ending the archive. */
async function* archive(
    entries: AsyncIterable<TarballEntry>,
): AsyncGenerator<Uint8Array<ArrayBuffer>> {
    for await (const entry of entries) {
        yield* blocks(entry.path, entry.contents);
    }
    yield new Uint8Array(BLOCK_BYTES * 2);
}

/** Write one regular file as ustar blocks: its header, a PAX path record when the path is too long, its contents padded to a block. */
function blocks(path: string, contents: Uint8Array<ArrayBuffer>): Uint8Array<ArrayBuffer>[] {
    // split the path into a ustar prefix and name, or record it in a PAX header
    const encoded = new TextEncoder().encode(path);
    const split = splitPath(encoded);
    const written: Uint8Array<ArrayBuffer>[] = [];
    if (split === undefined) {
        const record = paxRecord("path", path);
        written.push(
            header("PaxHeader", new Uint8Array(0), record.byteLength, "x"),
            ...pad(record),
        );
    }

    // write the file's header and padded contents
    const { prefix, name } = split ?? {
        prefix: new Uint8Array(0),
        name: encoded.slice(0, NAME_BYTES),
    };
    written.push(header(name, prefix, contents.byteLength, "0"), ...pad(contents));

    return written;
}

/** Split an encoded path at a slash into a prefix and name that fit their fields, absent when none fits. */
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
