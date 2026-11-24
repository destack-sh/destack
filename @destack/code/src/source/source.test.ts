import { expect, test } from "@destack/test";
import { Random } from "../test/random.ts";
import { Source } from "./index.ts";

test("count lines as text has them, and find them by number and offset", () => {
    const source = Source.of("one\n\nthree\n");

    expect({
        counts: ["", "a", "a\n", "a\nb", "\n\n"].map((text) => Source.of(text).lines),
        lines: [0, 1, 2, 3].map((line) => source.line(line)),
        ranges: [0, 3].map((line) => source.range(line)),
        at: [0, 3, 4, 5, 9, 11].map((offset) => source.lineAt(offset)),
    }).toEqual({
        counts: [0, 1, 1, 2, 2],
        lines: ["one", "", "three", ""],
        ranges: [
            { from: 0, to: 3 },
            { from: 11, to: 11 },
        ],
        at: [0, 0, 1, 2, 2, 3],
    });
});

test("edit a text as if it was read again, refusing places outside it", () => {
    // replace random ranges of random texts
    const random = new Random(1);
    const wrong = Array.from({ length: 500 }, () => {
        const text = random.text(12);
        const from = random.below(text.length + 1);
        const to = from + random.below(text.length - from + 1);
        const insert = random.text(4);
        const edited = Source.of(text).edit({ from, to, insert });
        const read = Source.of(text.slice(0, from) + insert + text.slice(to));

        return edited.text === read.text && edited.starts.join() === read.starts.join();
    }).filter((isRight) => !isRight);

    expect(wrong).toEqual([]);
    expect(
        Source.of("one two three").edit([
            { from: 8, to: 13, insert: "3\n" },
            { from: 0, to: 3, insert: "1" },
        ]),
    ).toEqual(Source.of("1 two 3\n"));
    expect(() => Source.of("a").edit({ from: 0, to: 2, insert: "" })).toThrow(
        new RangeError("edit is invalid: it is outside the text or over another"),
    );
});
