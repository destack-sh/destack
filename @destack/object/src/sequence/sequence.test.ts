import { expect, test } from "@destack/test";
import { nth, present, schema } from "@destack/schema";
import {
    RICH_TEXT,
    TextSequence,
    TextChange,
    TextSchema,
    TextTypes,
    type TextElement,
    type TextEdit,
    type TextMark,
} from "./index.ts";

/** The schema the sequence tests resolve marks and nodes with. */
const SCHEMA = TextSchema.of(
    ...RICH_TEXT,
    TextSchema.mark("highlight", { expand: "none", isMultiple: true, value: schema.string() }),
);

/** The edits each seeded run makes. */
const EDITS = 400;

/** One party editing a replica: the server's sequence it last saw and its edits not yet pushed. */
interface Party {
    /** The party's site, prefixing its runs. */
    readonly site: string;
    /** The marks and nodes the party has made. */
    counter: number;
    /** The offset of the next element the party types. */
    readonly clock: { value: number };
    /** The server's sequence the party last saw. */
    seen: TextSequence;
    /** The party's edits not yet pushed, in order. */
    pending: TextEdit[];
}

/** Draw a pseudo-random number from a seed, the same sequence every run. */
function random(seed: { value: number }): number {
    seed.value = (seed.value * 1_103_515_245 + 12_345) % 2_147_483_648;

    return seed.value / 2_147_483_648;
}

/** Pick a random edit on a party's replica: text, a node, a node change, a mark or its removal. */
function edit(
    sequence: TextSequence,
    seed: { value: number },
    site: string,
    counter: number,
    clock: { value: number },
): TextEdit {
    // delete a range of the visible text now and then
    const length = sequence.text().length;
    const run = `${site}.${counter}`;
    const draw = random(seed);
    if (length > 2 && draw < 0.25) {
        const from = Math.floor(random(seed) * (length - 1));
        const to = Math.min(length - 1, from + Math.floor(random(seed) * 5));
        const [deleted] = sequence.change({ from, to: to + 1, insert: "" }, { run, offset: 0 });

        return present(deleted, `the deletion between ${from} and ${to}`);
    }

    // mark a range, or remove a mark
    const marked = sequence.marks;
    if (length > 1 && draw < 0.4) {
        const from = Math.floor(random(seed) * (length - 1));
        const to = from + 1 + Math.floor(random(seed) * Math.min(4, length - from - 1));
        const isRemoving = random(seed) < 0.2;
        const marking = [
            { type: TextTypes.bold, value: isRemoving ? null : true },
            { type: TextTypes.link, value: { href: "https://destack.sh" } },
            { type: SCHEMA.markType("highlight"), value: site },
        ];
        const { type, value } = nth(marking, Math.floor(random(seed) * marking.length));

        return {
            mark: {
                id: run,
                type: type.name,
                value,
                ...sequence.range(from, to, type),
                order: null,
            },
        };
    } else if (marked.length > 0 && draw < 0.45) {
        return { unmark: [nth(marked, Math.floor(random(seed) * marked.length))] };
    }

    // change a visible node's level now and then
    const nodes = sequence.runs.filter((piece) => "node" in piece);
    if (nodes.length > 0 && draw < 0.5) {
        const node = nth(nodes, Math.floor(random(seed) * nodes.length));
        const level = 1 + Math.floor(random(seed) * 6);

        return {
            set: { run: node.run, offset: node.start },
            node: { type: "heading", value: { level } },
        };
    }

    // insert a heading marker, or type a few characters, after a visible element or at the start
    const offset = Math.floor(random(seed) * (length + 1));
    const after = offset === 0 ? undefined : sequence.element(offset - 1);
    // type through change as the site's next elements
    if (draw >= 0.55) {
        const typed = "abcdefghij".slice(0, 1 + Math.floor(random(seed) * 4));
        const next = { run: site, offset: clock.value };
        clock.value += typed.length;
        const [inserted] = sequence.change({ from: offset, to: offset, insert: typed }, next);

        return present(inserted, `the insertion at ${offset}`);
    }

    return {
        insert: { type: "heading", value: { level: 2 } },
        run,
        start: 0,
        ...(after === undefined ? {} : { after }),
    };
}

/** Apply a party's pushed edits on the server, ordering each mark. */
function push(server: { sequence: TextSequence; order: number }, edits: readonly TextEdit[]): void {
    for (const pushed of edits) {
        const ordered: TextEdit =
            "mark" in pushed ? { mark: { ...pushed.mark, order: (server.order += 1) } } : pushed;
        server.sequence = server.sequence.apply(ordered);
    }
}

