import { expect, test } from "@destack/test";
import { createRemSize, getRemSize, setServerRemSize, useRemSize } from "./styles.ts";

test("answer the server's rem size on the server, as set", () => {
    const initial = [getRemSize(), createRemSize()(), useRemSize()()];
    setServerRemSize(20);
    const set = getRemSize();
    setServerRemSize(16);

    expect([initial, set]).toEqual([[16, 16, 16], 20]);
});
