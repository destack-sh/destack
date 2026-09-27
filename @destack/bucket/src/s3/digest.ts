import { createHash, type Hash } from "node:crypto";
import { crc32 } from "node:zlib";

/** The byte table of the reversed CRC-32C (Castagnoli) polynomial, which node:zlib does not offer. */
const CRC32C_TABLE = createTable(0x82f63b78);

/** An incremental digest of a byte stream, spent by its one digest call. */
export interface Digest {
    /** Add bytes to the digest. */
    update(bytes: Uint8Array): void;
    /** Finish the digest and return its bytes. */
    digest(): Uint8Array<ArrayBuffer>;
}

/** An incremental MD5 or SHA-256 digest from node:crypto. */
export class HashDigest implements Digest {
    /** The running hash. */
    readonly #hash: Hash;

    /** Start a digest of the algorithm. */
    constructor(algorithm: "md5" | "sha256") {
        this.#hash = createHash(algorithm);
    }

    /** Add bytes to the digest. */
    update(bytes: Uint8Array): void {
        this.#hash.update(bytes);
    }

    /** Finish the digest and return its bytes. */
    digest(): Uint8Array<ArrayBuffer> {
        return new Uint8Array(this.#hash.digest());
    }
}

/** An incremental CRC-32 (ISO-HDLC) from node:zlib, digested big-endian as S3 encodes it. */
export class Crc32 implements Digest {
    /** The running checksum. */
    #crc = 0;

    /** Add bytes to the checksum. */
    update(bytes: Uint8Array): void {
        this.#crc = crc32(bytes, this.#crc);
    }

    /** Finish the checksum and return its 4 bytes. */
    digest(): Uint8Array<ArrayBuffer> {
        const output = new Uint8Array(4);
        new DataView(output.buffer).setUint32(0, this.#crc);

        return output;
    }
}

/** An incremental CRC-32C (Castagnoli), digested big-endian as S3 encodes it. */
export class Crc32c implements Digest {
    /** The running remainder. */
    #crc = -1;

    /** Add bytes to the checksum. */
    update(bytes: Uint8Array): void {
        let crc = this.#crc;
        for (let index = 0; index < bytes.length; index++) {
            crc = CRC32C_TABLE[(crc ^ bytes[index]!) & 0xff]! ^ (crc >>> 8);
        }
        this.#crc = crc;
    }

    /** Finish the checksum and return its 4 bytes. */
    digest(): Uint8Array<ArrayBuffer> {
        const output = new Uint8Array(4);
        new DataView(output.buffer).setInt32(0, ~this.#crc);

        return output;
    }
}

/** Build the byte table of a reversed CRC-32 polynomial. */
function createTable(polynomial: number): Int32Array {
    const table = new Int32Array(256);
    for (let byte = 0; byte < 256; byte++) {
        let crc = byte;
        for (let bit = 0; bit < 8; bit++) {
            crc = crc & 1 ? (crc >>> 1) ^ polynomial : crc >>> 1;
        }
        table[byte] = crc;
    }

    return table;
}
