import { expect, test } from "@destack/test";
import { createPageVisibility, usePageVisibility } from "./page-visibility.ts";

test("read the page as visible on the server", () => {
    expect([createPageVisibility()(), usePageVisibility()()]).toEqual([true, true]);
});
