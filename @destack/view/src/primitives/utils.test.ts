import { expect, test } from "@destack/test";
import { createHydratableSignal, handleDiffArray } from "./utils.ts";
import { flush } from "solid-js";

/** Diff two lists into the items added and removed. */
function diff(current: string[], previous: string[]): { added: string[]; removed: string[] } {
    const added: string[] = [];
    const removed: string[] = [];
    handleDiffArray(
        current,
        previous,
        (item) => added.push(item),
        (item) => removed.push(item),
    );

    return { added, removed };
}

test("add every item of a list that had none", () => {
    expect(diff(["foo", "bar", "baz"], [])).toEqual({ added: ["foo", "bar", "baz"], removed: [] });
});

test("remove every item of a cleared list", () => {
    expect(diff([], ["foo", "bar", "baz"])).toEqual({ added: [], removed: ["foo", "bar", "baz"] });
});

test("report nothing for an unchanged list", () => {
    expect(diff(["foo", "bar"], ["foo", "bar"])).toEqual({ added: [], removed: [] });
});

test("report the items a list gained and lost", () => {
    expect(diff(["foo", "bar", "hello", "world"], ["foo", "baz", "hello"])).toEqual({
        added: ["bar", "world"],
        removed: ["baz"],
    });
});

test("read the browser's value of a hydratable signal outside hydration", () => {
    const [state, setState] = createHydratableSignal("server", () => "client");
    const before = state();
    setState("written");
    flush();

    expect([before, state()]).toEqual(["client", "written"]);
});
