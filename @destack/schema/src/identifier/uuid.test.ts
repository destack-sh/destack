import { expect, test } from "@destack/test";
import { Uuid } from "./uuid.ts";

/** The clock's start: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

/** Randomness of all-ones bytes, which version and variant bits must overwrite. */
const ONES = { bytes: (length: number) => new Uint8Array(length).fill(0xff) };

/** Randomness of all-zero bytes. */
const ZEROS = { bytes: (length: number) => new Uint8Array(length) };

test("lay out a UUIDv7 as the millisecond big-endian, then the version, variant and random bits", () => {
    expect([
        Uuid.v7({ clock: { now: () => START }, random: ONES }),
        Uuid.v7({ clock: { now: () => START + 1 }, random: ZEROS }),
        Uuid.v7({ clock: { now: () => 2 ** 48 - 1 }, random: ZEROS }),
    ]).toEqual([
        "01a16b35-0800-7fff-bfff-ffffffffffff",
        "01a16b35-0801-7000-8000-000000000000",
        "ffffffff-ffff-7000-8000-000000000000",
    ]);
});

test("lay out a UUIDv4 as random bits under the version and variant, leaving the drawn bytes alone", () => {
    const drawn = new Uint8Array(16).fill(0xff);

    expect([Uuid.v4({ bytes: () => drawn }), Uuid.v4(ZEROS), drawn]).toEqual([
        "ffffffff-ffff-4fff-bfff-ffffffffffff",
        "00000000-0000-4000-8000-000000000000",
        new Uint8Array(16).fill(0xff),
    ]);
});

test("refuse a time outside the 48-bit timestamp, and a draw of the wrong length", () => {
    // create at a time, or from a short draw, reading the range error
    const at = (now: number) => () => Uuid.v7({ clock: { now: () => now }, random: ZEROS });
    const short = { bytes: (length: number) => new Uint8Array(length - 1) };
    const refused = [at(-1), at(2 ** 48), at(START + 0.5), () => Uuid.v4(short)].map((create) => {
        try {
            return create();
        } catch (error) {
            return error instanceof RangeError ? error.message : "other";
        }
    });

    expect(refused).toEqual([
        "uuid time -1 is outside 0 to 2^48 milliseconds",
        "uuid time 281474976710656 is outside 0 to 2^48 milliseconds",
        `uuid time ${START + 0.5} is outside 0 to 2^48 milliseconds`,
        "random bytes are 15, not 16",
    ]);
});
