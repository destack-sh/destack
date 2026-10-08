import { expect, test } from "@destack/test";
import { createIdleTimer } from "./idle.ts";

test("run no idle timer on the server", () => {
    const timer = createIdleTimer();
    timer.start();

    expect([timer.isIdle(), timer.isPrompted()]).toEqual([false, false]);
});
