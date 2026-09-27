import { open, unlink } from "node:fs/promises";
import { join } from "node:path";
import { createHash } from "node:crypto";
import type { BucketBody, BucketPutOptions, StringChecksums } from "../bucket/index.ts";
import { CHECKSUM_ALGORITHMS } from "../bucket/index.ts";
import { StorageError } from "../error/index.ts";
import { syncDirectory } from "./directory.ts";

/** A complete immutable content file. */
export class ContentFile {
    /** The unique filename. */
    readonly version: string;
    /** The length in bytes. */
    readonly size: number;
    /** The MD5 entity tag. */
    readonly etag: string;
    /** The stored content checksums. */
    readonly checksums: StringChecksums;

    /** Retain the written content's identifier, length, and digest. */
    constructor(version: string, size: number, etag: string, checksums: StringChecksums) {
        // retain the content description
        this.version = version;
        this.size = size;
        this.etag = etag;
        this.checksums = checksums;
    }

    /** Stream, hash, and flush contents before publishing any catalogue reference. */
    static async write(
        directory: string,
        body: BucketBody,
        options: BucketPutOptions = {},
    ): Promise<ContentFile> {
        // accept at most one supplied checksum
        const supplied = CHECKSUM_ALGORITHMS.filter(
            (algorithm) => options[algorithm] !== undefined,
        );
        if (supplied.length > 1) {
            throw new StorageError("INVALID_CHECKSUM", "supply at most one checksum");
        }
        const algorithm = supplied[0];
        const expected = algorithm === undefined ? undefined : options[algorithm]!;
        const checksum =
            algorithm === undefined || algorithm === "md5" ? undefined : createHash(algorithm);
        const version = crypto.randomUUID();
        const path = join(directory, version);
        const hash = createHash("md5");
        let size = 0;
        let isCreated = false;
        try {
            // write complete chunks and hash exactly the bytes stored
            {
                await using file = await open(path, "wx", 0o600);
                isCreated = true;
                const content = ArrayBuffer.isView(body)
                    ? new Uint8Array(body.buffer, body.byteOffset, body.byteLength)
                    : body;
                const stream =
                    body instanceof ReadableStream
                        ? body
                        : new Response(content ?? new Uint8Array()).body!;
                for await (const chunk of stream) {
                    let offset = 0;
                    while (offset < chunk.byteLength) {
                        const { bytesWritten } = await file.write(chunk, offset);
                        if (bytesWritten === 0) {
                            throw new StorageError(
                                "WRITE_FAILED",
                                "the filesystem stopped accepting file contents",
                            );
                        }
                        offset += bytesWritten;
                    }
                    hash.update(chunk);
                    checksum?.update(chunk);
                    size += chunk.byteLength;
                }
                await file.sync();
            }

            // verify the supplied digest before making the file available to the catalogue
            const etag = hash.digest("hex");
            const digests: StringChecksums = { md5: etag };
            if (algorithm !== undefined && expected !== undefined) {
                const actual = checksum === undefined ? etag : checksum.digest("hex");
                const hex =
                    typeof expected === "string"
                        ? expected.toLowerCase()
                        : expected instanceof ArrayBuffer
                          ? new Uint8Array(expected).toHex()
                          : new Uint8Array(
                                expected.buffer,
                                expected.byteOffset,
                                expected.byteLength,
                            ).toHex();
                if (actual !== hex) {
                    throw new StorageError(
                        "INVALID_CHECKSUM",
                        "object checksum does not match its contents",
                    );
                }
                digests[algorithm] = actual;
            }

            // persist the directory entry before the catalogue can name the file
            await syncDirectory(directory);

            return new ContentFile(version, size, etag, digests);
        } catch (error) {
            if (!isCreated) {
                throw error;
            }
            try {
                await unlink(path);
            } catch (cleanup) {
                throw new AggregateError([error, cleanup], "file upload and file cleanup failed");
            }
            throw error;
        }
    }
}
