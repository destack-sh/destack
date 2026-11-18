import { testSimulation } from "@destack/test/world";
import { expect, test } from "@destack/test";
import { Digest } from "@destack/schema";
import { BuildImage, IMAGE_FORMAT } from "./image.ts";

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

test("digest one image for builds in any order, list its root first, and resolve per build", async () => {
    // describe one image in both orders, each package resolving js-tokens to its own version
    await using simulation = testSimulation();
    const { crypto } = simulation.environment("test");
    const forward: BuildImage = {
        formatVersion: IMAGE_FORMAT,
        root: APP,
        builds: {
            [APP]: { "loose-envify": ENVIFY, tokenizer: TOKENIZER },
            [ENVIFY]: { "js-tokens": TOKENS_4 },
            [TOKENIZER]: { "js-tokens": TOKENS_10 },
            [TOKENS_4]: {},
            [TOKENS_10]: {},
        },
    };
    const backward: BuildImage = {
        ...forward,
        builds: Object.fromEntries(Object.entries(forward.builds).toReversed()),
    };

    // serialize both to one digest, list the root first, and resolve js-tokens per package
    expect([
        (await BuildImage.digest(forward, crypto)) === (await BuildImage.digest(backward, crypto)),
        BuildImage.builds(forward),
        BuildImage.resolve(forward, ENVIFY, "js-tokens"),
        BuildImage.resolve(forward, TOKENIZER, "js-tokens"),
    ]).toEqual([true, [APP, ENVIFY, TOKENIZER, TOKENS_4, TOKENS_10], TOKENS_4, TOKENS_10]);
});

test("refuse an image resolving to a build it does not list", () => {
    // refuse an app resolving js-tokens to an unlisted build
    const parsed = BuildImage.safeParse({
        formatVersion: IMAGE_FORMAT,
        root: APP,
        builds: { [APP]: { "js-tokens": TOKENS_4 } },
    });

    expect(parsed.error?.issues.map((issue) => [issue.path, issue.message])).toEqual([
        [["builds", TOKENS_4], `build '${TOKENS_4}' is missing: the image resolves to it`],
    ]);
});

test("layer an image with a cycle, leaving the edge back into it", () => {
    // image an app linking a package and its peer, which links the package back
    const image = BuildImage.of(APP, {
        [APP]: { "loose-envify": ENVIFY },
        [ENVIFY]: { tokenizer: TOKENIZER },
        [TOKENIZER]: { "loose-envify": ENVIFY },
    });

    expect(BuildImage.layers(image)).toEqual([[TOKENIZER], [ENVIFY], [APP]]);
});
