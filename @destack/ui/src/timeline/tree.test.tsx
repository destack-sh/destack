import { expect, onTestFinished, test } from "@destack/test";
import { createRoot, flush } from "@destack/view";
import { type TimelineItem, TimelineTree } from "./tree.ts";

/** A plan in days: a launch with design and build under it, and a review on its own. */
const PLAN: readonly TimelineItem[] = [
    { key: "launch", label: "Launch" },
    { key: "design", label: "Design", parent: "launch", start: 0, end: 3 },
    { key: "build", label: "Build", parent: "launch", start: 3, end: 10 },
    { key: "review", label: "Review", start: 10, end: 12 },
];

/** Nest items as a tree, disposed after the test. */
function treeOf(items: readonly TimelineItem[]): TimelineTree {
    return createRoot((dispose) => {
        onTestFinished(dispose);

        return new TimelineTree({ items, axis: "offset" }, 0);
    });
}

/** Read the rows as their keys and depths. */
function rowsOf(tree: TimelineTree): string[] {
    return tree.rows().map((row) => `${"  ".repeat(row.depth)}${row.item.key}`);
}

test("nest items under their parents, folding a parent's children away", () => {
    // fold the launch, then read the rows
    const tree = treeOf(PLAN);
    const open = rowsOf(tree);
    tree.toggle("launch");
    flush();

    expect({ open, folded: rowsOf(tree), isNested: tree.isNested() }).toEqual({
        open: ["launch", "  design", "  build", "review"],
        folded: ["launch", "review"],
        isNested: true,
    });
});

test("place a moved row in a gap at a depth, under the nearest row a level up", () => {
    // move the review into the launch's children, the build to the top, and nest the review under build
    const tree = treeOf(PLAN);

    expect({
        underLaunch: tree.place("review", 1, 1),
        afterDesign: tree.place("review", 2, 1),
        top: tree.place("build", 0, 0),
        underBuild: tree.place("review", 3, 2),
        launchGaps: [0, 1, 2, 3, 4].map((index) => tree.gapOf("launch", index)),
        reviewGaps: [0, 1, 2, 3, 4].map((index) => tree.gapOf("review", index)),
        depths: [tree.depths("review", 2), tree.depths("review", 3), tree.depths("build", 0)],
        moves: [tree.isMove("review", 3, 0), tree.isMove("review", 3, 2)],
    }).toEqual({
        underLaunch: { key: "review", parent: "launch", next: "design" },
        afterDesign: { key: "review", parent: "launch", previous: "design", next: "build" },
        top: { key: "build", next: "launch" },
        underBuild: { key: "review", parent: "build" },
        // the gaps inside the launch's children are none, the gap below them is its own place
        launchGaps: [0, undefined, undefined, 0, 4],
        reviewGaps: [0, 1, 2, 3, 3],
        // between design and build one level or two, below build any level, above launch the top
        depths: [
            { min: 1, max: 2 },
            { min: 0, max: 2 },
            { min: 0, max: 0 },
        ],
        moves: [false, true],
    });
});

test("step a row a place up or down past its children, none past the ends", () => {
    // step the review up between design and build, the launch down past its children, and up from the top
    const tree = treeOf(PLAN);

    expect([tree.step("review", -1), tree.step("launch", 1), tree.step("launch", -1)]).toEqual([
        { key: "review", parent: "launch", previous: "design", next: "build" },
        { key: "launch", previous: "review" },
        undefined,
    ]);
});

test("step a row down into an open sibling as its first child, and up out of it again", () => {
    // a task above a phase with one open child, stepped down, then the child stepped up past the phase
    const tree = treeOf([
        { key: "task", label: "Task" },
        { key: "phase", label: "Phase" },
        { key: "child", label: "Child", parent: "phase" },
    ]);

    expect([tree.step("task", 1), tree.step("child", -1)]).toEqual([
        { key: "task", parent: "phase", next: "child" },
        { key: "child", previous: "task", next: "phase" },
    ]);
});

/** Read what reading a tree's rows throws, past the reactive system's wrapping. */
function refusalOf(items: readonly TimelineItem[]): unknown {
    try {
        treeOf(items).rows();
    } catch (error) {
        return error instanceof Error ? error.cause : error;
    }
    throw new TypeError("timeline tree read its rows without the refusal the test expects");
}

test("refuse an item whose parent is missing, and items that loop through their parents", () => {
    // a task under a project the plan lacks, and two items each under the other
    const orphan = refusalOf([{ key: "task", label: "Task", parent: "project" }]);
    const looped = refusalOf([
        { key: "a", label: "A", parent: "b" },
        { key: "b", label: "B", parent: "a" },
    ]);

    expect([orphan, looped]).toEqual([
        new RangeError("timeline item 'task' is invalid: its parent 'project' is missing"),
        new RangeError("timeline items 'a', 'b' loop through their parents"),
    ]);
});

test("refuse an item drawn on a missing row, or on a row that is drawn on another", () => {
    // a task on a person the plan lacks, and a task on a task that is on a person
    const missing = refusalOf([{ key: "copy", label: "Copy", row: "ada", start: 0, end: 1 }]);
    const nested = refusalOf([
        { key: "ada", label: "Ada" },
        { key: "copy", label: "Copy", row: "ada", start: 0, end: 1 },
        { key: "edit", label: "Edit", row: "copy", start: 0, end: 1 },
    ]);

    expect([missing, nested]).toEqual([
        new RangeError("timeline item 'copy' is invalid: its row 'ada' is missing"),
        new RangeError("timeline item 'edit' is invalid: its row 'copy' is drawn on another row"),
    ]);
});

test("roll up a folded item's dated descendants, listed in any order", () => {
    // list a grandchild before its parent and grandparent, then fold the top
    const tree = treeOf([
        { key: "spec", label: "Spec", parent: "design", start: 1, end: 2 },
        { key: "design", label: "Design", parent: "launch" },
        { key: "launch", label: "Launch" },
        { key: "build", label: "Build", parent: "launch", start: 3, end: 10 },
    ]);
    tree.toggle("launch");
    flush();

    // list them in the rows' order: the spec under the design, then the build
    expect(tree.rollup("launch").map((item) => item.key)).toEqual(["spec", "build"]);
});

test("pack items with one date into lanes by the length their bars draw", () => {
    // an open task drawn past the next task's start, and two milestones at one instant
    const tree = treeOf([
        { key: "alice", label: "Alice" },
        { key: "open", label: "Open", row: "alice", start: 0 },
        { key: "next", label: "Next", row: "alice", start: 1, end: 100 },
        { key: "first", label: "First", row: "alice", start: 100, end: 100 },
        { key: "second", label: "Second", row: "alice", start: 100, end: 100 },
    ]);

    // the open task draws 8% of the span, so the next task overlaps it and the milestones stack
    expect(tree.lanes("alice").map((lane) => lane.map((item) => item.key))).toEqual([
        ["open", "first"],
        ["next", "second"],
    ]);
});
