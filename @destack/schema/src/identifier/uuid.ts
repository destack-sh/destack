import type { DigestCrypto } from "../digest/digest.ts";

/** The bytes of a UUID (RFC 9562 4). */
const UUID_BYTES = 16;

/** The bytes of a UUIDv7's millisecond timestamp, 48 bits (RFC 9562 5.7). */
const TIMESTAMP_BYTES = 6;

/** The milliseconds a UUIDv7's 48-bit timestamp has room for. */
const TIMESTAMP_LIMIT = 2 ** 48;

/** A UUID's canonical lowercase hyphenated text form (RFC 9562 4). */
const UUID_TEXT = /^[0-9a-f]{8}-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}$/u;

/** The largest UUIDv7 counter, 12 bits in rand_a (RFC 9562 6.2, method 1). */
const COUNTER_LIMIT = 0xfff;

/** The largest counter a millisecond starts at, its top bit clear (RFC 9562 6.2). */
const COUNTER_START_LIMIT = 0x7ff;

/** The clock a UUIDv7 orders by. */
export interface UuidClock {
    /** Read the time, in UTC epoch milliseconds. */
    readonly now: () => number;
}

/** The randomness a UUID draws from, with the sequence its UUIDv7s count in. */
export interface UuidRandom {
    /** Draw a number of random bytes. */
    readonly bytes: (length: number) => Uint8Array;
    /** The last UUIDv7 the randomness drew. */
    readonly sequence: UuidSequence;
}

/** The last UUIDv7 a source drew, which the next one in its millisecond counts past. */
export interface UuidSequence {
    /** The millisecond of the last UUIDv7, -1 before the first. */
    millisecond: number;
    /** The counter of the last UUIDv7 within its millisecond. */
    counter: number;
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
    /** Start a source's UUIDv7 sequence, before its first draw. */
    sequence(): UuidSequence {
        return { millisecond: -1, counter: 0 };
    },

    /** Draw a UUIDv4 of 122 random bits (RFC 9562 5.4). */
    v4(random: UuidRandom): Uuid {
        // draw every bit
        const bytes = new Uint8Array(UUID_BYTES);
        bytes.set(draw(random, UUID_BYTES));

        return print(bytes, 0x40);
    },

    /** Draw a UUIDv7 ordered after the source's last one, advancing its sequence (RFC 9562 6.2). */
    v7(environment: UuidEnvironment): Uuid {
        // refuse a time outside the 48-bit timestamp
        const now = environment.clock.now();
        if (!Number.isSafeInteger(now) || now < 0 || now >= TIMESTAMP_LIMIT) {
            throw new RangeError(`uuid time ${now} is outside 0 to 2^48 milliseconds`);
        }

        // count past the last UUIDv7 in its millisecond, moving to the next one once the counter runs out
        const { random } = environment;
        const { sequence } = random;
        const bytes = new Uint8Array(UUID_BYTES);
        const view = new DataView(bytes.buffer);
        bytes.set(draw(random, UUID_BYTES - TIMESTAMP_BYTES), TIMESTAMP_BYTES);
        const isCounting = now <= sequence.millisecond && sequence.counter < COUNTER_LIMIT;
        if (isCounting) {
            sequence.counter += 1;
        }
        // start a later millisecond's counter at random, below half its range
        else {
            sequence.millisecond = Math.max(now, sequence.millisecond + 1);
            sequence.counter = view.getUint16(6) & COUNTER_START_LIMIT;
        }

        // refuse a sequence past the 48-bit timestamp
        if (sequence.millisecond >= TIMESTAMP_LIMIT) {
            throw new RangeError("uuid sequence ran past 2^48 milliseconds");
        }

        // write the millisecond big-endian over the first 48 bits, then the counter in rand_a
        view.setUint16(0, Math.floor(sequence.millisecond / 2 ** 32));
        view.setUint32(2, sequence.millisecond % 2 ** 32);
        view.setUint16(6, sequence.counter);

        return print(bytes, 0x70);
    },

    /** Hash a namespace's bytes and a name into a UUIDv8 by SHA-256 (RFC 9562 B.2). */
    async v8(namespace: Uuid, name: string, crypto: DigestCrypto): Promise<Uuid> {
        // refuse a namespace in another UUID form
        if (!UUID_TEXT.test(namespace)) {
            throw new RangeError(`uuid namespace ${namespace} is no lowercase hyphenated uuid`);
        }

        // hash the namespace's 16 bytes, then the name's UTF-8 bytes
        const encoded = new TextEncoder().encode(name);
        const input = new Uint8Array(UUID_BYTES + encoded.length);
        input.set(Uint8Array.fromHex(namespace.replaceAll("-", "")));
        input.set(encoded, UUID_BYTES);
        const digest = new Uint8Array(await crypto.digest("SHA-256", input));

        // keep its first 128 bits under the version and variant
        return print(digest.slice(0, UUID_BYTES), 0x80);
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
