import { expect, test } from "@destack/test";
import { nth } from "@destack/schema";
import {
    CHUNK_CHARACTERS,
    RICH_TEXT,
    TextRope,
    TextSchema,
    TextSequence,
    TextTypes,
    type TextEdit,
} from "./index.ts";

/** The schema the rope tests resolve marks and nodes with. */
const SCHEMA = TextSchema.of(...RICH_TEXT);

/** The edits each seeded run makes. */
const EDITS = 300;

/** Draw a pseudo-random number from a seed, the same sequence every run. */
function random(seed: { value: number }): number {
    seed.value = (seed.value * 1_103_515_245 + 12_345) % 2_147_483_648;

    return seed.value / 2_147_483_648;
}

/** Draw a whole number below a bound. */
function below(seed: { value: number }, bound: number): number {
    return Math.floor(random(seed) * bound);
}

/** Read what a text shows: its characters, elements and spans. */
function shown(sequence: TextSequence) {
    return {
        text: sequence.text(),
        elements: sequence.runs.flatMap((piece) =>
            TextSequence.isVisible(piece)
                ? Array.from(
                      { length: TextSequence.length(piece) },
                      (_, index) => `${piece.run}:${piece.start + index}`,
                  )
                : [],
        ),
        spans: sequence.spans(SCHEMA),
    };
}

/** Split a sequence's pieces into chunks at random piece cuts. */
function chunked(sequence: TextSequence, seed: { value: number }): TextRope {
    // cut every piece in two at a random element, then group the halves
    const pieces = sequence.runs.flatMap((piece) => {
        const [left, right] = TextSequence.split(piece, below(seed, TextSequence.length(piece)));

        return right === undefined ? [left] : [left, right];
    });
    const chunks: TextSequence[] = [];
    for (let start = 0; start < pieces.length;) {
        const end = start + 1 + below(seed, 3);
        chunks.push(new TextSequence(pieces.slice(start, end)));
        start = end;
    }

    return new TextRope(chunks);
}

test.for([3, 29, 113])(
    "edit a rope exactly as one sequence, deriving the same insertions and mark ranges (seed %i)",
    (value) => {
        const seed = { value };

        // start from a text of several runs, held whole and as chunks
        let sequence = new TextSequence(
            ["the quick ", "brown fox ", "jumps over ", "the lazy dog"].map((text, index) => ({
                run: `base.${index + 1}`,
                start: 0,
                text,
            })),
        );
        let rope = chunked(sequence, seed);
        const clocks = { ada: 0, bo: 0 };
        let order = 0;
        const insertions: {
            readonly whole: TextEdit | undefined;
            readonly split: TextEdit | undefined;
        }[] = [];
        const ranges: { readonly whole: unknown; readonly split: unknown }[] = [];
        for (let step = 0; step < EDITS; step++) {
            // draw an edit: a typed change, a mark, an unmark, a node or a node change
            const length = sequence.text().length;
            const draw = random(seed);
            const site = step % 3 === 0 ? "bo" : "ada";
            const next = { run: site, offset: clocks[site] };
            let edits: TextEdit[];
            // type or delete through the rope, recording both insertions
            if (draw < 0.55 || length < 2) {
                const from = below(seed, length + 1);
                const to = Math.min(length, from + (random(seed) < 0.3 ? below(seed, 6) : 0));
                const insert = random(seed) < 0.8 ? "abcdefgh".slice(0, 1 + below(seed, 3)) : "";
                const change = { from, to, insert };
                edits = rope.change(change, next);
                clocks[site] += insert.length;
                insertions.push({
                    whole: sequence.change(change, next).find((made) => "insert" in made),
                    split: edits.find((made) => "insert" in made),
                });
            }
            // mark a range bold
            else if (draw < 0.7) {
                const from = below(seed, length - 1);
                const to = from + 1 + below(seed, Math.min(6, length - from - 1));
                const range = sequence.range(from, to, TextTypes.bold);
                ranges.push({ whole: range, split: rope.range(from, to, TextTypes.bold) });
                order += 1;
                edits = [
                    { mark: { id: `mark.${step}`, type: "bold", value: true, ...range, order } },
                ];
            }
            // remove a mark
            else if (draw < 0.8 && sequence.marks.length > 0) {
                edits = [{ unmark: [nth(sequence.marks, below(seed, sequence.marks.length))] }];
            }
            // insert a paragraph node
            else if (draw < 0.9 || !sequence.runs.some((piece) => "node" in piece)) {
                const offset = below(seed, length + 1);
                const after = offset === 0 ? undefined : sequence.element(offset - 1);
                edits = [
                    {
                        insert: { type: "paragraph", value: null },
                        run: `node.${step}`,
                        start: 0,
                        ...(after === undefined ? {} : { after }),
                    },
                ];
            }
            // make a node a heading
            else {
                const nodes = sequence.runs.filter((piece) => "node" in piece);
                const node = nth(nodes, below(seed, nodes.length));
                edits = [
                    {
                        set: { run: node.run, offset: node.start },
                        node: { type: "heading", value: { level: 1 + below(seed, 6) } },
                    },
                ];
            }

            // apply the edit to the whole text and to the chunks
            for (const made of edits) {
                sequence = sequence.apply(made);
                rope = rope.apply(made);
            }
        }

        // match the whole sequence's text and insertions
        expect({ shown: shown(rope.read()), length: rope.length }).toEqual({
            shown: shown(sequence),
            length: sequence.text().length,
        });
        expect(insertions.map(({ split }) => split)).toEqual(insertions.map(({ whole }) => whole));
        expect(ranges.map(({ split }) => split)).toEqual(ranges.map(({ whole }) => whole));
    },
);

