import { expect, test } from "@destack/test";
import { createWebShare } from "./share.ts";

test("share nothing on the server", () => {
    const { pending, status, message } = createWebShare();

    expect([pending(), status(), message()]).toEqual([false, undefined, undefined]);
});
