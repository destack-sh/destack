const U128_VARINT_MAX_BYTES = 19;
const U128_VARINT_LAST_BYTE_MAX = 0x03;
const U128_MAX = (1n << 128n) - 1n;
const I128_MIN = -(1n << 127n);
const I128_MAX = (1n << 127n) - 1n;

/** Error thrown while encoding or decoding Destack binary serde bytes. */
export class SerdeError extends Error {
    /** Create one serde error. */
    constructor(message: string) {
        super(message);
        this.name = "SerdeError";
    }
}

/** Writer for canonical Destack binary serde bytes. */
export class Writer {
    readonly #bytes: number[] = [];

    /** Return the written bytes. */
    bytes(): Uint8Array {
        return new Uint8Array(this.#bytes);
    }

    /** Write one raw byte. */
    writeByte(value: number): void {
        if (!Number.isInteger(value) || value < 0 || value > 0xff) {
            throw new SerdeError(`byte out of range: ${value}`);
        }

        this.#bytes.push(value);
    }

    /** Write raw bytes. */
    writeBytes(value: Uint8Array | readonly number[]): void {
        for (const byte of value) {
            this.writeByte(byte);
        }
    }

    /** Write one boolean. */
    writeBool(value: boolean): void {
        this.writeByte(value ? 1 : 0);
    }

    /** Write one unsigned integer varint. */
    writeUnsigned(value: number | bigint): void {
        let integer = unsignedBigint(value);
        let byteCount = 0;

        while (true) {
            if (byteCount === U128_VARINT_MAX_BYTES) {
                throw new SerdeError("varint too large");
            }
            byteCount += 1;

            let byte = Number(integer & 0x7fn);
            integer >>= 7n;

            if (integer === 0n) {
                this.writeByte(byte);
                return;
            }

            byte |= 0x80;
            this.writeByte(byte);
        }
    }

    /** Write one signed integer zigzag varint. */
    writeSigned(value: number | bigint): void {
        const integer = signedBigint(value);
        const encoded = (integer << 1n) ^ (integer >> 127n);

        this.writeUnsigned(encoded);
    }

    /** Write one signed i8 byte. */
    writeI8(value: number): void {
        if (!Number.isInteger(value) || value < -128 || value > 127) {
            throw new SerdeError(`i8 out of range: ${value}`);
        }

        this.writeByte(value < 0 ? value + 256 : value);
    }

    /** Write one little endian f32. */
    writeF32(value: number): void {
        const bytes = new ArrayBuffer(4);
        new DataView(bytes).setFloat32(0, value, true);

        this.writeBytes(new Uint8Array(bytes));
    }

    /** Write one little endian f64. */
    writeF64(value: number): void {
        const bytes = new ArrayBuffer(8);
        new DataView(bytes).setFloat64(0, value, true);

        this.writeBytes(new Uint8Array(bytes));
    }

    /** Write one unicode scalar value. */
    writeChar(value: string): void {
        const codePoint = singleCodePoint(value);

        this.writeUnsigned(codePoint);
    }

    /** Write one UTF-8 string. */
    writeString(value: string): void {
        const bytes = new TextEncoder().encode(value);

        this.writeByteSlice(bytes);
    }

    /** Write one JSON value. */
    writeJson(value: unknown): void {
        const json = JSON.stringify(value);
        if (json === undefined) {
            throw new SerdeError("JSON value is not serializable");
        }

        this.writeString(json);
    }

    /** Write one length-prefixed byte slice. */
    writeByteSlice(value: Uint8Array | readonly number[]): void {
        this.writeUnsigned(value.length);
        this.writeBytes(value);
    }

    /** Write one optional value. */
    writeOption<T>(value: T | undefined, encode: (value: T) => void): void {
        if (value === undefined) {
            this.writeByte(0);
        } else {
            this.writeByte(1);
            encode(value);
        }
    }
}

/** Reader for canonical Destack binary serde bytes. */
export class Reader {
    readonly #bytes: Uint8Array;
    #offset = 0;

    /** Create one reader. */
    constructor(bytes: Uint8Array | readonly number[]) {
        this.#bytes = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
    }

    /** Validate that all input bytes were consumed. */
    finish(): void {
        if (this.#offset !== this.#bytes.length) {
            throw new SerdeError("trailing bytes");
        }
    }

    /** Read one raw byte. */
    readByte(): number {
        const byte = this.#bytes[this.#offset];
        if (byte === undefined) {
            throw new SerdeError("unexpected end of input");
        }

        this.#offset += 1;

        return byte;
    }

    /** Read exact raw bytes. */
    readBytes(length: number): Uint8Array {
        if (!Number.isInteger(length) || length < 0) {
            throw new SerdeError(`invalid byte length: ${length}`);
        }

        const end = this.#offset + length;
        if (end > this.#bytes.length) {
            throw new SerdeError("unexpected end of input");
        }

        const bytes = this.#bytes.slice(this.#offset, end);
        this.#offset = end;

        return bytes;
    }