/** List the visible elements of a sequence as `run:offset`. */
function visible(sequence: TextSequence): string[] {
    return sequence.runs.flatMap((piece) =>
        !TextSequence.isVisible(piece)
            ? []
            : Array.from(
                  { length: TextSequence.length(piece) },
                  (_, index) => `${piece.run}:${piece.start + index}`,
              ),
    );
}

/** Read a party's replica: the server's sequence it saw with its pending edits over it. */
function replica(party: Party): TextSequence {
    return party.pending.reduce((sequence, next) => sequence.apply(next), party.seen);
}

/** Build an element of the first typed run. */
function at(offset: number): TextElement {
    return { run: "a.1", offset };
}

test.for([3, 17, 91])(
    "converge every party's predicted replica on the server's order of their edits, marks and nodes included (seed %i)",
    (value) => {
        const seed = { value };
        const sites = ["a", "b", "c"];

        // store the server's sequence and each party's pending edits
        const server = { sequence: new TextSequence(), order: 0 };
        const parties = sites.map((site): Party => ({
            site,
            counter: 0,
            clock: { value: 0 },
            seen: server.sequence,
            pending: [],
        }));

        // edit, push and follow in turns
        for (let step = 0; step < EDITS; step++) {
            const party = nth(parties, Math.floor(random(seed) * parties.length));
            const draw = random(seed);
            if (draw < 0.7) {
                party.counter += 1;
                party.pending.push(
                    edit(replica(party), seed, party.site, party.counter, party.clock),
                );
            } else if (draw < 0.9) {
                push(server, party.pending);
                party.pending = [];
                party.seen = server.sequence;
            } else {
                party.seen = server.sequence;
            }
        }

        // push the rest and compare every replica's spans
        for (const party of parties) {
            push(server, party.pending);
            party.pending = [];
        }
        const replicas = parties.map((party) => {
            party.seen = server.sequence;

            return replica(party).spans(SCHEMA);
        });
        expect(replicas).toEqual(sites.map(() => server.sequence.spans(SCHEMA)));
    },
);

test.for([5, 23, 77])(
    "undo every edit exactly in reverse order, restoring the text and its elements (seed %i)",
    (value) => {
        const seed = { value };

        // edit a text as two parties, recording each edit
        let sequence = new TextSequence().apply({
            insert: "the quick brown fox",
            run: "base.1",
            start: 0,
        });
        let order = 0;
        const original = {
            text: sequence.text(),
            elements: visible(sequence),
            spans: sequence.spans(SCHEMA),
        };
        const applied: { readonly edit: TextEdit; readonly before: TextSequence }[] = [];
        const clocks = { ada: { value: 0 }, bo: { value: 0 } };
        for (let step = 0; step < EDITS / 4; step++) {
            const site = step % 2 === 0 ? "ada" : "bo";
            const drawn = edit(sequence, seed, site, step, clocks[site]);
            const next: TextEdit =
                "mark" in drawn ? { mark: { ...drawn.mark, order: (order += 1) } } : drawn;
            applied.push({ edit: next, before: sequence });
            sequence = sequence.apply(next);
        }

        // undo each edit through its inverse, the latest first
        for (const { edit: undone, before } of applied.toReversed()) {
            sequence = TextSequence.inverse(undone, before).reduce(
                (restored, next) => restored.apply(next),
                sequence,
            );
        }
        expect({
            text: sequence.text(),
            elements: visible(sequence),
            spans: sequence.spans(SCHEMA),
        }).toEqual(original);
    },
);

test("restore only the elements a deletion hid, leaving another party's earlier deletion in place", () => {
    // delete "quick " as one party and "the quick brown" as another, undoing only the second
    const base = new TextSequence().apply({
        insert: "the quick brown fox",
        run: "base.1",
        start: 0,
    });
    const first = base.apply({ delete: [{ run: "base.1", start: 4, text: 6 }] });
    const second: TextEdit = { delete: [{ run: "base.1", start: 0, text: 15 }] };
    const undone = TextSequence.inverse(second, first).reduce(
        (sequence, next) => sequence.apply(next),
        first.apply(second),
    );
    expect([first.apply(second).text(), undone.text()]).toEqual([" fox", "the brown fox"]);
});

test("find elements by visible offset and offsets by element, across tombstones", () => {
    // type, delete the middle, and read positions on both sides of the tombstone
    const sequence = new TextSequence()
        .apply({ insert: "hello world", run: "a.1", start: 0 })
        .apply({ delete: [{ run: "a.1", start: 5, text: 1 }] });
    expect([
        sequence.text(),
        sequence.element(5),
        sequence.offset({ run: "a.1", offset: 6 }),
        sequence.offset({ run: "a.1", offset: 5 }),
        sequence.element(10),
    ]).toEqual(["helloworld", { run: "a.1", offset: 6 }, 5, 5, undefined]);
});

