import { expect, test } from "@destack/test";
import { Text } from "./text.ts";

test("order text by code units whatever the host locale", () => {
    expect(["b", "a", "B", "ä", "a"].toSorted((left, right) => Text.compare(left, right))).toEqual([
        "B",
        "a",
        "a",
        "b",
        "ä",
    ]);
});
