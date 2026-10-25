import { principal, relation } from "@destack/access";
import { space } from "@destack/account/object";
import { eq, TABLE } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { present, schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { Failure } from "@destack/error";
import { expect, refusal, test } from "@destack/test";
import { defineWorkflow, Run } from "../src/index.ts";
import { actor, alice, bob, note, openWorkflow, type WorkflowFixture } from "./fixture.ts";
import { TEST_START } from "@destack/object/test";
import { NOTES } from "./package.ts";
import { NO_OUTPUT } from "../src/index.ts";

/** A minute, in milliseconds. */
const MINUTE = 60_000;

test("retry a failing advance after its backoff until it succeeds, keeping the last failure", async () => {
    // fail twice before succeeding
    const flaky = defineWorkflow(
        {
            name: "flaky",
            input: schema.object({ succeedAt: schema.number() }),
            output: schema.number(),
            retries: { initialInterval: 1000, backoffCoefficient: 2 },
        },
        NOTES,
    );
    let calls = 0;
    const fixture = await openWorkflow(flaky, async (step, input) => {
        calls += 1;
        if (calls < input.succeedAt) {
            throw new Error(`attempt ${calls} failed`);
        }

        return step.run("answer", schema.number(), async () => calls);
    });

    // start a run that succeeds on its third attempt, a second, then a third after a second's backoff
    await fixture.start(() => flaky.start({ succeedAt: 3 }, actor()));
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "completed");
    expect([run?.["attempts"], run?.["output"], run?.["failure"]]).toEqual([
        2,
        3,
        Failure.http("INTERNAL_SERVER_ERROR", "attempt 2 failed"),
    ]);
});

test("fail a run once its attempts run out, keeping the last failure", async () => {
    const brittle = defineWorkflow(
        {
            name: "brittle",
            input: schema.object({}),
            retries: { initialInterval: 1000, maximumAttempts: 2 },
            output: NO_OUTPUT,
        },
        NOTES,
    );
    const fixture = await openWorkflow(brittle, async () => {
        throw new Error("always");
    });

    // start a run that fails both attempts
    await fixture.start(() => brittle.start({}, actor()));
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "failed");
    expect([run?.["attempts"], run?.["failure"]]).toEqual([
        1,
        Failure.http("INTERNAL_SERVER_ERROR", "always"),
    ]);
});

test("time a run out at its deadline while it sleeps past it", async () => {
    const slow = defineWorkflow(
        { name: "slow", input: schema.object({}), timeout: { hours: 1 }, output: NO_OUTPUT },
        NOTES,
    );
    const fixture = await openWorkflow(slow, async (step) => {
        await step.sleep("nap", { hours: 2 });
    });

    // start a run sleeping two hours under an hour's timeout, and move to its deadline
    await fixture.start(() => slow.start({}, actor()));
    await fixture.simulation.jump(TEST_START + 60 * MINUTE);
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "failed");
    expect([run?.["deadline"], run?.["failure"]]).toEqual([
        TEST_START + 60 * MINUTE,
        {
            code: "TIMEOUT",
            reason: "RUN_TIMED_OUT",
            domain: "@destack/workflow",
            message: `run ${String(run?.["id"])} timed out`,
        },
    ]);
});

test("start a run at its time, answer the live run of a key, and replace one that has not begun", async () => {
    const digest = defineWorkflow(
        {
            name: "digest",
            input: schema.object({ topic: schema.string() }),
            output: schema.string(),
        },
        NOTES,
    );
    const fixture = await openWorkflow(digest, async (step, input) =>
        step.run("send", schema.string(), async () => input.topic),
    );

    // start a run in ten minutes, replace its topic, and start the key again
    const at = TEST_START + 10 * MINUTE;
    await fixture.start(() => digest.start({ topic: "a" }, { key: "daily", at, ...actor() }));
    await fixture.start(() =>
        digest.start({ topic: "b" }, { key: "daily", at, mode: "replace", ...actor() }),
    );
    await fixture.start(() => digest.start({ topic: "c" }, { key: "daily", ...actor() }));
    const waiting = await fixture.until(() => true);

    // run it at its time, then start the key anew once it ended
    const [ran] = await fixture.until((rows) => rows[0]?.["state"] === "completed");
    await fixture.start(() => digest.start({ topic: "d" }, { key: "daily", ...actor() }));
    const rows = await fixture.until(
        (all) => all.length === 2 && all[1]?.["state"] === "completed",
    );
    expect({
        waiting: waiting.map((row) => [row["state"], row["input"]]),
        ran: [ran?.["startedAt"], ran?.["output"]],
        outputs: rows.map((row) => row["output"]),
    }).toEqual({
        waiting: [["pending", { topic: "b" }]],
        ran: [at, "b"],
        outputs: ["b", "d"],
    });
});

