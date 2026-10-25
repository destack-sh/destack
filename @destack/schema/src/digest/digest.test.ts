import { testSimulation } from "@destack/test/world";
import { expect, test } from "@destack/test";
import { Digest } from "./digest.ts";

test("hash text, bytes and JSON values, whatever their key order", async () => {
    // hash text and its UTF-8 bytes alike
    await using simulation = testSimulation();
    const crypto = simulation.environment("schema").crypto;
    expect(await Digest.of("destack", crypto)).toBe(
        await Digest.of(new TextEncoder().encode("destack"), crypto),
    );

    // hash a stream of chunks as its whole bytes
    expect(await Digest.of(chunks(), crypto)).toBe(await Digest.of("destack", crypto));

    // hash JSON values by their canonical form
    expect(await Digest.json({ first: 1, second: 2 }, crypto)).toBe(
        await Digest.json({ second: 2, first: 1 }, crypto),
    );
    expect(await Digest.json({ first: 1 }, crypto)).toBe(
        "774f958deec5b02dd6d95db6d282d75d329e49d0c997f40fae3b51eae9b8614e",
    );

    // accept only lowercase hexadecimal SHA-256
    expect(
        [await Digest.of("", crypto), "A".repeat(64), "a".repeat(63)].map(
            (value) => Digest.safeParse(value).success,
        ),
    ).toEqual([true, false, false]);
});

/** Stream "destack" in two chunks. */
async function* chunks(): AsyncGenerator<Uint8Array> {
    yield new TextEncoder().encode("dest");
    yield new TextEncoder().encode("ack");
}
