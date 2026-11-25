import { expect, test } from "@destack/test";
import { PaneLayout, type PaneNode, type PanePlace, type PaneSide } from "./layout.ts";

/** Write a part of a layout. */
function writeNode(node: PaneNode): string {
    if (node.kind === "leaf") {
        return node.items
            .map(({ id }) => (id === PaneLayout.selected(node)?.id ? `*${id}` : id))
            .join(" ");
    }
    const parts = node.children.map((child) => writeNode(child));

    return node.direction === "horizontal" ? `[${parts.join(" | ")}]` : `(${parts.join(" / ")})`;
}

/** Write a layout, selected items starred, rows in brackets and columns in parentheses. */
function write(layout: PaneLayout): string {
    return layout.root === undefined ? "" : writeNode(layout.root);
}

/** Read the shares of the root split, in percent. */
function sharesOf(layout: PaneLayout): readonly number[] {
    return layout.root?.kind === "split"
        ? layout.root.sizes.map((size) => Math.round(size * 100))
        : [];
}

/** Read the place beside the pane an item is in. */
function beside(layout: PaneLayout, item: string, side: PaneSide): PanePlace {
    const pane = PaneLayout.paneOf(layout, item);

    return pane === undefined ? { kind: "side", side } : { kind: "side", pane: pane.id, side };
}

test("split a pane by moving an item to its side, and go back as it closes", () => {
    const start = PaneLayout.of(["a", "b"]);
    const split = PaneLayout.move(start, "b", beside(start, "b", "end"));
    const closed = PaneLayout.close(split, "b");

    expect([start, split, closed].map((each) => write(each))).toEqual(["*a b", "[*a | *b]", "*a"]);
});

test("flatten a split along the same axis, and nest one across it", () => {
    const start = PaneLayout.of(["a", "x"], ["b"]);
    const along = PaneLayout.move(start, "x", beside(start, "a", "start"));
    const across = PaneLayout.move(along, "x", beside(along, "b", "bottom"));

    expect([write(along), sharesOf(along), write(across), sharesOf(across)]).toEqual([
        "[*x | *a | *b]",
        [25, 25, 50],
        "[*a | (*b / *x)]",
        [33, 67],
    ]);
});

test("open beside all panes with an even part", () => {
    const start = PaneLayout.of(["a"], ["b"]);
    const end = PaneLayout.open(start, "c", { kind: "side", side: "end" });
    const bottom = PaneLayout.open(start, "c", { kind: "side", side: "bottom" });

    expect([write(end), sharesOf(end), write(bottom), sharesOf(bottom)]).toEqual([
        "[*a | *b | *c]",
        [33, 33, 33],
        "([*a | *b] / *c)",
        [50, 50],
    ]);
});

test("open an item in the active pane, once", () => {
    const start = PaneLayout.of(["a"], ["b"]);
    const opened = PaneLayout.open(start, "c");
    const again = PaneLayout.open(PaneLayout.select(opened, "b"), "c");

    expect([
        write(opened),
        write(again),
        PaneLayout.active(again)?.items.map(({ id }) => id),
    ]).toEqual(["[a *c | *b]", "[a *c | *b]", ["a", "c"]]);
});

test("show the same value in two places", () => {
    const start = PaneLayout.of([{ id: "one", value: "plan" }]);
    const both = PaneLayout.open(start, { id: "two", value: "plan" }, beside(start, "one", "end"));

    expect(PaneLayout.panes(both).map(({ items }) => items[0]?.value)).toEqual(["plan", "plan"]);
});

test("select the item selected before a closed one", () => {
    let layout = PaneLayout.of(["a", "b", "c", "d"]);
    for (const item of ["d", "b", "c"]) {
        layout = PaneLayout.select(layout, item);
    }
    const closed = PaneLayout.close(layout, "c");

    expect([write(closed), write(PaneLayout.close(closed, "b"))]).toEqual(["a *b d", "a *d"]);
});

test("keep a pane without items where it stays", () => {
    const start = PaneLayout.of(["a"], ["b"]);
    const kept: PaneLayout = {
        ...start,
        root:
            start.root?.kind === "split"
                ? {
                      ...start.root,
                      children: start.root.children.map((child) =>
                          child.kind === "leaf" ? { ...child, isKept: true } : child,
                      ),
                  }
                : start.root,
    };

    expect([
        write(PaneLayout.close(start, "b")),
        write(PaneLayout.close(kept, "b")),
        write(PaneLayout.close(PaneLayout.of(["a"]), "a")),
    ]).toEqual(["*a", "[*a | ]", ""]);
});