test("pause, resume and cancel a run as its actor, a resumed run due at its sleep's end", async () => {
    const wait = defineWorkflow(
        { name: "wait", input: schema.object({}), output: NO_OUTPUT },
        NOTES,
    );
    const fixture = await openWorkflow(wait, async (step) => {
        await step.sleep("hold", { hours: 1 });
    });

    // start a sleeping run as alice, pause and resume it, then cancel it
    await fixture.start(() => wait.start({}, actor()));
    const [started] = await fixture.until((rows) => rows[0]?.["state"] === "waiting");
    const runs = fixture.objects(alice);
    const target = { id: String(started?.["id"]) };
    await runs.call(wait.run.name, "pause", target);
    const paused = await fixture.until(() => true);
    await runs.call(wait.run.name, "resume", target);
    const resumed = await fixture.until((rows) => rows[0]?.["state"] === "pending");
    await runs.call(wait.run.name, "cancel", target);
    const cancelled = await fixture.until(() => true);
    // resume it due at its sleep's end
    expect([
        paused[0]?.["state"],
        [resumed[0]?.["state"], resumed[0]?.["wakeAt"]],
        cancelled[0]?.["state"],
    ]).toEqual(["paused", ["pending", TEST_START + 60 * MINUTE], "cancelled"]);
});

test("reset a completed run to a step, running that step and the later ones again", async () => {
    const tally = defineWorkflow(
        {
            name: "tally",
            input: schema.object({}),
            output: schema.number(),
        },
        NOTES,
    );
    const counted: string[] = [];
    const fixture = await openWorkflow(tally, async (step) => {
        const first = await step.run("first", schema.number(), async () => {
            counted.push("first");

            return 1;
        });
        const second = await step.run("second", schema.number(), async () => {
            counted.push("second");

            return 2;
        });

        return first + second;
    });

    // complete a run, reset it to its second step as alice, and complete it again
    await fixture.start(() => tally.start({}, actor()));
    const [done] = await fixture.until((rows) => rows[0]?.["state"] === "completed");
    const runs = fixture.objects(alice);
    const id = String(done?.["id"]);
    const reset = await runs.call(tally.run.name, "reset", { id, step: "second" });
    const missing = await refusal(runs.call(tally.run.name, "reset", { id, step: "third" }));
    const [again] = await fixture.until((rows) => rows[0]?.["state"] === "completed");
    expect({ reset, missing, counted, output: again?.["output"] }).toEqual({
        reset: { forgotten: 1 },
        missing: ["NOT_FOUND", "STEP_NOT_FOUND", `run ${String(done?.["id"])} has no step third`],
        counted: ["first", "second", "second"],
        output: 3,
    });
});

test("start runs at the workflow's rate, and advance one singleton run of a key at a time", async () => {
    // start one run a minute, and hold one slot per space from a run's start to its end
    const report = defineWorkflow(
        {
            name: "report",
            input: schema.object({ title: schema.string() }),
            limits: { rate: { count: 1, every: { minutes: 1 } }, concurrency: { slots: 1 } },
            singleton: true,
            output: NO_OUTPUT,
        },
        NOTES,
    );
    const fixture = await openWorkflow(report, async (step, input) => {
        await step.call(
            "write",
            note.call("create", { spaceId: fixture.served.scope, title: input.title }),
        );
        await step.sleep("settle", { seconds: 30 });
    });

    // start three runs at once
    for (const title of ["a", "b", "c"]) {
        await fixture.start(() => report.start({ title }, actor(alice)));
    }
    const rows = await fixture.until(
        (all) => all.length === 3 && all.every((row) => row["state"] === "completed"),
    );

    // started a minute apart, each after the one before ended
    const started = rows
        .map((row) => Number(row["startedAt"]) - TEST_START)
        .toSorted((left, right) => left - right);
    expect(started).toEqual([0, MINUTE, 2 * MINUTE]);
});

