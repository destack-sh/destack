import { expect, test } from "@destack/test";
import { BlobPack } from "./pack.ts";

/** The size of a body large enough for a pack of its own: 1 MiB. */
const LARGE = 1024 * 1024;

test("group bodies in order, each large body alone and each run of small ones together", () => {
    // group small, small, large, small, unsized, large, large bodies by their sizes
    const sizes = [5, 7, LARGE, 3, undefined, LARGE + 1, LARGE];
    const groups = BlobPack.groups(sizes.map((size) => (size === undefined ? {} : { size })));

    // keep the order, split runs at each large body, and count an unsized body as small
    expect(groups.map((group) => group.map((body) => body.size ?? null))).toEqual([
        [5, 7],
        [LARGE],
        [3, null],
        [LARGE + 1],
        [LARGE],
    ]);
});
