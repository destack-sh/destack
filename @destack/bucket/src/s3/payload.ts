import { Digest } from "@destack/schema";
import { Crc32, Crc32c, CryptoHasher, type Hasher } from "./hasher.ts";
import { S3Error } from "./error.ts";
import { EMPTY_HASH, type S3Authorization, signString, UNSIGNED_PAYLOAD } from "./signature.ts";

/** The payload hash of aws-chunked bodies with signed chunks. */
const SIGNED_CHUNKS = "STREAMING-AWS4-HMAC-SHA256-PAYLOAD";
/** The payload hash of aws-chunked bodies with unsigned chunks and a trailing checksum. */
const UNSIGNED_CHUNKS_WITH_TRAILER = "STREAMING-UNSIGNED-PAYLOAD-TRAILER";
/** The payload hash of aws-chunked bodies with signed chunks and a signed trailing checksum. */
const SIGNED_CHUNKS_WITH_TRAILER = "STREAMING-AWS4-HMAC-SHA256-PAYLOAD-TRAILER";
/** The algorithm line of a chunk's string to sign. */
const CHUNK_ALGORITHM = "AWS4-HMAC-SHA256-PAYLOAD";
/**
 * The longest aws-chunked header or trailer line in bytes.
 *
 * A chunk header is at most 16 hexadecimal digits, `;chunk-signature=` and 64 digits, about 100 bytes.
 */
const MAX_LINE_BYTES = 1024;
/** The checksum headers S3 defines, with the digests verifying them and their lengths in bytes. */
const CHECKSUMS = {
    "x-amz-checksum-crc32": { create: () => new Crc32(), length: 4 },
    "x-amz-checksum-crc32c": { create: () => new Crc32c(), length: 4 },
    "x-amz-checksum-sha256": { create: () => new CryptoHasher("sha256"), length: 32 },
    "x-amz-checksum-sha1": undefined,
    "x-amz-checksum-crc64nvme": undefined,
} as const;

/** A checksum a request declares, verified over the decoded body. */
interface Check {
    /** The header or trailer naming the expected value. */
    readonly name: string;
    /** The running digest. */
    readonly digest: Hasher;
    /** The expected base64 value, or undefined until the trailer supplies it. */
    expected?: string;
}

/** The position of an aws-chunked decoder. */
type ChunkState = "header" | "data" | "data-end" | "final" | "trailer" | "done";

/** A request body as the bytes the client sent, decoded from aws-chunked, with its hashes verified while streaming. */
export class Payload implements Transformer<Uint8Array, Uint8Array> {
    /** The verified signature whose key signs streamed chunks. */
    readonly #authorization: S3Authorization;
    /** The expected hexadecimal SHA-256 of a signed plain body. */
    readonly #payloadHash?: string;
    /** The SHA-256 of a signed plain body. */
    readonly #payloadDigest?: CryptoHasher;
    /** The declared checksums. */
    readonly #checks: Check[];
    /** The declared decoded length, when the request declares one. */
    readonly #length?: number;
    /** Whether the body is aws-chunked, and whether its chunks are signed. */
    readonly #chunks?: "signed" | "unsigned";
    /** Whether an aws-chunked body ends with trailing headers. */
    readonly #hasTrailer: boolean;
    /** The number of decoded bytes so far. */
    #received = 0;
    /** The aws-chunked position. */
    #state: ChunkState = "header";
    /** The bytes of the line being read. */
    #line: number[] = [];
    /** The bytes left in the current chunk. */
    #remaining = 0;
    /** The signature the current chunk declares. */
    #chunkSignature = "";
    /** The SHA-256 of the current chunk's data. */
    #chunkDigest = new CryptoHasher("sha256");
    /** The signature of the previous chunk, seeded by the request signature. */
    #previousSignature: string;
    /** The trailing headers by lowercase name. */
    readonly #trailers = new Map<string, string>();