test("number a site's elements by its clock, one run wherever it types", () => {
    // type a word key by key, then type at the start, then after another party's insertion
    const keys = "hello"
        .split("")
        .reduce(
            (sequence, key, index) =>
                sequence
                    .change({ from: index, to: index, insert: key }, { run: "ada", offset: index })
                    .reduce((next, made) => next.apply(made), sequence),
            new TextSequence(),
        );
    const elsewhere = keys
        .change({ from: 0, to: 0, insert: ">" }, { run: "ada", offset: 5 })
        .reduce((next, made) => next.apply(made), keys);
    const interleaved = elsewhere
        .apply({ insert: "!", run: "bo", start: 0, after: { run: "ada", offset: 4 } })
        .change({ from: 7, to: 7, insert: "?" }, { run: "ada", offset: 6 });
    expect({ keys: keys.runs, elsewhere: elsewhere.runs, interleaved }).toEqual({
        keys: [{ run: "ada", start: 0, text: "hello" }],
        elsewhere: [
            { run: "ada", start: 5, text: ">" },
            { run: "ada", start: 0, text: "hello" },
        ],
        interleaved: [{ insert: "?", run: "ada", start: 6, after: { run: "bo", offset: 0 } }],
    });
});

test("grow bold as people type after it, never a link, and neither at its start, as Peritext marks do", () => {
    // mark "hello" bold and "world" as a link by their types' ranges
    const text = new TextSequence().apply({ insert: "hello world", run: "a.1", start: 0 });
    const bold: TextMark = {
        id: "m.1",
        type: "bold",
        value: true,
        ...text.range(0, 5, TextTypes.bold),
        order: 1,
    };
    const link: TextMark = {
        id: "m.2",
        type: "link",
        value: { href: "https://destack.sh" },
        ...text.range(6, 11, TextTypes.link),
        order: 2,
    };

    // type after each range and before each, and delete inside the bold one
    const edited = text
        .apply({ mark: bold })
        .apply({ mark: link })
        .apply({ insert: "!", run: "b.1", start: 0, after: at(4) })
        .apply({ insert: "?", run: "b.2", start: 0, after: at(10) })
        .apply({ insert: ">", run: "b.3", start: 0, after: at(5) })
        .apply({ delete: [{ run: "a.1", start: 1, text: 2 }] });
    expect(edited.spans(SCHEMA)).toEqual([
        { text: "hlo!", marks: { bold: true } },
        { text: " >" },
        { text: "world", marks: { link: { href: "https://destack.sh" } } },
        { text: "?" },
    ]);
});

test("resolve each character to its latest single mark, keeping every overlapping highlight and removing what code excludes", () => {
    // bold, unbold, highlight and code overlapping ranges
    const text = new TextSequence().apply({ insert: "the quick brown fox", run: "a.1", start: 0 });
    const highlight = SCHEMA.markType("highlight");
    const marks: TextMark[] = [
        { id: "m.1", type: "bold", value: true, ...text.range(0, 19, TextTypes.bold), order: 1 },
        { id: "m.2", type: "bold", value: null, ...text.range(4, 9, TextTypes.bold), order: 2 },
        { id: "m.3", type: "highlight", value: "ada", ...text.range(4, 15, highlight), order: 3 },
        { id: "m.4", type: "highlight", value: "bo", ...text.range(10, 19, highlight), order: 4 },
        { id: "m.5", type: "code", value: true, ...text.range(16, 19, TextTypes.code), order: 5 },
    ];
    const marked = marks.reduce((sequence, mark) => sequence.apply({ mark }), text);
    expect(marked.spans(SCHEMA)).toEqual([
        { text: "the ", marks: { bold: true } },
        { text: "quick", marks: { highlight: ["ada"] } },
        { text: " ", marks: { bold: true, highlight: ["ada"] } },
        { text: "brown", marks: { bold: true, highlight: ["ada", "bo"] } },
        { text: " ", marks: { bold: true, highlight: ["bo"] } },
        { text: "fox", marks: { highlight: ["bo"], code: true } },
    ]);
});

