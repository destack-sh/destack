import { expect, test } from "@destack/test";
import { Source } from "../source/index.ts";
import { Random } from "../test/random.ts";
import { Difference, type DifferenceOptions } from "./index.ts";

/** Compare two texts. */
function compare(before: string, after: string, options?: DifferenceOptions): Difference {
    return Difference.of(Source.of(before), Source.of(after), options);
}

/** Write the words that changed between texts. */
function words(before: string, after: string): string[] {
    const difference = compare(before, after);

    return difference.changes.flatMap((change) =>
        difference
            .words(change)
            .map(
                (word) =>
                    `${before.slice(word.before.from, word.before.to)}>${after.slice(word.after.from, word.after.to)}`,
            ),
    );
}

/** Write the changed ranges of texts, old then new. */
function ranges(difference: Difference): string[] {
    return difference.changes.map(
        ({ before, after }) => `${before.start}-${before.end}:${after.start}-${after.end}`,
    );
}

test("make the new text of the old one by the edits, for texts of any kind", () => {
    // compare random texts, some alike and some not
    const random = new Random(2);
    const wrong = Array.from({ length: 2000 }, () => {
        const before = random.text(14);
        const after = random.happens(50) ? random.changed(before) : random.text(14);
        const edited = Source.of(before).edit(compare(before, after).edits());

        return edited.text === after ? [] : [[before, after]];
    }).flat();

    expect(wrong).toEqual([]);
});

test("tell each change of a long text apart, however many there are", () => {
    // change every twentieth of forty thousand lines, most of them alike
    const lines = Array.from({ length: 40_000 }, (_, line) => `line ${String(line % 50)}`);
    const changed = lines.map((line, at) => (at % 20 === 7 ? `${line} changed` : line));
    const difference = compare(lines.join("\n"), changed.join("\n"));

    expect([difference.changes.length, difference.added, difference.removed]).toEqual([
        2000, 2000, 2000,
    ]);
});

test("put an added block where it starts, and compare without space when told to", () => {
    const before = "a() {\n  one();\n}\n\nc() {\n  three();\n}\n";
    const after = "a() {\n  one();\n}\n\nb() {\n  two();\n}\n\nc() {\n  three();\n}\n";

    expect({
        block: ranges(compare(before, after)),
        exact: ranges(compare("a b\n  c\n", "a  b\nc \n")),
        trailing: ranges(compare("a\nb \n", "a \nb\n", { whitespace: "trailing" })),
        ignored: ranges(compare("a b\n  c\n", "a  b\nc \n", { whitespace: "ignored" })),
    }).toEqual({ block: ["4-4:4-8"], exact: ["0-2:0-2"], trailing: [], ignored: [] });
});

test("compare the words of a change, none where little is left or too much changed", () => {
    expect({
        word: words("const port = 1;", "const port = 3000;"),
        several: words("load(id, name)", "load(key, name, force)"),
        call: words("plan.price + extra * 4", "plan.price + extra * plan.extra"),
        rewritten: words("alpha beta gamma", "one two three four"),
        indented: words("load(id)", "    load(key)"),
        indent: words("load(id)", "    load(id)"),
        added: words("a\n", "a\nb\n"),
        inserted: words("load(5);\n", "load(plan.people);\nconst total = plan.price;\n"),
    }).toEqual({
        word: ["1>3000"],
        several: ["id>key", ">, force"],
        call: ["4>plan.extra"],
        rewritten: [],
        indented: ["id>key"],
        indent: [">    "],
        added: [],
        inserted: ["5>plan.people"],
    });
});

test("gather changes into hunks with the lines around them, joined where those meet", () => {
    const before = Array.from({ length: 20 }, (_, line) => `${String(line)}\n`).join("");
    const after = before.replace("3\n", "three\n").replace("6\n", "").replace("19\n", "end");

    expect(
        compare(before, after)
            .hunks(1)
            .map((hunk) => [
                `${hunk.before.start}-${hunk.before.end}:${hunk.after.start}-${hunk.after.end}`,
                hunk.lines.map(
                    ({ kind, text, hasLineBreak }) =>
                        `${kind} ${text}${hasLineBreak ? "" : " (open)"}`,
                ),
            ]),
    ).toEqual([
        [
            "2-8:2-7",
            ["same 2", "removed 3", "added three", "same 4", "same 5", "removed 6", "same 7"],
        ],
        ["18-20:17-19", ["same 18", "removed 19", "added end (open)"]],
    ]);
});