    /** Read the payload hash, content length and checksums a request declares. */
    private constructor(headers: Headers, authorization: S3Authorization) {
        // chain chunk signatures from the request signature
        this.#authorization = authorization;
        this.#previousSignature = authorization.signature;
        const { payloadHash } = authorization;

        // decode aws-chunked bodies, verify signed plain bodies, and pass unsigned ones
        if (payloadHash === SIGNED_CHUNKS || payloadHash === UNSIGNED_CHUNKS_WITH_TRAILER) {
            this.#chunks = payloadHash === SIGNED_CHUNKS ? "signed" : "unsigned";
            this.#length = readLength(headers.get("x-amz-decoded-content-length"));
        } else if (Digest.safeParse(payloadHash).success) {
            this.#payloadHash = payloadHash;
            this.#payloadDigest = new CryptoHasher("sha256");
        } else if (payloadHash === SIGNED_CHUNKS_WITH_TRAILER) {
            throw new S3Error("NotImplemented", `the payload hash ${payloadHash} is not supported`);
        } else if (payloadHash !== UNSIGNED_PAYLOAD) {
            throw new S3Error(
                "InvalidArgument",
                "x-amz-content-sha256 must be UNSIGNED-PAYLOAD, a streaming payload or a SHA-256",
            );
        }

        // read the declared length of plain bodies
        const contentLength = headers.get("content-length");
        if (this.#chunks === undefined && contentLength !== null) {
            this.#length = readLength(contentLength);
        }

        // collect the declared checksums, one of which the trailer may carry
        this.#checks = readChecks(headers);
        const trailer = headers.get("x-amz-trailer")?.toLowerCase();
        this.#hasTrailer = trailer !== undefined;
        if (trailer !== undefined) {
            if (this.#chunks !== "unsigned") {
                throw new S3Error(
                    "InvalidRequest",
                    "trailing checksums require an aws-chunked body",
                );
            }
            this.#checks.push({ name: trailer, digest: readChecksum(trailer).create() });
        }
        if (this.#chunks === "unsigned" && !this.#hasTrailer) {
            throw new S3Error(
                "InvalidRequest",
                `${UNSIGNED_CHUNKS_WITH_TRAILER} requires x-amz-trailer`,
            );
        }
    }

    /** Stream a request body through its decoding and verification, as an empty body when absent. */
    static open(
        request: Request,
        headers: Headers,
        authorization: S3Authorization,
    ): ReadableStream<Uint8Array> {
        const payload = new Payload(headers, authorization);
        const body = request.body ?? new Response(new Uint8Array()).body!;

        return body.pipeThrough(new TransformStream(payload));
    }

    /** Decode and verify one received chunk of the body. */
    async transform(
        chunk: Uint8Array,
        controller: TransformStreamDefaultController<Uint8Array>,
    ): Promise<void> {
        // pass plain bodies through while hashing them
        if (this.#chunks === undefined) {
            this.#consume(chunk);
            controller.enqueue(chunk);

            return;
        }

        // alternate between chunk data and the lines around it
        let offset = 0;
        while (offset < chunk.length) {
            // pass the data of the current chunk
            if (this.#state === "data") {
                const size = Math.min(this.#remaining, chunk.length - offset);
                const data = chunk.subarray(offset, offset + size);
                this.#chunkDigest.update(data);
                this.#consume(data);
                controller.enqueue(data);
                this.#remaining -= size;
                offset += size;
                if (this.#remaining === 0) {
                    this.#state = "data-end";
                }
            }
            // collect a line up to its line feed
            else {
                const end = chunk.indexOf(0x0a, offset);
                const stop = end === -1 ? chunk.length : end + 1;
                if (this.#line.length + stop - offset > MAX_LINE_BYTES) {
                    throw new S3Error("IncompleteBody", "an aws-chunked line is too long");
                }
                this.#line.push(...chunk.subarray(offset, stop));
                offset = stop;
                if (end !== -1) {
                    await this.#readLine();
                }
            }
        }
    }

    /** Require the complete body and compare its hashes with the declared ones. */
    flush(): void {
        // require a complete aws-chunked body of the declared length
        if (this.#chunks !== undefined && this.#state !== "done") {
            throw new S3Error("IncompleteBody", "the aws-chunked body ends before its final chunk");
        }
        if (this.#length !== undefined && this.#received !== this.#length) {
            throw new S3Error("IncompleteBody", "the body length differs from the declared length");
        }

        // compare the signed payload hash
        if (
            this.#payloadDigest !== undefined &&
            this.#payloadDigest.digest().toHex() !== this.#payloadHash
        ) {
            throw new S3Error(
                "XAmzContentSHA256Mismatch",
                "the provided x-amz-content-sha256 does not match what was computed",
            );
        }

        // compare each declared checksum, reading trailing ones from the trailer
        for (const check of this.#checks) {
            const expected = check.expected ?? this.#trailers.get(check.name);
            if (expected === undefined) {
                throw new S3Error("InvalidRequest", `the trailer does not carry ${check.name}`);
            }
            if (check.digest.digest().toBase64() !== expected) {
                throw new S3Error(
                    "BadDigest",
                    `the ${check.name} you specified did not match the calculated checksum`,
                );
            }
        }
    }

    /** Count and hash decoded bytes. */
    #consume(data: Uint8Array): void {
        this.#received += data.length;
        this.#payloadDigest?.update(data);
        for (const check of this.#checks) {
            check.digest.update(data);
        }
    }

    /** Act on one complete aws-chunked line. */
    async #readLine(): Promise<void> {
        // require CRLF line endings
        const bytes = this.#line;
        this.#line = [];
        if (bytes.at(-2) !== 0x0d) {
            throw new S3Error("IncompleteBody", "aws-chunked lines must end with CRLF");
        }
        const line = new TextDecoder().decode(new Uint8Array(bytes.slice(0, -2)));

