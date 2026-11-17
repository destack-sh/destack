import { expect, test } from "@destack/test";
import { readDigest } from "./executable.ts";

test("read a file's digest from a release's listing, and none for a file it does not list", () => {
    // list two runtimes as Bun's releases do: a digest, two spaces, a file name
    const listing = [
        "c240d7f0ccb90c263a849fcdb09b7ee62630d921348ed6aafda86b9d577bc510  bun-darwin-aarch64.zip",
        "50a3491f73abc2fe81673171a85762d3bc2cc1680ccecadd983de99adf77977e  bun-darwin-x64.zip",
        "",
    ].join("\n");

    // read each listed file's own digest, and none for a file whose name another ends with
    expect([
        readDigest(listing, "bun-darwin-x64.zip"),
        readDigest(listing, "bun-darwin-aarch64.zip"),
        readDigest(listing, "x64.zip"),
        readDigest(listing, "bun-linux-x64.zip"),
    ]).toEqual([
        "50a3491f73abc2fe81673171a85762d3bc2cc1680ccecadd983de99adf77977e",
        "c240d7f0ccb90c263a849fcdb09b7ee62630d921348ed6aafda86b9d577bc510",
        undefined,
        undefined,
    ]);
});
