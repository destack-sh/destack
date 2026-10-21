import { expect, test } from "@destack/test";
import { Label } from "../label/index.ts";
import { Derivation } from "./index.ts";

test("derive apart by label, by context and by where the label ends, and refuse an overlong label", async () => {
    // derive under labels that share their characters but split them differently
    const root = await Derivation.root(crypto.getRandomValues(new Uint8Array(32)));
    const secret = async (label: string, context: string) =>
        (await Derivation.secret(root, { label, context })).toHex();
    const derived = [
        await secret("@destack/db/column", ""),
        await secret("@destack/db/column", "a"),
        await secret("@destack/db/column", "b"),
        await secret("@destack/db/columna", ""),
        await secret("ab", "c"),
        await secret("a", "bc"),
    ];

    // derive six distinct secrets, the same again for the same label and context
    expect({
        distinct: new Set(derived).size,
        again: derived[0] === (await secret("@destack/db/column", "")),
        overlong: (() => {
            try {
                Label.encodeExpansion({ label: "x".repeat(244), context: "" }, 32);

                return "encoded";
            } catch (error) {
                return error instanceof TypeError ? error.message : error;
            }
        })(),
    }).toEqual({
        distinct: 6,
        again: true,
        overlong: `label ${"x".repeat(244)} or its context is too long`,
    });
});