        // start a chunk from its size and signature
        if (this.#state === "header") {
            await this.#readHeader(line);
        }
        // verify the chunk the line ends
        else if (this.#state === "data-end") {
            if (line !== "") {
                throw new S3Error("IncompleteBody", "an aws-chunked chunk is longer than its size");
            }
            await this.#verifyChunk();
            this.#state = "header";
        }
        // collect trailing headers until the empty line
        else if (this.#state === "trailer") {
            const separator = line.indexOf(":");
            if (line === "") {
                this.#state = "done";
            } else if (separator === -1) {
                throw new S3Error("IncompleteBody", "an aws-chunked trailer line lacks a colon");
            } else {
                this.#trailers.set(
                    line.slice(0, separator).trim().toLowerCase(),
                    line.slice(separator + 1).trim(),
                );
            }
        }
        // end after the empty line following the final chunk
        else if (this.#state === "final" && line === "") {
            this.#state = "done";
        }
        // refuse bytes after the end
        else {
            throw new S3Error(
                "IncompleteBody",
                "the aws-chunked body continues after its final chunk",
            );
        }
    }

    /** Read a chunk header, verifying the final empty chunk at once. */
    async #readHeader(line: string): Promise<void> {
        // read the hexadecimal size and, for signed chunks, the signature
        const match =
            this.#chunks === "signed"
                ? /^([0-9a-fA-F]{1,16});chunk-signature=([0-9a-f]{64})$/.exec(line)
                : /^([0-9a-fA-F]{1,16})$/.exec(line);
        if (match === null) {
            throw new S3Error("IncompleteBody", "an aws-chunked chunk header is malformed");
        }
        this.#remaining = Number.parseInt(match[1]!, 16);
        this.#chunkSignature = match[2] ?? "";
        this.#chunkDigest = new CryptoHasher("sha256");

        // read data, or end with the empty chunk
        if (this.#remaining > 0) {
            this.#state = "data";
        } else {
            await this.#verifyChunk();
            this.#state = this.#hasTrailer ? "trailer" : "final";
        }
    }

    /** Verify the signature of a signed chunk, chained to the previous one. */
    async #verifyChunk(): Promise<void> {
        if (this.#chunks !== "signed") {
            return;
        }

        // sign the chunk's data hash after the previous signature
        const { signingKey, date, scope } = this.#authorization;
        const stringToSign = [
            CHUNK_ALGORITHM,
            date,
            scope,
            this.#previousSignature,
            EMPTY_HASH,
            this.#chunkDigest.digest().toHex(),
        ].join("\n");
        const expected = await signString(signingKey, stringToSign);
        if (expected !== this.#chunkSignature) {
            throw new S3Error(
                "SignatureDoesNotMatch",
                "an aws-chunked chunk signature does not match",
            );
        }
        this.#previousSignature = expected;
    }
}

/** Read the checksum headers a request declares. */
function readChecks(headers: Headers): Check[] {
    // verify an MD5 digest of the body
    const checks: Check[] = [];
    const md5 = headers.get("content-md5");
    if (md5 !== null) {
        checks.push({
            name: "content-md5",
            digest: new CryptoHasher("md5"),
            expected: readBase64(md5, 16, "content-md5"),
        });
    }

    // verify at most one x-amz checksum
    const names = [...headers.keys()].filter((name) => name.startsWith("x-amz-checksum-"));
    if (names.length > 1) {
        throw new S3Error("InvalidRequest", "expecting a single x-amz-checksum- header");
    }
    for (const name of names) {
        const checksum = readChecksum(name);
        const expected = readBase64(headers.get(name)!, checksum.length, name);
        checks.push({ name, digest: checksum.create(), expected });
    }

    return checks;
}

/** Select the digest of a checksum header, refusing algorithms this server does not compute. */
function readChecksum(name: string): { create: () => Hasher; length: number } {
    if (!(name in CHECKSUMS)) {
        throw new S3Error("InvalidRequest", `the checksum ${name} is not an S3 checksum`);
    }
    const checksum = CHECKSUMS[name as keyof typeof CHECKSUMS];
    if (checksum === undefined) {
        throw new S3Error("NotImplemented", `the checksum ${name} is not supported`);
    }

    return checksum;
}

/** Read a base64 digest of an expected length, returning it canonically encoded. */
function readBase64(value: string, length: number, name: string): string {
    try {
        const bytes = Uint8Array.fromBase64(value);
        if (bytes.length === length) {
            return bytes.toBase64();
        }
    } catch (cause) {
        throw new S3Error("InvalidDigest", `the ${name} header is not valid base64`, { cause });
    }
    throw new S3Error("InvalidDigest", `the ${name} header must encode ${length} bytes`);
}

/** Read a declared byte length. */
function readLength(value: string | null): number {
    if (value === null) {
        throw new S3Error(
            "MissingContentLength",
            "aws-chunked bodies require x-amz-decoded-content-length",
        );
    }
    if (!/^\d{1,15}$/.test(value)) {
        throw new S3Error(
            "InvalidArgument",
            "the declared body length must be a non-negative integer",
        );
    }

    return Number(value);
}
