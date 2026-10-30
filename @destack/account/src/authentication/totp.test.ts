import { testSimulation } from "@destack/object/test";
import { expect, refusal, test } from "@destack/test";
import { Totp } from "./totp.ts";

/** The SHA-1 seed of RFC 6238 Appendix B, the ASCII digits 1 to 0 twice. */
const SEED = new TextEncoder().encode("12345678901234567890");

/** The SHA-1 rows of RFC 6238 Appendix B: the moment in seconds and its eight-digit code. */
const VECTORS = [
    [59, "94287082"],
    [1_111_111_109, "07081804"],
    [1_111_111_111, "14050471"],
    [1_234_567_890, "89005924"],
    [2_000_000_000, "69279037"],
    [20_000_000_000, "65353130"],
] as const;

test("generate RFC 6238 Appendix B's SHA-1 codes at each of its moments", async () => {
    await using simulation = testSimulation();
    const { crypto } = simulation.environment("totp");
    const totp = await Totp.of(SEED, crypto, 8);
    const codes = await Promise.all(
        VECTORS.map(async ([seconds]) => [
            seconds,
            await totp.code(Totp.step(seconds * 1000), crypto),
        ]),
    );

    expect(codes).toEqual(VECTORS.map(([seconds, code]) => [seconds, code]));
});

test("accept a code a step either side of now once, refusing an expired code and a reused step", async () => {
    // generate the codes of the steps around a moment, paused there
    await using simulation = testSimulation();
    const environment = simulation.environment("totp");
    const { clock, crypto } = environment;
    const totp = await Totp.of(SEED, crypto);
    const now = Totp.step(clock.now());
    const before = await totp.code(now - 1, crypto);
    const current = await totp.code(now, crypto);
    const after = await totp.code(now + 1, crypto);
    const expired = await totp.code(now - 2, crypto);

    // accept the previous step, then the current, then refuse both again, an expired code and a reused step
    const accepted = [
        await totp.verify(before, 0, environment),
        await totp.verify(current, now - 1, environment),
    ];
    const refused = [
        await refusal(totp.verify(expired, 0, environment)),
        await refusal(totp.verify(current, now, environment)),
        await refusal(totp.verify(before, now, environment)),
    ];
    const next = await totp.verify(after, now, environment);

    expect([accepted, refused, next]).toEqual([
        [now - 1, now],
        [
            ["UNAUTHORIZED", "CODE_INVALID", "the code is wrong or expired"],
            ["UNAUTHORIZED", "CODE_REUSED", "the code was already used"],
            ["UNAUTHORIZED", "CODE_REUSED", "the code was already used"],
        ],
        now + 1,
    ]);
});
