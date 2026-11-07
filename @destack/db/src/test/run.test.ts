import { expect, test } from "@destack/test";
import { TestRun } from "./run.ts";

/** The deadline the stalled write runs past: 200 ms, far below the writer's start. */
const DEADLINE_MILLISECONDS = 200;

test("stop a template write running past its deadline, failing loudly", async () => {
    // write a module whose one template never finishes
    const writing = TestRun.write(new URL("./stalled.ts", import.meta.url), "sqlite", {
        deadline: DEADLINE_MILLISECONDS,
    });

    // fail once the writer is stopped
    await expect(writing).rejects.toEqual(
        new Error("writing a template took over 0.2 s and was stopped: "),
    );
});