test("merge, swap and move panes", () => {
    const start = PaneLayout.of(["a", "b"], ["c"], ["d"]);
    const [one, two, three] = PaneLayout.panes(start).map(({ id }) => id);
    const ids = [one ?? "", two ?? "", three ?? ""] as const;

    expect([
        write(PaneLayout.merge(start, ids[0], ids[2])),
        write(PaneLayout.swap(start, ids[0], ids[2])),
        write(PaneLayout.dock(start, ids[2], { kind: "side", pane: ids[0], side: "top" })),
        write(PaneLayout.dock(start, ids[2], { kind: "item", pane: ids[0] })),
    ]).toEqual(["[*c | d *a b]", "[*d | *c | *a b]", "[(*d / *a b) | *c]", "[a b *d | *c]"]);
});

test("find the pane on each side", () => {
    const start = PaneLayout.of(["a"], ["b"]);
    const layout = PaneLayout.open(start, "c", beside(start, "b", "bottom"));
    const from = (item: string) =>
        (["start", "end", "top", "bottom"] as const).map((side) => {
            const pane = PaneLayout.paneOf(layout, item);
            const neighbor =
                pane === undefined ? undefined : PaneLayout.neighbor(layout, pane.id, side);

            return neighbor?.items[0]?.id;
        });

    expect([from("a"), from("b"), from("c")]).toEqual([
        [undefined, "b", undefined, undefined],
        ["a", undefined, undefined, "c"],
        ["a", undefined, "b", undefined],
    ]);
});

/** Write what each pane has open, where it is and how high. */
function where(layout: PaneLayout): readonly string[] {
    return PaneLayout.placements(layout).map(({ pane, box, height }) =>
        [
            pane.items.map(({ id }) => id).join(" "),
            `@${[box.start, box.top, box.width, box.height].map((each) => each.toFixed(2)).join(",")}`,
            height,
        ].join(" "),
    );
}

/** Where the floats of these tests are. */
const BOUNDS = { start: 0.1, top: 0.2, width: 0.5, height: 0.4 };

test("float a tab, split it there, raise it, move it and dock it again", () => {
    const start = PaneLayout.of(["a", "b", "c"]);
    const floated = PaneLayout.move(start, "b", { kind: "float", bounds: BOUNDS });
    const own = PaneLayout.paneOf(floated, "b")?.id ?? "";
    const split = PaneLayout.move(floated, "c", { kind: "side", pane: own, side: "end" });
    const second = PaneLayout.open(split, "d", { kind: "float", bounds: BOUNDS });
    const raised = PaneLayout.select(second, "b");
    const moved = PaneLayout.place(raised, "float-1", { ...BOUNDS, start: 0.3 });
    const docked = PaneLayout.dock(moved, own, { kind: "side", side: "end" });

    expect([split, raised, moved, docked].map(where)).toEqual([
        ["a @0.00,0.00,1.00,1.00 0", "b @0.10,0.20,0.25,0.40 1", "c @0.35,0.20,0.25,0.40 1"],
        [
            "a @0.00,0.00,1.00,1.00 0",
            "d @0.10,0.20,0.50,0.40 1",
            "b @0.10,0.20,0.25,0.40 2",
            "c @0.35,0.20,0.25,0.40 2",
        ],
        [
            "a @0.00,0.00,1.00,1.00 0",
            "d @0.10,0.20,0.50,0.40 1",
            "b @0.30,0.20,0.25,0.40 2",
            "c @0.55,0.20,0.25,0.40 2",
        ],
        [
            "a @0.00,0.00,0.50,1.00 0",
            "b @0.50,0.00,0.50,1.00 0",
            "d @0.10,0.20,0.50,0.40 1",
            "c @0.30,0.20,0.50,0.40 2",
        ],
    ]);
});

/** List the panes that show. */
function shown(layout: PaneLayout): readonly string[] {
    return PaneLayout.placements(layout).map(({ pane }) => pane.id);
}

test("cover the others with a pane, and follow the person into another", () => {
    const start = PaneLayout.of(["a"], ["b"]);
    const [one, two] = PaneLayout.panes(start).map(({ id }) => id);
    const covered = PaneLayout.maximize(start, one ?? "");
    const other = PaneLayout.select(covered, "b");

    expect([shown(covered), shown(other), shown(PaneLayout.restore(other))]).toEqual([
        [one],
        [two],
        [one, two],
    ]);
});

