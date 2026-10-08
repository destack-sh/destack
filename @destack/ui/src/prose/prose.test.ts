import { readFile } from "node:fs/promises";
import { join } from "node:path";
import * as tokens from "@destack/theme/tokens.stylex";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";

/** A custom property a stylesheet reads through `var()`. */
const VARIABLE = /var\((--destack-[a-z0-9-]+)/gu;

/** Collect the custom properties a tree of token constants reads. */
function variablesOf(value: unknown, into: Set<string>): Set<string> {
    // read a constant's custom properties
    if (typeof value === "string") {
        for (const match of value.matchAll(VARIABLE)) {
            into.add(present(match[1], "a custom property's name"));
        }
    }
    // read every constant of a family
    else if (typeof value === "object" && value !== null) {
        for (const entry of Object.values(value)) {
            variablesOf(entry, into);
        }
    }

    return into;
}

test("style prose through custom properties the theme's tokens define alone", async () => {
    // read the custom properties the stylesheet and the tokens read
    const sheet = await readFile(join(import.meta.dirname, "prose.css"), "utf8");
    const used = [...sheet.matchAll(VARIABLE)].map((match) =>
        present(match[1], "a custom property's name"),
    );
    const defined = variablesOf(tokens, new Set());

    // keep none the tokens leave undefined
    expect(used.filter((name) => !defined.has(name))).toEqual([]);
});
