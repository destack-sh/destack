import { expect, test } from "@destack/test";
import { aligned, present } from "@destack/schema";
import { Sequence, TextChange, type Annotation, type Element, type SequenceEdit } from "./index.ts";

/** The edits each seeded run makes. */
const EDITS = 400;

/** One party editing a copy: the server's sequence it last saw and its edits not yet pushed. */
interface Party {
    /** The party's site, prefixing its runs. */
    readonly site: string;
    /** The runs the party has typed. */
    counter: number;
    /** The server's sequence the party last saw. */
    seen: Sequence;
    /** The party's edits not yet pushed, in order. */
    pending: SequenceEdit[];
}

/** Draw a pseudo-random number from a seed, the same sequence every run. */
function random(seed: { value: number }): number {
    seed.value = (seed.value * 1_103_515_245 + 12_345) % 2_147_483_648;

    return seed.value / 2_147_483_648;
}

/** Pick a random insertion or deletion on a party's copy. */
function edit(
    sequence: Sequence,
    seed: { value: number },
    site: string,
    counter: number,
): SequenceEdit {
    // delete a range of the visible text now and then
    const length = sequence.text().length;
    if (length > 2 && random(seed) < 0.3) {
        const from = Math.floor(random(seed) * (length - 1));
        const to = Math.min(length - 1, from + Math.floor(random(seed) * 5));

        return {
            delete: {
                from: present(sequence.element(from), `the element at ${from}`),
                to: present(sequence.element(to), `the element at ${to}`),
            },
        };
    }

    // type a few characters after a visible element, or at the start
    const offset = Math.floor(random(seed) * (length + 1));
    const after = offset === 0 ? undefined : sequence.element(offset - 1);
    const text = "abcdefghij".slice(0, 1 + Math.floor(random(seed) * 4));

    return { insert: text, run: `${site}.${counter}`, ...(after === undefined ? {} : { after }) };
}

/** List the visible elements of a sequence as `run:offset`. */
function visible(sequence: Sequence): string[] {
    return sequence.runs.flatMap((piece) =>
        typeof piece.text === "number"
            ? []
            : Array.from(
                  { length: piece.text.length },
                  (_, index) => `${piece.run}:${piece.start + index}`,
              ),
    );
}

/** Read a party's copy: the server's sequence it saw with its pending edits over it. */
function copy(party: Party): Sequence {
    return party.pending.reduce((sequence, next) => sequence.apply(next), party.seen);
}

/** Name an element of the base run. */
function element(offset: number): Element {
    return { run: "base.1", offset };
}

/** Name an element of the first typed run. */
function at(offset: number): Element {
    return { run: "a.1", offset };
}

test.for([3, 17, 91])(
    "converge every party's predicted copy on the server's order of their edits (seed %i)",
    (value) => {
        const seed = { value };
        const sites = ["a", "b", "c"];

        // keep the server's sequence and each party's pending edits
        let server = new Sequence();
        const parties = sites.map((site): Party => ({
            site,
            counter: 0,
            seen: server,
            pending: [],
        }));

        // edit, push and follow in turns
        for (let step = 0; step < EDITS; step++) {
            const party = aligned(parties, Math.floor(random(seed) * parties.length));
            const draw = random(seed);
            if (draw < 0.7) {
                party.counter += 1;
                party.pending.push(edit(copy(party), seed, party.site, party.counter));
            } else if (draw < 0.9) {
                server = party.pending.reduce((sequence, next) => sequence.apply(next), server);
                party.pending = [];
                party.seen = server;
            } else {
                party.seen = server;
            }
        }

        // push what is left and follow, finding every copy equal to the server's
        for (const party of parties) {
            server = party.pending.reduce((sequence, next) => sequence.apply(next), server);
            party.pending = [];
        }
        const copies = parties.map((party) => {
            party.seen = server;

            return copy(party).text();
        });
        expect(copies).toEqual(sites.map(() => server.text()));
    },
);

