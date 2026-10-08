import { expect, test } from "@destack/test";
import { createElementBounds } from "./bounds.ts";

test("read no element and keep null bounds on the server", () => {
    let isRead = false;
    const bounds = createElementBounds(() => {
        isRead = true;

        return undefined;
    });

    expect([isRead, { ...bounds }]).toEqual([
        false,
        { top: null, left: null, bottom: null, right: null, width: null, height: null },
    ]);
});
