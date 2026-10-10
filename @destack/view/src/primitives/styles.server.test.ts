import { expect, test } from "@destack/test";
import { createRemSize, getRemSize, useRemSize } from "./styles.ts";

test("answer the browsers' default rem size on the server", () => {
    expect([getRemSize(), createRemSize()(), useRemSize()()]).toEqual([16, 16, 16]);
});
