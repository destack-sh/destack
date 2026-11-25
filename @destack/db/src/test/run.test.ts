import { Failure } from "@destack/error";
import { present } from "@destack/schema";
import { expect, inject, test } from "@destack/test";
import { TestRun } from "./run.ts";
import { failingTemplate } from "./templates.ts";
import { stalledTemplate } from "./stalled.ts";

/** The deadline the stalled build runs past: 200 ms. */
const DEADLINE_MILLISECONDS = 200;

test("stop a template's build past its deadline, and fail the run's later forks with that failure at once", async () => {
    // build a template whose build never finishes, in the run with a short deadline
    const run = { ...present(inject("run"), "the run"), deadline: DEADLINE_MILLISECONDS };
    const building = TestRun.image(stalledTemplate, run);

    // fail at the deadline, and at once with that failure when asked again in the run
    await expect(building).rejects.toEqual(
        new Error("building template 'db.stalled' took over 0.2 s"),
    );
    await expect(TestRun.image(stalledTemplate, run)).rejects.toEqual(
        new Error(
            "building template 'db.stalled' failed: building template 'db.stalled' took over 0.2 s",
        ),
    );
});

test("keep a template build's failure when closing its databases fails too", async () => {
    // build a template that fails, and whose database fails to close after it
    const run = present(inject("run"), "the run");
    const first = await TestRun.image(failingTemplate, run).catch((error: unknown) => error);
    const again = await TestRun.image(failingTemplate, run).catch((error: unknown) => error);

    // report the build's failure before the close's, at once and in the run's later forks
    expect([
        Failure.causes(first).map((error) => error.message),
        again instanceof Error ? again.message : again,
    ]).toEqual([
        ["", "the build failed", "closing the build's databases failed", "connection is closing"],
        "building template 'db.failing' failed: the build failed: closing the build's databases failed: connection is closing",
    ]);
});
