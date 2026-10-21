import { principal, relation } from "@destack/access";
import { space } from "@destack/account/object";
import { eq, TABLE } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { present, schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { Failure } from "@destack/error";
import { expect, refusal, test } from "@destack/test";
import { defineWorkflow, Run } from "../src/index.ts";
import { actor, alice, note, openWorkflow, START, type WorkflowFixture } from "./fixture.ts";

/** A minute, in milliseconds. */
const MINUTE = 60_000;

test("retry a failing advance after its backoff, and fail once the attempts run out", async () => {
    // fail twice before succeeding, with two attempts for the second workflow
    const flaky = defineWorkflow({
        name: "flaky",
        input: schema.object({ succeedAt: schema.number() }),
        output: schema.number(),
        retries: { initialInterval: 1000, backoffCoefficient: 2 },
    });
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
        { code: "INTERNAL_SERVER_ERROR", message: "attempt 2 failed" },
    ]);
});

test("fail a run once its attempts run out, keeping the last failure", async () => {
    const brittle = defineWorkflow({
        name: "brittle",
        input: schema.object({}),
        retries: { initialInterval: 1000, maximumAttempts: 2 },
    });
    const fixture = await openWorkflow(brittle, async () => {
        throw new Error("always");
    });

    // start a run that fails both attempts
    await fixture.start(() => brittle.start({}, actor()));
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "failed");
    expect([run?.["attempts"], run?.["failure"]]).toEqual([
        1,
        { code: "INTERNAL_SERVER_ERROR", message: "always" },
    ]);
});

test("time a run out at its deadline while it sleeps past it", async () => {
    const slow = defineWorkflow({ name: "slow", input: schema.object({}), timeout: { hours: 1 } });
    const fixture = await openWorkflow(slow, async (step) => {
        await step.sleep("nap", { hours: 2 });
    });

    // start a run sleeping two hours under an hour's timeout
    await fixture.start(() => slow.start({}, actor()));
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "failed");
    expect([run?.["deadline"], run?.["failure"]]).toEqual([
        START + 60 * MINUTE,
        { code: "TIMEOUT", message: `run ${String(run?.["id"])} timed out` },
    ]);
});

test("start a run at its time, answer the live run of a key, and replace one that has not begun", async () => {
    const digest = defineWorkflow({
        name: "digest",
        input: schema.object({ topic: schema.string() }),
        output: schema.string(),
    });
    const fixture = await openWorkflow(digest, async (step, input) =>
        step.run("send", schema.string(), async () => input.topic),
    );

    // start a run in ten minutes, replace its topic, and start the key again
    const at = START + 10 * MINUTE;
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
    const wait = defineWorkflow({ name: "wait", input: schema.object({}) });
    const fixture = await openWorkflow(wait, async (step) => {
        await step.sleep("hold", { hours: 1 });
    });

    // start a sleeping run as alice, pause and resume it, then cancel it
    await fixture.start(() => wait.start({}, actor()));
    const [started] = await fixture.until((rows) => rows[0]?.["state"] === "waiting");
    const target = wait.run.target({ scope: fixture.spaceId, id: String(started?.["id"]) });
    await fixture.asAlice([wait.run.call("pause", target)]);
    const paused = await fixture.until(() => true);
    await fixture.asAlice([wait.run.call("resume", target)]);
    const resumed = await fixture.until((rows) => rows[0]?.["state"] === "pending");
    await fixture.asAlice([wait.run.call("cancel", target)]);
    const cancelled = await fixture.until(() => true);
    // resume it due at its sleep's end
    expect([
        paused[0]?.["state"],
        [resumed[0]?.["state"], resumed[0]?.["wakeAt"]],
        cancelled[0]?.["state"],
    ]).toEqual(["paused", ["pending", START + 60 * MINUTE], "cancelled"]);
});

test("reset a completed run to a step, running that step and the later ones again", async () => {
    const tally = defineWorkflow({
        name: "tally",
        input: schema.object({}),
        output: schema.number(),
    });
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
    const target = tally.run.target({ scope: fixture.spaceId, id: String(done?.["id"]) });
    const [reset] = await fixture.asAlice([tally.run.call("reset", { ...target, step: "second" })]);
    const missing = await refusal(
        fixture.asAlice([tally.run.call("reset", { ...target, step: "third" })]),
    );
    const [again] = await fixture.until((rows) => rows[0]?.["state"] === "completed");
    expect({ reset, missing, counted, output: again?.["output"] }).toEqual({
        reset: { forgotten: 1 },
        missing: ["NOT_FOUND", `run ${String(done?.["id"])} took no step third`],
        counted: ["first", "second", "second"],
        output: 3,
    });
});