test("take a call step hours after the run started, past a request's retry period", async () => {
    const late = defineWorkflow(
        { name: "late", input: schema.object({}), output: NO_OUTPUT },
        NOTES,
    );
    const fixture = await openWorkflow(late, async (step) => {
        await step.sleep("wait", { hours: 3 });
        await step.call(
            "write",
            note.call("create", { spaceId: fixture.served.scope, title: "late" }),
        );
    });

    // start a run that writes a note three hours in, and move to its wake
    await fixture.start(() => late.start({}, actor()));
    await fixture.simulation.jump(TEST_START + 180 * MINUTE);
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "completed");
    const titles = await fixture.served.database
        .select({ title: note.table.title })
        .from(note.table);
    expect([run?.["state"], titles]).toEqual(["completed", [{ title: "late" }]]);
});

test("fail a run acting for a user who granted its installation nothing, as no consent lets it act", async () => {
    const write = defineWorkflow(
        { name: "write", input: schema.object({}), output: NO_OUTPUT },
        NOTES,
    );
    const fixture = await openWorkflow(write, async (step) => {
        await step.call(
            "write",
            note.call("create", { spaceId: fixture.served.scope, title: "late" }),
        );
    });

    // start a run acting for carol, who never consented to the installation serving it
    const carol = principal.user.reference(
        Scope.universe.id,
        "user-01996ab0-0000-7000-8000-000000000102",
    );
    await fixture.start(() => write.start({}, actor(carol)));
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "failed");
    const { failure } = schema.looseObject({ failure: Failure }).parse(run);
    expect([failure.code, failure.message]).toEqual([
        "FORBIDDEN",
        `installation ${fixture.served.store.as(write.run).subject.id} has no grant to act for user ${carol.id}`,
    ]);
});

test("refuse a stranger controlling another's run", async () => {
    const idle = defineWorkflow(
        { name: "idle", input: schema.object({}), output: NO_OUTPUT },
        NOTES,
    );
    const fixture = await openWorkflow(idle, async (step) => {
        await step.sleep("hold", { hours: 1 });
    });

    // start alice's run, and pause it as bob
    await fixture.start(() => idle.start({}, actor()));
    const [started] = await fixture.until((rows) => rows[0]?.["state"] === "waiting");
    const target = { id: String(started?.["id"]) };
    expect(await refusal(fixture.objects(bob).call(idle.run.name, "pause", target))).toEqual([
        "NOT_FOUND",
        "NOT_FOUND",
        `idle-run ${String(started?.["id"])} not found`,
    ]);
});

test("fail a run whose function takes other steps on a later advance than it recorded", async () => {
    // take a step named after the advance, then sleep, so the second advance takes another name first
    const fickle = defineWorkflow(
        { name: "fickle", input: schema.object({}), output: NO_OUTPUT },
        NOTES,
    );
    let advances = 0;
    const fixture = await openWorkflow(fickle, async (step) => {
        advances += 1;
        const name = advances === 1 ? "first" : "second";
        await step.run(name, schema.number(), async () => advances);
        await step.sleep("pause", { minutes: 1 });
    });

    // expect the run failed for good at its second advance with the divergence, as the first step changed
    await fixture.start(() => fickle.start({}, actor()));
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "failed");
    expect([advances, run?.["failure"]]).toEqual([
        2,
        {
            code: "UNPROCESSABLE_CONTENT",
            reason: "RUN_NONDETERMINISTIC",
            domain: "@destack/workflow",
            message: `run ${String(run?.["id"])} takes step second where it recorded first`,
        },
    ]);
});

