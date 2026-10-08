import { expect, test } from "@destack/test";
import {
    createPerPointerListeners,
    createPointerList,
    createPointerListeners,
    createPointerPosition,
} from "./pointer.ts";

test("follow no pointers on the server", () => {
    createPointerListeners({ onMove: () => {} });
    createPerPointerListeners({ onDown: () => {} });
    const position = createPointerPosition();

    expect([position().isActive, position().x, createPointerList()()]).toEqual([false, 0, []]);
});
