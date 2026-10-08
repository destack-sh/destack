import { expect, test } from "@destack/test";
import { createSelection } from "./selection.ts";

test("select nothing on the server", () => {
    const [selection, setSelection] = createSelection();
    setSelection([null, null, null]);

    expect(selection()).toEqual([null, null, null]);
});
