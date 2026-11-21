import { expect, test } from "@destack/test";
import { Digest } from "@destack/schema";
import { BuildLayer, BuildLayerTable, type BuildLayerFile } from "./layer.ts";

/** Draw a digest from a seed, as a layer keys its files. */
function digestOf(seed: number): Digest {
    // spread the seed over 32 bytes by a multiplicative hash
    const bytes = new Uint8Array(32);
    let state = seed * 2_654_435_761 + 1;
    for (let index = 0; index < bytes.length; index += 1) {
        state = (state * 1_103_515_245 + 12_345) % 2_147_483_648;
        bytes[index] = state % 256;
    }

    return Digest.parse(bytes.toHex());
}

test("find every file of a linked table by its digest, and none outside it", () => {
    // write a thousand files in shuffled order, with multibyte paths
    const placed: BuildLayerFile[] = Array.from({ length: 1000 }, (_, index) => ({
        digest: digestOf(index),
        size: index * 13,
        addressing: "path",
        path: `build-${String(index % 7)}/src/ünïcode/${String(index)}.js`,
    }));
    const shuffled = placed.toSorted((left, right) => (left.size % 97) - (right.size % 97));
    const files = new BuildLayerTable(BuildLayerTable.encode(shuffled), "path");

    // expect each file's path and size back, and nothing for digests outside the table
    const found = placed.map((file) => files.find(file.digest));
    const outside = [1000, 1001, 1002].map((seed) => files.find(digestOf(seed)));
    expect({ found, outside }).toEqual({
        found: placed,
        outside: [undefined, undefined, undefined],
    });
});

test("find files at their offsets in a packed table, recording no paths", () => {
    // write a packed table of three files laid end to end
    const placed: BuildLayerFile[] = [0, 1, 2].map((seed) => ({
        digest: digestOf(seed),
        size: seed + 1,
        addressing: "pack",
        offset: seed * 100,
    }));
    const bytes = BuildLayerTable.encode(placed);
    const files = new BuildLayerTable(bytes, "pack");

    // expect the count and the rows alone, and each file at its offset
    expect({
        bytes: bytes.byteLength,
        found: placed.map((file) => files.find(file.digest)),
        empty: new BuildLayerTable(new Uint8Array(), "path").find(digestOf(0)),
    }).toEqual({
        bytes: 4 + 3 * 40,
        found: placed,
        empty: undefined,
    });
});

test("lay groups out into packs, a group's files together and a new pack at the limit", () => {
    // lay out two groups under a limit of ten bytes: the first fills two packs
    const file = (seed: number, size: number) => ({ digest: digestOf(seed), size });
    const packs = BuildLayer.pack([[file(1, 6), file(2, 6), file(3, 2)], [file(4, 1)], []], 10);

    expect(packs).toEqual([
        [{ digest: digestOf(1), size: 6, addressing: "pack", offset: 0 }],
        [
            { digest: digestOf(2), size: 6, addressing: "pack", offset: 6 },
            { digest: digestOf(3), size: 2, addressing: "pack", offset: 12 },
        ],
        [{ digest: digestOf(4), size: 1, addressing: "pack", offset: 14 }],
    ]);
});
