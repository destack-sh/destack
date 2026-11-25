import { expect, test } from "@destack/test";
import { TextPacer } from "./pacer.ts";

/** The milliseconds of a frame. */
const FRAME = 16;

/** Let text out frame by frame as pieces arrive at times, reading what came out when. */
function paced(
    pacer: TextPacer,
    pieces: readonly (readonly [time: number, text: string])[],
    finished: number,
): { readonly time: number; readonly text: string }[] {
    const out: { time: number; text: string }[] = [];
    let arrived = "";
    for (let time = 0; time < 5000; time += FRAME) {
        // take what arrived since the last frame
        for (const [at, text] of pieces) {
            if (at > time - FRAME && at <= time) {
                arrived += text;
                pacer.update(arrived);
            }
        }
        if (finished > time - FRAME && finished <= time) {
            pacer.finish();
        }

        // keep what came out
        const before = pacer.text.length;
        const text = pacer.release(time).slice(before);
        if (text !== "") {
            out.push({ time, text });
        }
    }

    return out;
}

test("let out whole words, the first at once and the last once all arrived", () => {
    const out = paced(
        new TextPacer("word"),
        [
            [0, "The order for"],
            [32, "m goes"],
        ],
        96,
    );

    expect(out.map(({ text }) => text).join("")).toBe("The order form goes");
    expect(out[0]).toEqual({ time: 0, text: "The " });
    expect(out.map(({ text }) => text)).toEqual(["The ", "order ", "form ", "goes"]);
});

test("let words out evenly that arrive in bursts, and none long after", () => {
    // send ten words every 400 ms
    const words = Array.from({ length: 80 }, (_, index) => `word${index} `);
    const pieces = Array.from({ length: 8 }, (_, burst): [number, string] => [
        burst * 400,
        words.slice(burst * 10, burst * 10 + 10).join(""),
    ]);
    const out = paced(new TextPacer("word"), pieces, 3200);

    // count the most words of one frame, and how long after its arrival each came out
    let count = 0;
    const delays = out.flatMap(({ time, text }) =>
        text
            .trim()
            .split(" ")
            .map(() => time - Math.floor(count++ / 10) * 400),
    );
    const most = Math.max(...out.map(({ text }) => text.trim().split(" ").length));
    expect({ count, most, longest: Math.max(...delays) }).toEqual({
        count: 80,
        most: 1,
        longest: 336,
    });
});

test("let words out evenly where pieces end within a word", () => {
    // send uneven pieces of a long sentence, as a network does
    const text = Array.from({ length: 120 }, (_, index) => `word${index}`).join(" ");
    const lengths = [24, 6, 60, 15];
    const waits = [120, 30, 280, 60];
    const pieces: [number, string][] = [];
    for (let from = 0, time = 0, turn = 0; from < text.length; turn += 1) {
        const length = lengths[turn % 4] ?? 1;
        time += waits[turn % 4] ?? 1;
        pieces.push([time, text.slice(from, from + length)]);
        from += length;
    }
    const out = paced(new TextPacer("word"), pieces, (pieces.at(-1)?.[0] ?? 0) + FRAME);
    const most = Math.max(...out.map((each) => each.text.trim().split(" ").length));

    expect([out.map((each) => each.text).join("") === text, most]).toEqual([true, 1]);
});

test("keep up with text that arrives faster than anyone reads", () => {
    const words = Array.from({ length: 2000 }, (_, index) => `word${index} `);
    const pieces = words.map((word, index): [number, string] => [index, word]);
    const out = paced(new TextPacer("word"), pieces, 2000);

    expect(out.map(({ text }) => text).join("")).toBe(words.join(""));
    expect((out.at(-1)?.time ?? 0) <= 2300).toBe(true);
});

/** List what a text comes out by. */
function unitsOf(pacer: TextPacer, text: string): string[] {
    return paced(pacer, [[0, text]], 0).map((each) => each.text);
}

test("let text out by characters, lines, and words without spaces", () => {
    expect({
        characters: unitsOf(new TextPacer("character"), "né👍🏽"),
        lines: unitsOf(new TextPacer("line"), "one\ntwo\nthree"),
        words: unitsOf(new TextPacer("word"), "東京都庁です。").join("|").includes("|"),
    }).toEqual({
        characters: ["n", "é", "👍🏽"],
        lines: ["one\n", "two\n", "three"],
        words: true,
    });
});

test("take back what is let out as far as the text changes", () => {
    const pacer = new TextPacer("word");
    pacer.update("one two three ");
    for (let time = 0; time < 400; time += FRAME) {
        pacer.release(time);
    }
    pacer.update("one 2 three ");

    expect([pacer.text, pacer.flush()]).toEqual(["one ", "one 2 three "]);
});
