import { expect, test } from "@destack/test";
import { ObjectViewState, type ViewLine } from "./state.ts";

/** A request with a read and a write, the write with a commit: one line per span. */
const LINES: readonly ViewLine[] = [
    { key: "request", depth: 0 },
    { key: "read", depth: 1 },
    { key: "write", depth: 1 },
    { key: "commit", depth: 2 },
];

/** Press keys in order from a state, at a height. */
function pressed(
    keys: readonly Parameters<typeof ObjectViewState.press>[1][],
    lines: readonly ViewLine[] = LINES,
    height = 10,
    from: ObjectViewState = ObjectViewState.start(),
): ObjectViewState {
    return keys.reduce((state, key) => ObjectViewState.press(state, key, lines, height), from);
}

test("move down and up, fold a subtree and step out to its parent, unfold it and step into it", () => {
    // fold the write's subtree, then step from the commit-less write out to the request
    const folded = pressed(["down", "down", "left"]);
    const outOfWrite = pressed(["left"], LINES, 10, folded);

    // unfold the write and step into its commit
    const unfolded = pressed(["right"], LINES, 10, { ...folded, cursor: "write" });
    const intoCommit = pressed(["right"], LINES, 10, unfolded);

    expect({
        folded: [folded.cursor, [...folded.folded]],
        visible: ObjectViewState.visible(LINES, folded.folded).map((line) => line.key),
        outOfWrite: outOfWrite.cursor,
        unfolded: [unfolded.cursor, [...unfolded.folded]],
        intoCommit: intoCommit.cursor,
        clamped: [pressed(["up"]).cursor, pressed(["down", "down", "down", "down"]).cursor],
    }).toEqual({
        folded: ["write", ["write"]],
        visible: ["request", "read", "write"],
        outOfWrite: "request",
        unfolded: ["write", []],
        intoCommit: "commit",
        clamped: ["request", "commit"],
    });
});

test("open a line's details and close them, select lines for bulk actions, and scroll to the cursor", () => {
    // open and close the read's details
    const opened = pressed(["down", "enter"]);
    const closed = pressed(["enter"], LINES, 10, opened);

    // select the read and the commit, then deselect the read
    const selected = pressed(["down", "select", "down", "down", "select", "up", "up", "select"]);

    // walk to the last line two lines at a time, and back to the first
    const bottom = pressed(["down", "down", "down"], LINES, 2);
    const back = pressed(["up", "up", "up"], LINES, 2, bottom);

    expect({
        opened: opened.opened,
        closed: closed.opened,
        selected: [...selected.selected],
        bottom: [bottom.cursor, bottom.top],
        back: [back.cursor, back.top],
    }).toEqual({
        opened: "read",
        closed: undefined,
        selected: ["commit"],
        bottom: ["commit", 2],
        back: ["request", 0],
    });
});

test("keep the cursor on its object when new lines arrive above it", () => {
    // stand on the write, then let a new span arrive first
    const onWrite = pressed(["down", "down"]);
    const arrived = [{ key: "auth", depth: 0 }, ...LINES];

    // move down from the write among the new lines
    expect(pressed(["down"], arrived, 10, onWrite).cursor).toBe("commit");
});