test("start runs at the workflow's rate, and advance one singleton run of a key at a time", async () => {
    // start one run a minute, and hold one slot per space from a run's start to its end
    const report = defineWorkflow({
        name: "report",
        input: schema.object({ title: schema.string() }),
        limits: { rate: { count: 1, every: { minutes: 1 } }, concurrency: { slots: 1 } },
        singleton: true,
    });
    const fixture = await openWorkflow(report, async (step, input) => {
        await step.call(
            "write",
            note.call("create", { spaceId: fixture.spaceId, title: input.title }),
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
        .map((row) => Number(row["startedAt"]) - START)
        .toSorted((left, right) => left - right);
    expect(started).toEqual([0, MINUTE, 2 * MINUTE]);
});

test("take a call step hours after the run started, past a request's retry period", async () => {
    const late = defineWorkflow({ name: "late", input: schema.object({}) });
    const fixture = await openWorkflow(late, async (step) => {
        await step.sleep("wait", { hours: 3 });
        await step.call("write", note.call("create", { spaceId: fixture.spaceId, title: "late" }));
    });

    // start a run that writes a note three hours in
    await fixture.start(() => late.start({}, actor()));
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "completed");
    const titles = await fixture.store.database
        .select({ title: note.table.title })
        .from(note.table);
    expect([run?.["state"], titles]).toEqual(["completed", [{ title: "late" }]]);
});

test("fail a run acting for a user who granted its installation nothing, as no consent lets it act", async () => {
    const write = defineWorkflow({ name: "write", input: schema.object({}) });
    const fixture = await openWorkflow(write, async (step) => {
        await step.call("write", note.call("create", { spaceId: fixture.spaceId, title: "late" }));
    });

    // start a run acting for carol, who never consented to the installation
    const carol = principal.user.reference(
        Scope.universe.id,
        "user-01996ab0-0000-7000-8000-000000000102",
    );
    await fixture.start(() => write.start({}, actor(carol)));
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "failed");
    const { failure } = schema.looseObject({ failure: Failure }).parse(run);
    expect([failure.code, failure.message]).toEqual([
        "FORBIDDEN",
        `installation installation-01996ab0-0000-7000-8000-000000000201 acts for user ${carol.id} under no grant`,
    ]);
});

test("refuse a stranger controlling another's run", async () => {
    const idle = defineWorkflow({ name: "idle", input: schema.object({}) });
    const fixture = await openWorkflow(idle, async (step) => {
        await step.sleep("hold", { hours: 1 });
    });

    // start alice's run, and pause it as the store acting for bob
    await fixture.start(() => idle.start({}, actor()));
    const [started] = await fixture.until((rows) => rows[0]?.["state"] === "waiting");
    const target = idle.run.target({ scope: fixture.spaceId, id: String(started?.["id"]) });
    expect(await refusal(fixture.asBob([idle.run.call("pause", target)]))).toEqual([
        "NOT_FOUND",
        `no idle-run ${String(started?.["id"])}`,
    ]);
});

test("fail a run whose function takes other steps on a later advance than it recorded", async () => {
    // take a step named after the advance, then sleep, so the second advance takes another name first
    const fickle = defineWorkflow({
        name: "fickle",
        input: schema.object({}),
        retries: { maximumAttempts: 1 },
    });
    let advances = 0;
    const fixture = await openWorkflow(fickle, async (step) => {
        advances += 1;
        const name = advances === 1 ? "first" : "second";
        await step.run(name, schema.number(), async () => advances);
        await step.sleep("pause", { minutes: 1 });
    });

    // expect the run failed at its second advance with the divergence, as the first step changed
    await fixture.start(() => fickle.start({}, actor()));
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "failed");
    expect([advances, run?.["failure"]]).toEqual([
        2,
        {
            code: "INTERNAL_SERVER_ERROR",
            message: `run ${String(run?.["id"])} diverged from its recorded steps: it takes second where it recorded first, as its function is not deterministic between steps`,
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
    const press = defineWorkflow({
        name: "press",
        input: schema.object({}),
        output: schema.number(),
    });
    const fixture: WorkflowFixture = await openWorkflow(
        press,
        async (step) => {
            const { spaceId } = fixture;
            const created = await step.call(
                "create",
                stamp.call("create", { spaceId, title: "first" }),
            );

            return step.call("issue", stamp.call("issue", { spaceId, id: created.id }));
        },
        {},
        [issuing],
    );
    await fixture.start(() => press.start({}, actor()));
    const [first] = await fixture.until((rows) => rows[0]?.["state"] === "completed");

    // leave the state a crash after the issue committed and before its step completed leaves: the step started, the run due
    const steps = press.step.table;
    await fixture.store.database
        .update(steps)
        .set({ output: null, completedAt: null })
        .where(eq(steps[TABLE].column("name"), "issue"));
    const runs = press.run.table;
    await fixture.store.database
        .update(runs)
        .set({ state: "pending", output: null, wakeAt: null })
        .where(eq(runs[TABLE].column("id"), String(first?.["id"])));
    const [again] = await fixture.until((rows) => rows[0]?.["state"] === "completed");

    // expect one issue, answered again by the journal under the request its started step derives
    expect([issued, first?.["output"], again?.["output"]]).toEqual([1, 1, 1]);
});

test("wait until a run ends and answer it as it ended, and refuse a wait its signal aborts first", async () => {
    // answer a number at once, or sleep an hour first
    const answer = defineWorkflow({
        name: "answer",
        input: schema.object({ isSleeping: schema.boolean() }),
        output: schema.number(),
    });
    const fixture = await openWorkflow(answer, async (step, input) => {
        if (input.isSleeping) {
            await step.sleep("nap", { hours: 1 });
        }

        return step.run("answer", schema.number(), async () => 42);
    });
    const runOf = async (isSleeping: boolean) => {
        await fixture.start(() => answer.start({ isSleeping }, actor()));
        const rows = await fixture.store.database.select().from(answer.run.table);
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
    ).ended(fixture.store.database, AbortSignal.timeout(5000));
    const aborted = new AbortController();
    const waiting = (await runOf(true)).ended(fixture.store.database, aborted.signal);
    aborted.abort(new Error("stopped waiting"));

    // expect the quick run completed with its answer, and the aborted wait refused
    expect([
        ended.state,
        ended.row["output"],
        await waiting.catch((error: unknown) => String(error)),
    ]).toEqual(["completed", 42, "Error: stopped waiting"]);
});