test.for([5, 23, 77])(
    "undo every edit exactly in reverse order, restoring the text and its elements (seed %i)",
    (value) => {
        const seed = { value };

        // edit a text as two parties, recording each edit
        let sequence = new Sequence().apply({ insert: "the quick brown fox", run: "base.1" });
        const original = { text: sequence.text(), elements: visible(sequence) };
        const applied: { readonly edit: SequenceEdit; readonly before: Sequence }[] = [];
        for (let step = 0; step < EDITS / 4; step++) {
            const next = edit(sequence, seed, step % 2 === 0 ? "a" : "b", step);
            applied.push({ edit: next, before: sequence });
            sequence = sequence.apply(next);
        }

        // undo each edit through its inverse, the latest first
        for (const { edit: undone, before } of applied.toReversed()) {
            sequence = Sequence.inverse(undone, before).reduce(
                (restored, next) => restored.apply(next),
                sequence,
            );
        }
        expect({ text: sequence.text(), elements: visible(sequence) }).toEqual(original);
    },
);

test("restore only the elements a deletion hid, leaving another party's earlier deletion in place", () => {
    // delete "quick " as one party and "the quick brown" as another, undoing only the second
    const base = new Sequence().apply({ insert: "the quick brown fox", run: "base.1" });
    const first = base.apply({ delete: { from: element(4), to: element(9) } });
    const second = { delete: { from: element(0), to: element(14) } } as const;
    const undone = Sequence.inverse(second, first).reduce(
        (sequence, next) => sequence.apply(next),
        first.apply(second),
    );
    expect([first.apply(second).text(), undone.text()]).toEqual([" fox", "the brown fox"]);
});

test("find elements by visible offset and offsets by element, across tombstones", () => {
    // type, delete the middle, and read positions on both sides of the tombstone
    const sequence = new Sequence()
        .apply({ insert: "hello world", run: "a.1" })
        .apply({ delete: { from: { run: "a.1", offset: 5 }, to: { run: "a.1", offset: 5 } } });
    expect([
        sequence.text(),
        sequence.element(5),
        sequence.offset({ run: "a.1", offset: 6 }),
        sequence.offset({ run: "a.1", offset: 5 }),
        sequence.element(10),
    ]).toEqual(["helloworld", { run: "a.1", offset: 6 }, 5, 5, undefined]);
});

test("resolve annotations over edits by their boundaries: bold grows at its end, a link does not, neither grows at its start", () => {
    // write "hello world" with a bold "hello" and a linked "world"
    const text = new Sequence().apply({ insert: "hello world", run: "a.1" });
    const bold: Annotation = {
        id: "bold",
        kind: "bold",
        value: true,
        start: { element: at(0), side: "before" },
        end: { element: at(5), side: "before" },
    };
    const link: Annotation = {
        id: "link",
        kind: "link",
        value: "https://destack.sh",
        start: { element: at(6), side: "before" },
        end: { element: at(10), side: "after" },
    };

    // type at both ends of each range and delete inside the bold range
    const edited = text
        .apply({ insert: "!", run: "b.1", after: at(4) })
        .apply({ insert: "?", run: "b.2", after: at(10) })
        .apply({ insert: "¿", run: "b.3", after: at(5) })
        .apply({ delete: { from: at(1), to: at(2) } });
    const spans = edited
        .spans([bold, link])
        .map((span) => [span.text, span.annotations.map((annotation) => annotation.kind)]);
    expect(spans).toEqual([
        ["hlo!", ["bold"]],
        [" ¿", []],
        ["world", ["link"]],
        ["?", []],
    ]);
});

test("cover overlapping annotations in one stretch each, keeping boundaries on deleted elements in place", () => {
    // annotate overlapping ranges and delete the comment's first element
    const text = new Sequence().apply({ insert: "the quick brown fox", run: "a.1" });
    const comment: Annotation = {
        id: "comment",
        kind: "comment",
        value: "check",
        start: { element: at(4), side: "before" },
        end: { element: at(14), side: "after" },
    };
    const italic: Annotation = {
        id: "italic",
        kind: "italic",
        value: true,
        start: { element: at(10), side: "before" },
        end: { element: at(18), side: "after" },
    };
    const spans = text
        .apply({ delete: { from: at(4), to: at(4) } })
        .spans([comment, italic])
        .map((span) => [span.text, span.annotations.map((annotation) => annotation.kind)]);
    expect(spans).toEqual([
        ["the ", []],
        ["uick ", ["comment"]],
        ["brown", ["comment", "italic"]],
        [" fox", ["italic"]],
    ]);
});

