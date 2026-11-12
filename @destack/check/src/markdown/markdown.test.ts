import { expect, test } from "vitest";
import { checkMarkdown, fixMarkdown } from "./index.ts";

/** Codes reported for a Markdown document, a package README when given its description. */
function codes(text: string, description?: string): string[] {
    return checkMarkdown("note.md", text, description).map((diagnostic) => {
        if (diagnostic.code === undefined) {
            throw new TypeError("a markdown finding has no code");
        }

        return diagnostic.code;
    });
}

test("split prose lines into one sentence each", () => {
    // report and fix a line with two sentences, preserving list indentation
    const text = "# Notes\n\n- Read the note. Then write it.\n";
    expect(codes(text)).toEqual(["markdown(one-sentence-per-line)"]);
    expect(fixMarkdown(text)).toBe("# Notes\n\n- Read the note.\n  Then write it.\n");

    // ignore the numbers of ordered lists, also in blockquotes
    expect(codes("# Notes\n\n1. **Read** the note.\n\n> 2. **Write** it, then keep it.\n")).toEqual(
        [],
    );

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

test("start a package README with its package's description", () => {
    const readme = "# @destack/mail\n\nSend email over SMTP.\n\n## Overview\n";
    expect([
        codes(readme, "Send email over SMTP."),
        codes(readme, "Send email over SMTP, SES or a printer."),
    ]).toEqual([[], ["markdown(readme-description)"]]);
});