    /** Read one boolean. */
    readBool(): boolean {
        const value = this.readByte();

        if (value === 0) {
            return false;
        } else if (value === 1) {
            return true;
        }

        throw new SerdeError(`invalid bool byte: ${value}`);
    }

    /** Read one unsigned integer varint. */
    readUnsigned(): bigint {
        let value = 0n;
        let shift = 0n;
        let byteCount = 0;

        while (true) {
            if (byteCount === U128_VARINT_MAX_BYTES) {
                throw new SerdeError("varint too large");
            }
            byteCount += 1;

            const byte = this.readByte();
            const chunk = BigInt(byte & 0x7f);
            if (shift === 126n && (byte & 0x7f) > U128_VARINT_LAST_BYTE_MAX) {
                throw new SerdeError("varint too large");
            }
            value |= chunk << shift;

            if ((byte & 0x80) === 0) {
                if (shift > 0n && chunk === 0n) {
                    throw new SerdeError("non canonical varint");
                }

                return value;
            }

            shift += 7n;
        }
    }

    /** Read one unsigned integer varint as a number. */
    readNumber(): number {
        return safeNumber(this.readUnsigned());
    }

    /** Read one signed integer zigzag varint. */
    readSigned(): bigint {
        const value = this.readUnsigned();

        return (value >> 1n) ^ (-(value & 1n));
    }

    /** Read one signed integer zigzag varint as a number. */
    readSignedNumber(): number {
        return safeNumber(this.readSigned());
    }

    /** Read one signed i8 byte. */
    readI8(): number {
        const byte = this.readByte();

        return byte > 127 ? byte - 256 : byte;
    }

    /** Read one little endian f32. */
    readF32(): number {
        const bytes = this.readBytes(4);

        return new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getFloat32(0, true);
    }

    /** Read one little endian f64. */
    readF64(): number {
        const bytes = this.readBytes(8);

        return new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getFloat64(0, true);
    }

    /** Read one unicode scalar value. */
    readChar(): string {
        const codePoint = this.readNumber();
        const value = String.fromCodePoint(codePoint);

        if (value.length === 0) {
            throw new SerdeError(`invalid char code point: ${codePoint}`);
        }

        return value;
    }

    /** Read one UTF-8 string. */
    readString(): string {
        return new TextDecoder().decode(this.readByteSlice());
    }

    /** Read one JSON value. */
    readJson(): unknown {
        const json = this.readString();

        return JSON.parse(json);
    }

    /** Read one length-prefixed byte slice. */
    readByteSlice(): Uint8Array {
        const length = this.readNumber();

        return this.readBytes(length);
    }

    /** Read one optional value. */
    readOption<T>(decode: () => T): T | undefined {
        const tag = this.readByte();

        if (tag === 0) {
            return undefined;
        } else if (tag === 1) {
            return decode();
        }

        throw new SerdeError(`invalid option byte: ${tag}`);
    }
}

/** Encode one value into nested bytes. */
export function nestedBytes(encode: (writer: Writer) => void): Uint8Array {
    const writer = new Writer();
    encode(writer);

    return writer.bytes();
}

/** Compare canonical byte slices lexicographically. */
export function compareBytes(left: Uint8Array, right: Uint8Array): number {
    const length = Math.min(left.length, right.length);

    for (let index = 0; index < length; index += 1) {
        const delta = left[index] - right[index];
        if (delta !== 0) {
            return delta;
        }
    }

    return left.length - right.length;
}

/** Encode one complete value. */
export function encodeValue(encode: (writer: Writer) => void): Uint8Array {
    return nestedBytes(encode);
}

/** Decode one complete value. */
export function decodeValue<T>(
    bytes: Uint8Array | readonly number[],
    decode: (reader: Reader) => T,
): T {
    const reader = new Reader(bytes);
    const value = decode(reader);
    reader.finish();

    return value;
}

function unsignedBigint(value: number | bigint): bigint {
    const bigint = BigInt(value);
    if (bigint < 0n || bigint > U128_MAX) {
        throw new SerdeError(`unsigned integer out of range: ${value}`);
    }

    return bigint;
}

function signedBigint(value: number | bigint): bigint {
    const bigint = BigInt(value);
    if (bigint < I128_MIN || bigint > I128_MAX) {
        throw new SerdeError(`signed integer out of range: ${value}`);
    }

    return bigint;
}

function safeNumber(value: bigint): number {
    const number = Number(value);
    if (!Number.isSafeInteger(number)) {
        throw new SerdeError(`integer exceeds safe number range: ${value}`);
    }

    return number;
}

function singleCodePoint(value: string): number {
    const codePoints = Array.from(value);
    if (codePoints.length !== 1) {
        throw new SerdeError(`expected one unicode scalar value: ${value}`);
    }

    return codePoints[0].codePointAt(0)!;
}
