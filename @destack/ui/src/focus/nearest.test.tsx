import { expect, test } from "@destack/test";
import { nearestAcross } from "./index.ts";

/** Make an element at a box of left, top, width and height. */
function at(name: string, ...[x, y, width, height]: [number, number, number, number]): HTMLElement {
    const element = document.createElement("div");
    element.id = name;
    element.getBoundingClientRect = () => new DOMRect(x, y, width, height);

    return element;
}

test("find the nearest item above and below", () => {
    // two rows of unequal widths, and a short item alone
    const all = [
        at("a", 0, 0, 100, 50),
        at("b", 110, 0, 200, 50),
        at("c", 0, 60, 220, 60),
        at("d", 230, 60, 80, 60),
        at("e", 0, 130, 40, 50),
    ];
    const steps = all.map((from) => [
        from.id,
        nearestAcross(all, from, -1)?.id,
        nearestAcross(all, from, 1)?.id,
    ]);

    expect(steps).toEqual([
        ["a", undefined, "c"],
        ["b", undefined, "c"],
        ["c", "b", "e"],
        ["d", "b", "e"],
        ["e", "c", undefined],
    ]);
});

test("find the nearest item of staggered columns", () => {
    const left = [at("a", 0, 0, 100, 80), at("c", 0, 90, 100, 200)];
    const right = [at("b", 110, 0, 100, 150), at("d", 110, 160, 100, 60)];
    const all = [left[0], right[0], left[1], right[1]].filter((each) => each !== undefined);

    expect(
        all.map((from) => [
            from.id,
            nearestAcross(all, from, 1)?.id,
            nearestAcross(all, from, -1)?.id,
        ]),
    ).toEqual([
        ["a", "c", undefined],
        ["b", "d", undefined],
        ["c", undefined, "a"],
        ["d", undefined, "b"],
    ]);
});
