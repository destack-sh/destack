import { expect, test } from "@destack/test";
import { createClipboard } from "./clipboard.ts";

test("hold no clipboard items on the server", () => {
    const [clipboard] = createClipboard();

    expect(clipboard()).toEqual([]);
});
