import { expect, test } from "@destack/test";
import { share } from "./share.ts";

test("keep each unchanged item of a new list, matched by id across moves", () => {
    // list two tasks, then a new one in front of them with the second one renamed
    const first = { id: "a", title: "Ship", labels: ["bug"] };
    const second = { id: "b", title: "Test", labels: [] };
    const earlier = [first, second];
    const next = [
        { id: "c", title: "Plan", labels: [] },
        { id: "a", title: "Ship", labels: ["bug"] },
        { id: "b", title: "Test again", labels: [] },
    ];
    const shared = share(earlier, next);

    // keep the unchanged task itself, and take the new and the renamed ones as given
    expect([
        shared[1] === first,
        shared[2] === second,
        shared[0] === next[0],
        shared[2] === next[2],
        shared[2]?.labels === second.labels,
        shared,
    ]).toEqual([true, false, true, false, true, next]);
});

test("keep the earlier value whole when nothing changed, and take any other value as given", () => {
    // compare equal lists of objects, and values of other shapes
    const earlier = { tasks: [{ id: "a", due: null }], total: 1 };
    const date = new Date(0);

    expect([
        share(earlier, { tasks: [{ id: "a", due: null }], total: 1 }) === earlier,
        share(earlier, { tasks: [{ id: "a", due: null }] }) === earlier,
        share(undefined, earlier) === earlier,
        share(new Date(0), date) === date,
        share([1, 2], [1, 2, 3]),
    ]).toEqual([true, false, true, true, [1, 2, 3]]);
});
