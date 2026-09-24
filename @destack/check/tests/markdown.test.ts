import { expect, test } from "vitest";
import { checkMarkdown, fixMarkdown } from "../src/markdown/index.ts";

/** Codes reported for a Markdown document. */
function codes(text: string): string[] {
    return checkMarkdown("note.md", text).map((diagnostic) => diagnostic.code!);
}

test("split prose lines into one sentence each", () => {
    // report and fix a line with two sentences, keeping list indentation
    const text = "# Notes\n\n- Read the note. Then write it.\n";
    expect(codes(text)).toEqual(["markdown(one-sentence-per-line)"]);
    expect(fixMarkdown(text)).toBe("# Notes\n\n- Read the note.\n  Then write it.\n");

    // ignore abbreviations, code, links, tables and fenced blocks
    expect(
        codes(
            "# Notes\n\nUse e.g. `a. B` and [note](https://a.b/c. D).\n\n| A. B | C |\n\n```ts\nread(). Then();\n```\n",
        ),
    ).toEqual([]);
});

test("require stepwise headings, one title and fenced code languages", () => {
    expect(codes("# A\n\n### B\n\n# C\n\n```\nx\n```\n")).toEqual([
        "markdown(heading-increment)",
        "markdown(single-title)",
        "markdown(fenced-code-language)",
    ]);
});

test("allow top-level sections when front matter names the title", () => {
    expect(codes('---\ntitle: "Notes"\n---\n\n# Read\n\n# Write\n')).toEqual([]);
});
