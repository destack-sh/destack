import { expect, test } from "vitest";
import { checkMarkdown, fixMarkdown } from "./index.ts";

/** Codes reported for a Markdown document. */
function codes(text: string): string[] {
    return checkMarkdown("note.md", text).map((diagnostic) => {
        if (diagnostic.code === undefined) {
            throw new TypeError("a markdown finding has no code");
        }

        return diagnostic.code;
    });
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

test("require one prose line then listings in each package README section", () => {
    // accept the opening line, a section's prose line with a code block and a list
    const valid =
        "# @example/notes\n\nKeep notes.\n\n## Read\n\n`read` reads a note.\n\n```ts\nread();\n```\n\n- one\n  more\n";
    expect(checkMarkdown("README.md", valid)).toEqual([]);

    // report a second prose line, prose after a listing and a table, only in a README
    const invalid =
        "# @example/notes\n\nKeep notes.\nShare them.\n\n## Read\n\nRead.\n\n```ts\nread();\n```\n\nThen write.\n\n| A |\n| - |\n";
    expect(checkMarkdown("notes/README.md", invalid).map((diagnostic) => diagnostic.code)).toEqual([
        "markdown(readme-section)",
        "markdown(readme-section)",
        "markdown(readme-table)",
    ]);
    expect(codes(invalid)).toEqual([]);
});
