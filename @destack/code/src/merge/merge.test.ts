import { expect, test } from "@destack/test";
import { Source } from "../source/index.ts";
import { Random } from "../test/random.ts";
import { Merge } from "./index.ts";

/** Merge two texts changed from a third. */
function merged(base: string, ours: string, theirs: string): Merge {
    return Merge.of(Source.of(base), Source.of(ours), Source.of(theirs));
}

test("take what one side changed, what both did alike, and conflicts where they differ", () => {
    const base = "one\ntwo\nthree\nfour\nfive\nsix\n";
    const ours = "ONE\ntwo\nthree\nours\nfive\nsix\nseven\n";
    const theirs = "one\ntwo\nthree\ntheirs\nfive\nsix\nseven\n";
    const merge = merged(base, ours, theirs);
    const [conflict] = merge.conflicts;

    expect({
        text: merge.toString({ ours: "fork", base: "1.0", theirs: "2.0" }),
        resolved: (["ours", "theirs", "both", "base", { text: "4\n" }] as const).map(
            (resolution) =>
                conflict === undefined ? "" : merge.resolve(conflict, resolution).toString(),
        ),
    }).toEqual({
        text: "ONE\ntwo\nthree\n<<<<<<< fork\nours\n||||||| 1.0\nfour\n=======\ntheirs\n>>>>>>> 2.0\nfive\nsix\nseven\n",
        resolved: ["ours", "theirs", "ours\ntheirs", "four", "4"].map(
            (line) => `ONE\ntwo\nthree\n${line}\nfive\nsix\nseven\n`,
        ),
    });
});

test("leave the lines both sides share out of a conflict", () => {
    const merge = merged("a\n", "start\nours\nend\n", "start\ntheirs\nend\n");

    expect(merge.chunks.map(({ kind, ours, theirs }) => [kind, ours, theirs])).toEqual([
        ["ours", "start\n", "start\n"],
        ["conflict", "ours\n", "theirs\n"],
        ["ours", "end\n", "end\n"],
    ]);
});

test("take a side that alone changed as it is, and read back the marks it writes", () => {
    // merge random changes, one side at a time and both
    const random = new Random(4);
    const wrong = Array.from({ length: 1000 }, () => {
        const base = random.text(14);
        const [ours, theirs] = [random.changed(base), random.changed(base)];
        const written = merged(base, ours, theirs).toString();
        const isRight =
            merged(base, ours, base).toString() === ours &&
            merged(base, base, theirs).toString() === theirs &&
            merged(base, ours, ours).toString() === ours &&
            Merge.read(Source.of(written)).toString() === written;

        return isRight ? [] : [[base, ours, theirs]];
    }).flat();

    expect(wrong).toEqual([]);
    expect(() => Merge.read(Source.of("<<<<<<< ours\na\n"))).toThrow(
        new TypeError("merged text is invalid: its last conflict does not end"),
    );
});