/** Stamps their owners issue through an external counter, which a preparing call reserves. */
const stamp = defineObject(
    {
        name: "stamp",
        plural: "stamps",
        scope: space,
        fields: { owner: field.subject().caller(), title: field.string() },
        permissions: { read: relation("owner"), write: relation("owner") },
    },
    (method) => ({
        create: method.create("write", { fields: ["title"] }),
        issue: method.mutation({
            permission: "write",
            prepared: schema.number(),
            output: schema.number(),
        }),
    }),
    NOTES,
);

test("make a preparing call once across a crash between its commit and its step's record", async () => {
    // issue each stamp through an external counter, which only the preparing phase advances
    let issued = 0;
    const issuing = stamp.implement({
        methods: {
            issue: {
                prepare: async () => {
                    issued += 1;

                    return issued;
                },
                handle: async (context) => context.prepared,
            },
        },
    });
    const press = defineWorkflow(
        {
            name: "press",
            input: schema.object({}),
            output: schema.number(),
        },
        NOTES,
    );
    const fixture: WorkflowFixture = await openWorkflow(
        press,
        async (step) => {
            const spaceId = fixture.served.scope;
            const created = await step.call(
                "create",
                stamp.call("create", { spaceId, title: "first" }),
            );

            return step.call("issue", stamp.call("issue", { spaceId, id: created.id }));
        },
        async () => ({}),
        [issuing],
    );
    await fixture.start(() => press.start({}, actor()));
    const [first] = await fixture.until((rows) => rows[0]?.["state"] === "completed");

    // rewrite rows, as no verb reaches the moment between a call's commit and its step's record
    //  leave the state a crash after the issue committed and before its step completed leaves: the step started, the run due
    const steps = press.step.table;
    const runs = press.run.table;
    await fixture.served.database.transaction(async (transaction) => {
        await transaction
            .update(steps)
            .set({ output: null, completedAt: null })
            .where(eq(steps[TABLE].column("name"), "issue"));
        await transaction
            .update(runs)
            .set({ state: "pending", output: null, wakeAt: null })
            .where(eq(runs[TABLE].column("id"), String(first?.["id"])));
    });
    const [again] = await fixture.until((rows) => rows[0]?.["state"] === "completed");

    // expect one issue, answered again by the journal under the request its started step derives
    expect([issued, first?.["output"], again?.["output"]]).toEqual([1, 1, 1]);
});

test("wait until a run ends and answer it as it ended, and refuse a wait its signal aborts first", async () => {
    // answer a number at once, or sleep an hour first
    const answer = defineWorkflow(
        {
            name: "answer",
            input: schema.object({ isSleeping: schema.boolean() }),
            output: schema.number(),
        },
        NOTES,
    );
    const fixture = await openWorkflow(answer, async (step, input) => {
        if (input.isSleeping) {
            await step.sleep("nap", { hours: 1 });
        }

        return step.run("answer", schema.number(), async () => 42);
    });
    const runOf = async (isSleeping: boolean) => {
        await fixture.start(() => answer.start({ isSleeping }, actor()));
        const rows = await fixture.served.database.select().from(answer.run.table);
        const row = rows.find(
            (each) =>
                schema.object({ isSleeping: schema.boolean() }).parse(each["input"]).isSleeping ===
                isSleeping,
        );

        return new Run(answer, present(row, "the started run"));
    };

    // wait for the quick run, and abort the wait for the sleeping one
    const ended = await (
        await runOf(false)
    ).ended(fixture.served.database, fixture.simulation.clock.timeout(5000));
    const aborted = new AbortController();
    const waiting = (await runOf(true)).ended(fixture.served.database, aborted.signal);
    aborted.abort(new Error("stopped waiting"));

    // expect the quick run completed with its answer, and the aborted wait refused
    expect([
        ended.row.state,
        ended.row.output,
        await waiting.catch((error: unknown) => String(error)),
    ]).toEqual(["completed", 42, "Error: stopped waiting"]);
});
