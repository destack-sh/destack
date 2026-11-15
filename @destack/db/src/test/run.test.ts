import { present } from "@destack/schema";
import { expect, inject, test } from "@destack/test";
import { TestRun } from "./run.ts";
import { stalledTemplate } from "./stalled.ts";

/** The deadline the stalled build runs past: 200 ms. */
const DEADLINE_MILLISECONDS = 200;

test("stop waiting for a template's build past its deadline, failing loudly", async () => {
    // build a template whose build never finishes, in the run with a short deadline
    const run = { ...present(inject("run"), "the run"), deadline: DEADLINE_MILLISECONDS };
    const building = TestRun.image(stalledTemplate, run);

    // fail at the deadline
    await expect(building).rejects.toEqual(
        new Error("building template 'db.stalled' took over 0.2 s"),
    );
});
