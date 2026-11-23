import { expect, test } from "@destack/test";
import { Page, PageState } from "./page.ts";

/** A request with a read and a write, the write with a commit: one line per span. */
const LINES = new Page(
    [],
    ["request", "read", "write", "commit"].map((key, index) => ({
        key,
        depth: [0, 1, 1, 2][index] ?? 0,
        text: key,
        row: undefined,
    })),
    "",
);

/** Press keys in order from a state, on a terminal of a height, the pager taking three lines. */
function pressed(
    keys: readonly Parameters<Page["press"]>[1][],
    page: Page = LINES,
    height = 13,
    from: PageState = PageState.start(),
): PageState {
    return keys.reduce((state, key) => page.press(state, key, height), from);
}

test("move down and up, fold a subtree and step out to its parent, unfold it and step into it", () => {
    // fold the write's subtree, then step from the commit-less write out to the request
    const folded = pressed(["down", "down", "left"]);
    const outOfWrite = pressed(["left"], LINES, 13, folded);

    // unfold the write and step into its commit
    const unfolded = pressed(["right"], LINES, 13, { ...folded, cursor: "write" });
    const intoCommit = pressed(["right"], LINES, 13, unfolded);

    expect({
        folded: [folded.cursor, [...folded.folded]],
        visible: LINES.visible(folded.folded).map((line) => line.key),
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
    const closed = pressed(["enter"], LINES, 13, opened);

    // select the read and the commit, then deselect the read
    const selected = pressed(["down", "select", "down", "down", "select", "up", "up", "select"]);

    // walk to the last line two lines at a time, and back to the first
    const bottom = pressed(["down", "down", "down"], LINES, 5);
    const back = pressed(["up", "up", "up"], LINES, 5, bottom);

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

test("follow the cursor's object when new lines arrive above it", () => {
    // stand on the write, then let a new span arrive first
    const onWrite = pressed(["down", "down"]);
    const arrived = new Page(
        [],
        [{ key: "auth", depth: 0, text: "auth", row: undefined }, ...LINES.lines],
        "",
    );

    // move down from the write among the new lines
    expect(pressed(["down"], arrived, 13, onWrite).cursor).toBe("commit");
});
