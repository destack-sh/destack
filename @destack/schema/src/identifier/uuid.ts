/** The bytes of a UUID (RFC 9562 4). */
const UUID_BYTES = 16;

/** The bytes of a UUIDv7's millisecond timestamp, 48 bits (RFC 9562 5.7). */
const TIMESTAMP_BYTES = 6;

/** The milliseconds a UUIDv7's 48-bit timestamp has room for. */
const TIMESTAMP_LIMIT = 2 ** 48;

/** The clock a UUIDv7 orders by. */
export interface UuidClock {
    /** Read the time, in UTC epoch milliseconds. */
    readonly now: () => number;
}

/** The randomness a UUID draws from. */
export interface UuidRandom {
    /** Draw a number of random bytes. */
    readonly bytes: (length: number) => Uint8Array;
}

/** The effects a UUIDv7 is created from: the time it orders by, and the randomness it draws. */
export interface UuidEnvironment {
    /** The clock. */
    readonly clock: UuidClock;
    /** The randomness. */
    readonly random: UuidRandom;
}

/** A UUID in its hyphenated lowercase text form. */
export type Uuid = `${string}-${string}-${string}-${string}-${string}`;

/** UUIDs drawn from an environment (RFC 9562). */
export const Uuid = {
    /** Draw a UUIDv4 of 122 random bits (RFC 9562 5.4). */
    v4(random: UuidRandom): Uuid {
        // draw every bit
        const bytes = new Uint8Array(UUID_BYTES);
        bytes.set(draw(random, UUID_BYTES));

        return print(bytes, 0x40);
    },

    /** Draw a UUIDv7 of the millisecond and 74 random bits, unordered within it (RFC 9562 5.7). */
    v7(environment: UuidEnvironment): Uuid {
        // refuse a time outside the 48-bit timestamp
        const now = environment.clock.now();
        if (!Number.isSafeInteger(now) || now < 0 || now >= TIMESTAMP_LIMIT) {
            throw new RangeError(`uuid time ${now} is outside 0 to 2^48 milliseconds`);
        }

        // write the millisecond big-endian over the first 48 bits
        const bytes = new Uint8Array(UUID_BYTES);
        const view = new DataView(bytes.buffer);
        view.setUint16(0, Math.floor(now / 2 ** 32));
        view.setUint32(2, now % 2 ** 32);

        // draw the 80 bits after the millisecond
        bytes.set(draw(environment.random, UUID_BYTES - TIMESTAMP_BYTES), TIMESTAMP_BYTES);

        return print(bytes, 0x70);
    },
};

/** Draw random bytes, refusing a draw of another length. */
function draw(random: UuidRandom, length: number): Uint8Array {
    const drawn = random.bytes(length);
    if (drawn.length !== length) {
        throw new RangeError(`random bytes are ${drawn.length}, not ${length}`);
    }

    return drawn;
}

/** Mark a UUID's bytes with a version and the RFC 9562 variant, and print their hyphenated form. */
function print(bytes: Uint8Array<ArrayBuffer>, version: number): Uuid {
    // set the version and variant bits (RFC 9562 4.1, 4.2)
    const view = new DataView(bytes.buffer);
    view.setUint8(6, version | (view.getUint8(6) & 0x0f));
    view.setUint8(8, 0x80 | (view.getUint8(8) & 0x3f));

    // print the hyphenated form
    const hex = bytes.toHex();

    return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}
