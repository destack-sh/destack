import { crc32 } from "node:zlib";
import type { Hasher } from "./hasher.ts";

/** The byte table of the reversed CRC-32C (Castagnoli) polynomial. */
const CRC32C_TABLE = createTable(0x82f63b78);

/** An incremental CRC-32 (ISO-HDLC) from node:zlib, digested big-endian as S3 encodes it. */
export class Crc32 implements Hasher {
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
export class Crc32c implements Hasher {
    /** The running remainder. */
    #crc = -1;

    /** Add bytes to the checksum. */
    update(bytes: Uint8Array): void {
        let crc = this.#crc;
        for (const byte of bytes) {
            const entry = CRC32C_TABLE[(crc ^ byte) & 0xff];
            if (entry === undefined) {
                throw new TypeError("crc-32c table has no entry for a byte");
            }
            crc = entry ^ (crc >>> 8);
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
