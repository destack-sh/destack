import { expect, test } from "@destack/test";
import { createActiveElement, makeActiveElementListener } from "./active-element.ts";

test("follow no focused element on the server", () => {
    let isCalled = false;
    makeActiveElementListener(() => {
        isCalled = true;
    });

    expect([isCalled, createActiveElement()()]).toEqual([false, null]);
});
