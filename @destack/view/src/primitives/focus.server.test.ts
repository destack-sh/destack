import { expect, test } from "@destack/test";
import { untouched } from "../test/server.ts";
import { createFocusSignal } from "./focus.ts";

test("follow no focus on the server", () => {
    expect(createFocusSignal(untouched<Element>())()).toBe(false);
});
