import { expect, test } from "@destack/test";
import { Simulation } from "@destack/environment/simulation";
import { Uuid } from "./uuid.ts";

/** The clock's start: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

/** Make randomness of all-ones bytes, which version and variant bits must overwrite. */
function ones() {
    return {
        bytes: (length: number) => new Uint8Array(length).fill(0xff),
        sequence: Uuid.sequence(),
    };
}

/** Make randomness of all-zero bytes. */
function zeros() {
    return {
        bytes: (length: number) => new Uint8Array(length),
        sequence: Uuid.sequence(),
    };
}

test("lay out a UUIDv7 as the millisecond big-endian, then the version, counter, variant and random bits", () => {
    expect([
        Uuid.v7({ clock: { now: () => START }, random: ones() }),
        Uuid.v7({ clock: { now: () => START + 1 }, random: zeros() }),
        Uuid.v7({ clock: { now: () => 2 ** 48 - 1 }, random: zeros() }),
    ]).toEqual([
        "01a16b35-0800-77ff-bfff-ffffffffffff",
        "01a16b35-0801-7000-8000-000000000000",
        "ffffffff-ffff-7000-8000-000000000000",
    ]);
});

test("order UUIDv7s within a millisecond, after a clock moving back, and past a full counter", () => {
    // draw twice in a millisecond, once after the clock moved back, then at a later one
    const random = ones();
    let now = START;
    const environment = { clock: { now: () => now }, random };
    const counted = [Uuid.v7(environment), Uuid.v7(environment)];
    now = START - 5;
    counted.push(Uuid.v7(environment));
    now = START + 1;
    counted.push(Uuid.v7(environment));

    // draw past the last counter, moving on to the next millisecond
    random.sequence.counter = 0xfff;
    counted.push(Uuid.v7(environment));

    expect(counted).toEqual([
        "01a16b35-0800-77ff-bfff-ffffffffffff",
        "01a16b35-0800-7800-bfff-ffffffffffff",
        "01a16b35-0800-7801-bfff-ffffffffffff",
        "01a16b35-0801-77ff-bfff-ffffffffffff",
        "01a16b35-0802-77ff-bfff-ffffffffffff",
    ]);
});

test("lay out a UUIDv4 as random bits under the version and variant, leaving the drawn bytes alone", () => {
    const drawn = new Uint8Array(16).fill(0xff);

    expect([
        Uuid.v4({ bytes: () => drawn, sequence: Uuid.sequence() }),
        Uuid.v4(zeros()),
        drawn,
    ]).toEqual([
        "ffffffff-ffff-4fff-bfff-ffffffffffff",
        "00000000-0000-4000-8000-000000000000",
        new Uint8Array(16).fill(0xff),
    ]);
});

test("refuse a time outside the 48-bit timestamp, and a draw of the wrong length", () => {
    // create at a time, or from a short draw, reading the range error
    const at = (now: number) => () => Uuid.v7({ clock: { now: () => now }, random: zeros() });
    const short = {
        bytes: (length: number) => new Uint8Array(length - 1),
        sequence: Uuid.sequence(),
    };
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

test("hash www.example.com in the DNS namespace into RFC 9562's UUIDv8, and another name differently", async () => {
    // hash the namespace's bytes and the name with SHA-256 (RFC 9562 B.2)
    await using simulation = new Simulation({ seed: 1, start: START });
    const { crypto } = simulation.environment("schema");
    const dns = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";

    expect([
        await Uuid.v8(dns, "www.example.com", crypto),
        await Uuid.v8(dns, "www.example.org", crypto),
    ]).toEqual(["5c146b14-3c52-8afd-938a-375d0df1fbf6", "d44a3010-8e6d-8bfb-ae29-59e0267f6516"]);
});

test("refuse a namespace that is not a lowercase hyphenated UUID", async () => {
    await using simulation = new Simulation({ seed: 1, start: START });
    const { crypto } = simulation.environment("schema");

    await expect(Uuid.v8("6BA7B810-9DAD-11D1-80B4-00C04FD430C8", "x", crypto)).rejects.toThrow(
        new RangeError(
            "uuid namespace 6BA7B810-9DAD-11D1-80B4-00C04FD430C8 is no lowercase hyphenated uuid",
        ),
    );
});