test("rewrite only the chunks an edit names, leaving the others as they were", () => {
    // hold three chunks of one run, then type after the middle chunk's last element
    const rope = new TextRope([
        new TextSequence([{ run: "a", start: 0, text: "one " }]),
        new TextSequence([{ run: "a", start: 4, text: "two " }]),
        new TextSequence([{ run: "a", start: 8, text: "three" }]),
    ]);
    const edits = rope.change({ from: 8, to: 8, insert: "and " }, { run: "b", offset: 0 });
    const edited = edits.reduce((text, edit) => text.apply(edit), rope);
    expect({
        edits,
        isFirstKept: edited.chunks[0] === rope.chunks[0],
        isLastKept: edited.chunks[2] === rope.chunks[2],
        lengths: edited.lengths,
        text: edited.read().text(),
    }).toEqual({
        edits: [{ insert: "and ", run: "b", start: 0, after: { run: "a", offset: 7 } }],
        isFirstKept: true,
        isLastKept: true,
        lengths: [4, 8, 5],
        text: "one two and three",
    });
});

test("refuse an edit naming an element no chunk holds", () => {
    const rope = new TextRope([new TextSequence([{ run: "a", start: 0, text: "one" }])]);
    expect(() => rope.apply({ delete: [{ run: "b.1", start: 0, text: 1 }] })).toThrow(
        new RangeError("element 'b.1' at offset 0 is invalid: no chunk has it"),
    );
});

test("end chunks between words, whatever pieces they were typed in", () => {
    // type sentences with spaces and without in pieces of seven characters, and lay them out
    const laidOut = ["The margins of the order form. ", "東京都庁、手紙を買う。"].map(
        (sentence) => {
            const text = sentence.repeat(60);
            let sequence = new TextSequence();
            for (let start = 0; start < text.length; start += 7) {
                const after = start === 0 ? {} : { after: { run: `run.${start - 7}`, offset: 6 } };
                sequence = sequence.apply({
                    insert: text.slice(start, start + 7),
                    run: `run.${start}`,
                    start: 0,
                    ...after,
                });
            }
            const chunks = TextRope.layout(sequence, [])
                .flat()
                .map((chunk) => chunk.text());

            return {
                isSame: chunks.join("") === text,
                isBounded: chunks.every((chunk) => chunk.length <= CHUNK_CHARACTERS),
                ends: [...new Set(chunks.map((chunk) => chunk.at(-1)))],
            };
        },
    );

    expect(laidOut).toEqual([
        { isSame: true, isBounded: true, ends: [" "] },
        { isSame: true, isBounded: true, ends: ["。"] },
    ]);
});