test.for([7, 41])(
    "keep an editor's plain text in step with the sequence through changes both ways (seed %i)",
    (value) => {
        const seed = { value };

        // mirror random edits between the sequence and a plain editor text
        let sequence = new Sequence();
        let editor = "";
        const applied: { readonly edit: SequenceEdit; readonly before: Sequence }[] = [];
        for (let step = 0; step < EDITS / 2; step++) {
            // apply a remote edit as offset changes
            if (step % 2 === 0 || editor.length === 0) {
                const remote = edit(sequence, seed, "remote", step);
                for (const change of sequence.changesOf(remote)) {
                    editor = editor.slice(0, change.from) + change.insert + editor.slice(change.to);
                }
                applied.push({ edit: remote, before: sequence });
                sequence = sequence.apply(remote);
            }
            // apply a local editor change as edits
            else {
                const from = Math.floor(random(seed) * editor.length);
                const to = Math.min(editor.length, from + Math.floor(random(seed) * 3));
                const insert = random(seed) < 0.5 ? "xy" : "";
                const local = sequence.change({ from, to, insert }, `local.${step}`);
                editor = editor.slice(0, from) + insert + editor.slice(to);
                for (const next of local) {
                    applied.push({ edit: next, before: sequence });
                    sequence = sequence.apply(next);
                }
            }
            expect(editor).toBe(sequence.text());
        }

        // undo everything, the editor following each inverse through its offset changes
        for (const { edit: undone, before } of applied.toReversed()) {
            for (const inverse of Sequence.inverse(undone, before)) {
                for (const change of sequence.changesOf(inverse)) {
                    editor = editor.slice(0, change.from) + change.insert + editor.slice(change.to);
                }
                sequence = sequence.apply(inverse);
            }
        }
        expect([editor, sequence.text()]).toEqual(["", ""]);
    },
);

test("keep a deletion's length alone, and restore exactly the characters it removed", () => {
    // delete the middle of a text and keep only its length
    const typed = new Sequence().apply({ insert: "abcdef", run: "a.1" });
    const deletion = {
        delete: { from: { run: "a.1", offset: 1 }, to: { run: "a.1", offset: 3 } },
    } as const;
    const deleted = typed.apply(deletion);
    expect([deleted.runs, deleted.text()]).toEqual([
        [
            { run: "a.1", start: 0, text: "a" },
            { run: "a.1", start: 1, text: 3 },
            { run: "a.1", start: 4, text: "ef" },
        ],
        "aef",
    ]);

    // insert after a deleted element, and restore the deletion through its inverse
    const inserted = deleted.apply({ insert: "X", run: "b.1", after: { run: "a.1", offset: 2 } });
    const [restore] = Sequence.inverse(deletion, typed);
    expect([
        inserted.text(),
        restore,
        deleted.apply(present(restore, "the restoring edit")).text(),
    ]).toEqual([
        "aXef",
        {
            restore: {
                from: { run: "a.1", offset: 1 },
                to: { run: "a.1", offset: 3 },
                text: "bcd",
            },
        },
        "abcdef",
    ]);

    // refuse restoring another number of characters than the tombstones have
    expect(() =>
        deleted.apply({
            restore: { from: { run: "a.1", offset: 1 }, to: { run: "a.1", offset: 3 }, text: "bc" },
        }),
    ).toThrow(new RangeError("sequence restores 2 characters into 3 deleted ones"));
});

test("find the one replacement between two texts, keeping their common start and end", () => {
    // typing, deleting, replacing a word, and a repeated letter the end must not overlap
    expect([
        TextChange.between("hello", "hello!"),
        TextChange.between("hello world", "hello"),
        TextChange.between("hello world", "hello there"),
        TextChange.between("aa", "aaa"),
        TextChange.between("same", "same"),
    ]).toEqual([
        { from: 5, to: 5, insert: "!" },
        { from: 5, to: 11, insert: "" },
        { from: 6, to: 11, insert: "there" },
        { from: 2, to: 2, insert: "a" },
        { from: 4, to: 4, insert: "" },
    ]);
});
