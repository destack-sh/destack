import { testSimulation } from "@destack/test/world";
import { expect, test } from "@destack/test";
import { Digest } from "@destack/schema";
import { Lock, LOCK_FORMAT } from "./lock.ts";

/** The build of an app. */
const APP = Digest.parse("a".repeat(64));

/** The build of a package requiring js-tokens 4. */
const ENVIFY = Digest.parse("b".repeat(64));

/** The build of a package requiring js-tokens 10. */
const TOKENIZER = Digest.parse("c".repeat(64));

/** The build of js-tokens 4. */
const TOKENS_4 = Digest.parse("d".repeat(64));

/** The build of js-tokens 10. */
const TOKENS_10 = Digest.parse("e".repeat(64));

test("digest one lock for builds in any order, list its root first, and resolve per build", async () => {
    // describe one lock in both orders, each package resolving js-tokens to its own version
    await using simulation = testSimulation();
    const { crypto } = simulation.environment("test");
    const forward: Lock = {
        formatVersion: LOCK_FORMAT,
        root: APP,
        builds: {
            [APP]: { "loose-envify": ENVIFY, tokenizer: TOKENIZER },
            [ENVIFY]: { "js-tokens": TOKENS_4 },
            [TOKENIZER]: { "js-tokens": TOKENS_10 },
            [TOKENS_4]: {},
            [TOKENS_10]: {},
        },
    };
    const backward: Lock = {
        ...forward,
        builds: Object.fromEntries(Object.entries(forward.builds).toReversed()),
    };

    // serialize both to one digest, list the root first, and resolve js-tokens per package
    expect([
        (await Lock.digest(forward, crypto)) === (await Lock.digest(backward, crypto)),
        Lock.builds(forward),
        Lock.resolve(forward, ENVIFY, "js-tokens"),
        Lock.resolve(forward, TOKENIZER, "js-tokens"),
    ]).toEqual([true, [APP, ENVIFY, TOKENIZER, TOKENS_4, TOKENS_10], TOKENS_4, TOKENS_10]);
});

test("refuse a lock resolving to a build it does not list", () => {
    // refuse an app resolving js-tokens to an unlisted build
    const parsed = Lock.safeParse({
        formatVersion: LOCK_FORMAT,
        root: APP,
        builds: { [APP]: { "js-tokens": TOKENS_4 } },
    });

    expect(parsed.error?.issues.map((issue) => [issue.path, issue.message])).toEqual([
        [["builds", TOKENS_4], `build '${TOKENS_4}' is missing: the lock resolves to it`],
    ]);
});

test("layer a lock with a cycle, leaving the edge back into it", () => {
    // lock an app linking a package and its peer, which links the package back
    const lock = Lock.of(APP, {
        [APP]: { "loose-envify": ENVIFY },
        [ENVIFY]: { tokenizer: TOKENIZER },
        [TOKENIZER]: { "loose-envify": ENVIFY },
    });

    expect(Lock.layers(lock)).toEqual([[TOKENIZER], [ENVIFY], [APP]]);
});
