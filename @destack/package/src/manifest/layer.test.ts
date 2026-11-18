import { expect, test } from "@destack/test";
import { Digest } from "@destack/schema";
import { LAYER_BLOBS, LayerFiles, type LayerFile } from "./layer.ts";

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
    const placed: LayerFile[] = Array.from({ length: 1000 }, (_, index) => ({
        digest: digestOf(index),
        path: `build-${String(index % 7)}/src/ünïcode/${String(index)}.js`,
        size: index * 13,
    }));
    const shuffled = placed.toSorted((left, right) => (left.size % 97) - (right.size % 97));
    const files = new LayerFiles(LayerFiles.encode(shuffled, "path"), "path");

    // expect each file's path and size back, and nothing for digests outside the table
    const found = placed.map((file) => files.find(file.digest));
    const outside = [1000, 1001, 1002].map((seed) => files.find(digestOf(seed)));
    expect({ found, outside }).toEqual({
        found: placed.map(({ path, size }) => ({ path, size })),
        outside: [undefined, undefined, undefined],
    });
});

test("find files at their blobs in a table addressed by digest, recording no paths", () => {
    // write an addressed table, whose paths the layer derives
    const placed: LayerFile[] = [0, 1, 2].map((seed) => ({
        digest: digestOf(seed),
        path: `${LAYER_BLOBS}/${digestOf(seed)}`,
        size: seed + 1,
    }));
    const bytes = LayerFiles.encode(placed, "digest");
    const files = new LayerFiles(bytes, "digest");

    // expect the count and the rows alone, and each file at its blob
    expect({
        bytes: bytes.byteLength,
        found: placed.map((file) => files.find(file.digest)),
        empty: new LayerFiles(new Uint8Array(), "path").find(digestOf(0)),
    }).toEqual({
        bytes: 4 + 3 * 40,
        found: placed.map(({ path, size }) => ({ path, size })),
        empty: undefined,
    });
});
