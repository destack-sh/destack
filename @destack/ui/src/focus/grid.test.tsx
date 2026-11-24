import { expect, test } from "@destack/test";
import { createRoot, flush } from "@destack/view";
import { Collection } from "../collection/index.ts";
import { type GridBreaks, GridDelegate } from "./index.ts";

/** Break five keys: a wide and a narrow cell, a narrow and a wide one, and one alone. */
const BREAKS: GridBreaks = (keys) => [
    { keys: keys.slice(0, 2), size: { height: 10, widths: [70, 30], gap: 10 } },
    { keys: keys.slice(2, 4), size: { height: 10, widths: [20, 80], gap: 10 } },
    { keys: keys.slice(4), size: { height: 10, widths: [40], gap: 10 } },
];

/** Press a key on each of some keys, and read the key it lands on. */
function press(key: string, breaks?: GridBreaks): readonly (string | undefined)[] {
    return createRoot((dispose) => {
        const collection = new Collection<string>({
            sections: () => [{ key: "all", items: ["a", "b", "c", "d", "e"] }],
            key: (item) => item,
            text: (item) => item,
        });
        const delegate = new GridDelegate(collection, () => 2, breaks);
        flush();
        const landed = ["a", "b", "c", "d", "e"].map((from) =>
            delegate.target(new KeyboardEvent("keydown", { key }), from, "ltr"),
        );
        dispose();

        return landed;
    });
}

test("move by column", () => {
    expect([press("ArrowDown"), press("ArrowUp")]).toEqual([
        ["c", "d", "e", "e", undefined],
        [undefined, undefined, "a", "b", "c"],
    ]);
});

test("move by where cells stand", () => {
    expect([press("ArrowDown", BREAKS), press("ArrowUp", BREAKS)]).toEqual([
        ["d", "d", "e", "e", undefined],
        [undefined, undefined, "a", "a", "c"],
    ]);
});