test("insert, change and delete nodes as single elements, undoing each exactly", () => {
    // insert a heading marker before the text, raise its level, and mention someone after it
    const text = new TextSequence().apply({ insert: "plans", run: "a.1", start: 0 });
    const heading = { type: "heading", value: { level: 2 } };
    const steps: TextEdit[] = [
        { insert: heading, run: "b.1", start: 0 },
        { set: { run: "b.1", offset: 0 }, node: { type: "heading", value: { level: 1 } } },
        { insert: { type: "mention", value: "ada" }, run: "b.2", start: 0, after: at(4) },
        { delete: [{ run: "b.1", start: 0, text: 1 }] },
    ];
    const states = [text];
    for (const step of steps) {
        states.push(nth(states, states.length - 1).apply(step));
    }
    const undone = steps.reduceRight(
        (sequence, step, index) =>
            TextSequence.inverse(step, nth(states, index)).reduce(
                (next, inverse) => next.apply(inverse),
                sequence,
            ),
        nth(states, steps.length),
    );
    expect([
        ...states.slice(1).map((state) => state.spans(SCHEMA)),
        undone.spans(SCHEMA),
        nth(states, steps.length).text(),
    ]).toEqual([
        [{ node: heading }, { text: "plans" }],
        [{ node: { type: "heading", value: { level: 1 } } }, { text: "plans" }],
        [
            { node: { type: "heading", value: { level: 1 } } },
            { text: "plans" },
            { node: { type: "mention", value: "ada" } },
        ],
        [{ text: "plans" }, { node: { type: "mention", value: "ada" } }],
        [{ text: "plans" }],
        "plans\uFFFC",
    ]);
});

test.for([7, 41])(
    "sync an editor's plain text with the sequence through changes both ways (seed %i)",
    (value) => {
        const seed = { value };

        // mirror random edits between the sequence and a plain editor text
        let sequence = new TextSequence();
        let editor = "";
        const clock = { value: 0 };
        const applied: { readonly edit: TextEdit; readonly before: TextSequence }[] = [];
        for (let step = 0; step < EDITS / 2; step++) {
            // apply a remote edit as offset changes
            if (step % 2 === 0 || editor.length === 0) {
                const remote = edit(sequence, seed, "remote", step, clock);
                for (const change of sequence.translate(remote)) {
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
                const local = sequence.change(
                    { from, to, insert },
                    { run: `local.${step}`, offset: 0 },
                );
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
            for (const inverse of TextSequence.inverse(undone, before)) {
                for (const change of sequence.translate(inverse)) {
                    editor = editor.slice(0, change.from) + change.insert + editor.slice(change.to);
                }
                sequence = sequence.apply(inverse);
            }
        }
        expect([editor, sequence.text()]).toEqual(["", ""]);
    },
);

test("record a deletion's length alone, and restore exactly the characters it removed", () => {
    // delete the middle of a text and record only its length
    const typed = new TextSequence().apply({ insert: "abcdef", run: "a.1", start: 0 });
    const deletion: TextEdit = { delete: [{ run: "a.1", start: 1, text: 3 }] };
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
    const inserted = deleted.apply({
        insert: "X",
        run: "b.1",
        start: 0,
        after: { run: "a.1", offset: 2 },
    });
    const [restore] = TextSequence.inverse(deletion, typed);
    expect([
        inserted.text(),
        restore,
        deleted.apply(present(restore, "the restoring edit")).text(),
        inserted.apply(present(restore, "the restoring edit")).text(),
    ]).toEqual(["aXef", { restore: [{ run: "a.1", start: 1, text: "bcd" }] }, "abcdef", "abcXdef"]);

    // refuse a piece naming elements the text lacks
    expect(() => deleted.apply({ restore: [{ run: "a.1", start: 4, text: "xyz" }] })).toThrow(
        new RangeError("sequence has no element 'a.1' at offset 6"),
    );
});

test("keep what another party typed inside a deleted range when the deletion runs after it, and undo the deletion exactly", () => {
    // delete "quick " on one replica while another types "very " inside it, the server taking the typing first
    const base = new TextSequence().apply({ insert: "the quick brown fox", run: "a.1", start: 0 });
    const [deletion] = base.change({ from: 4, to: 10, insert: "" }, { run: "b", offset: 0 });
    const typed = base.apply({
        insert: "very ",
        run: "c.1",
        start: 0,
        after: { run: "a.1", offset: 6 },
    });
    const deleted = typed.apply(present(deletion, "the deletion"));

    // undo the deletion against the text it ran on, restoring only what it hid
    const undone = TextSequence.inverse(present(deletion, "the deletion"), typed).reduce(
        (sequence, next) => sequence.apply(next),
        deleted,
    );

    expect([
        deleted.text(),
        undone.text(),
        deleted.translate(present(deletion, "the deletion")),
    ]).toEqual(["the very brown fox", "the quivery ck brown fox", []]);
});

test("find the one replacement between two texts, around their common start and end", () => {
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
