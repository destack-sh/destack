import { Digest } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { Manifest, type ManifestFile } from "./manifest.ts";

/** A file entry of a manifest, at a path. */
function file(path: string): ManifestFile {
    return { path, digest: Digest.parse("a".repeat(64)), size: 1, mode: 0o644 };
}

test.each([
    {
        name: "parent path",
        files: [file("../outside")],
        links: [],
        message: "could not read unsafe manifest path '../outside'",
    },
    {
        name: "absolute path",
        files: [file("/outside")],
        links: [],
        message: "could not read unsafe manifest path '/outside'",
    },
    {
        name: "drive path",
        files: [file("C:/outside")],
        links: [],
        message: "could not read unsafe manifest path 'C:/outside'",
    },
    {
        name: "relative link",
        files: [],
        links: [{ path: "bin/escape", target: "../../outside" }],
        message: "could not read manifest link 'bin/escape': it escapes its directory",
    },
    {
        name: "absolute link",
        files: [],
        links: [{ path: "escape", target: "/outside" }],
        message: "could not read manifest link 'escape': it escapes its directory",
    },
])("refuse a manifest with a $name before writing any file", async ({ files, links, message }) => {
    // parse the signed encoding of an unsafe manifest
    const parsed = refusal(
        Promise.resolve().then(() => Manifest.parse(JSON.stringify({ files, links }))),
    );

    // refuse it as unsafe, naming the offending entry
    expect(await parsed).toEqual(["INTERNAL_SERVER_ERROR", "MANIFEST_BROKEN", message]);
});

test("encode a manifest one way whatever order its entries come in", () => {
    // build one manifest from entries in two orders
    const links = [{ path: "bin/destack", target: "../Destack/destack" }];
    const forward = new Manifest([file("a"), file("b/c")], links);
    const backward = new Manifest([file("b/c"), file("a")], links);

    // encode both the same, sorted by path
    expect([forward.encode(), backward.encode()]).toEqual([
        JSON.stringify({ files: [file("a"), file("b/c")], links }),
        JSON.stringify({ files: [file("a"), file("b/c")], links }),
    ]);
});