test("hold every rule through any sequence of changes", () => {
    // step by a seeded sequence, so that a failure repeats
    let seed = 7;
    const next = (count: number): number => {
        seed = (seed * 1_103_515_245 + 12_345) % 2_147_483_648;

        return Math.floor((seed / 2_147_483_648) * count);
    };
    const sides: readonly PaneSide[] = ["start", "end", "top", "bottom"];
    const broken: string[] = [];
    for (let run = 0; run < 60; run += 1) {
        let layout = PaneLayout.of(["a", "b"], ["c"]);
        let opened = 0;
        for (let step = 0; step < 60; step += 1) {
            // change it by one verb at a time
            const panes = PaneLayout.panes(layout);
            const items = panes.flatMap((pane) => pane.items.map(({ id }) => id));
            const pane = next(12) === 0 ? "gone" : (panes[next(panes.length)]?.id ?? "gone");
            const other = panes[next(panes.length)]?.id ?? "gone";
            const item = items[next(items.length)] ?? "none";
            const side = sides[next(4)] ?? "end";
            const place: PanePlace =
                next(6) === 0
                    ? { kind: "float", bounds: BOUNDS }
                    : next(3) === 0
                      ? { kind: "item", pane, index: next(3) }
                      : next(4) === 0
                        ? { kind: "side", side }
                        : { kind: "side", pane, side };
            const expected = new Set(items);
            switch (next(9)) {
                case 0: {
                    opened += 1;
                    expected.add(`n${String(opened)}`);
                    layout = PaneLayout.open(layout, `n${String(opened)}`, place);
                    break;
                }
                case 1: {
                    expected.delete(item);
                    layout = PaneLayout.close(layout, item);
                    break;
                }
                case 2: {
                    layout = PaneLayout.merge(layout, pane, other);
                    break;
                }
                case 3: {
                    layout = PaneLayout.swap(layout, pane, other);
                    break;
                }
                case 4: {
                    layout = PaneLayout.dock(layout, pane, place);
                    break;
                }
                case 5: {
                    layout =
                        layout.maximized === undefined
                            ? PaneLayout.maximize(layout, pane)
                            : PaneLayout.restore(layout);
                    break;
                }
                case 6: {
                    layout = PaneLayout.place(layout, layout.floats[0]?.id ?? "gone", {
                        ...BOUNDS,
                        start: next(5) / 10,
                    });
                    break;
                }
                default: {
                    layout = PaneLayout.move(layout, item, place);
                }
            }
            broken.push(
                ...problemsOf(layout, expected).map(
                    (each) => `${String(run)}.${String(step)} ${each}`,
                ),
            );
        }
    }

    expect(broken).toEqual([]);
});

/** List the rules a layout breaks. */
function problemsOf(layout: PaneLayout, expected: ReadonlySet<string>): readonly string[] {
    // keep every item, once
    const problems: string[] = [];
    const panes = PaneLayout.panes(layout);
    const items = panes.flatMap((pane) => pane.items.map(({ id }) => id));
    if (items.length !== expected.size || items.some((item) => !expected.has(item))) {
        problems.push(`items ${items.join(" ")} for ${[...expected].join(" ")}`);
    }

    // select one of its own items in each pane, and be in one
    for (const pane of panes) {
        const isOwn = pane.recent.every((id) => pane.items.some((item) => item.id === id));
        if (pane.items.length === 0 || !isOwn || PaneLayout.selected(pane) === undefined) {
            problems.push(`pane ${pane.id}`);
        }
    }
    if (new Set(panes.map(({ id }) => id)).size !== panes.length) {
        problems.push("pane ids");
    }
    if (panes.length > 0 && PaneLayout.active(layout) === undefined) {
        problems.push("active");
    }

    // fill the group and each float without gaps, and alternate the axes
    const areas = new Map<number, number>();
    for (const { box, height } of PaneLayout.placements(PaneLayout.restore(layout))) {
        areas.set(height, (areas.get(height) ?? 0) + box.width * box.height);
    }
    const whole = [
        ...(layout.root === undefined ? [] : [[0, 1] as const]),
        ...layout.floats.map(
            ({ bounds }, index) => [index + 1, bounds.width * bounds.height] as const,
        ),
    ];
    for (const [height, area] of whole) {
        if (Math.abs((areas.get(height) ?? 0) - area) > 1e-6) {
            problems.push(`area ${String(areas.get(height))} at ${String(height)}`);
        }
    }
    if (areas.size !== whole.length) {
        problems.push(`trees ${String(areas.size)}`);
    }
    const check = (node: PaneNode): void => {
        if (node.kind === "split") {
            const total = node.sizes.reduce((sum, size) => sum + size, 0);
            const isFlat = node.children.every(
                (child) => child.kind === "leaf" || child.direction !== node.direction,
            );
            if (node.children.length < 2 || !isFlat || Math.abs(total - 1) > 1e-6) {
                problems.push(`split ${String(node.children.length)} ${String(total)}`);
            }
            node.children.forEach((child) => check(child));
        }
    };
    if (layout.root !== undefined) {
        check(layout.root);
    }
    layout.floats.forEach(({ root }) => check(root));

    return problems;
}
