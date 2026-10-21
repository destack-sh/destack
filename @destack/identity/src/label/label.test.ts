import { expect, test } from "@destack/test";
import { Label } from "./index.ts";

test("encode a label as its prefixed label and context, and a derivation's info with its output length first", () => {
    // encode a namespaced label bound to an installation
    const label = { label: "@destack/db/column", context: "installation-1" };
    const prefixed = new TextEncoder().encode("destack 1.0 @destack/db/column").toHex();
    const context = new TextEncoder().encode("installation-1").toHex();

    // write the 30-byte prefixed label after one length byte and the context after two, and 32 before both
    expect({
        encryption: Label.encode(label).toHex(),
        expansion: Label.encodeExpansion(label, 32).toHex(),
    }).toEqual({
        encryption: `1e${prefixed}000e${context}`,
        expansion: `00201e${prefixed}000e${context}`,
    });
});
