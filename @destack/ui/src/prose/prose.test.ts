import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { TOKENS } from "@destack/theme";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";

/** A custom property a stylesheet reads through `var()`. */
const VARIABLE = /var\((--destack-[a-z0-9-]+)/gu;

test("style prose through custom properties the theme's tokens define alone", async () => {
    // read the custom properties the stylesheet reads and the ones the theme's tokens define
    const sheet = await readFile(join(import.meta.dirname, "prose.css"), "utf8");
    const used = [...sheet.matchAll(VARIABLE)].map((match) =>
        present(match[1], "a custom property's name"),
    );
    const defined = new Set<string>(
        TOKENS.families.flatMap((family) =>
            family.entries.flatMap((entry) => entry.members().map((member) => member.variable)),
        ),
    );

    // keep none the tokens leave undefined
    expect(used.filter((name) => !defined.has(name))).toEqual([]);
});
