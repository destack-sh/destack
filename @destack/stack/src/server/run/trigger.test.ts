import { principal, Caller } from "@destack/access";
import { TEST_DIALECTS } from "@destack/db/test";
import { expect, refusal, test, vi } from "@destack/test";
import { ControlLoop } from "@destack/runtime/control";
import * as object from "../../object/index.ts";
import { aligned, present, Clock } from "@destack/schema";
import { endingOf, ids, pin, RunFixture, StackFixture, T0 } from "../../test/index.ts";
import { RequestId } from "@destack/runtime/request";
import type { ScheduleTiming } from "@destack/runtime/trigger";

/** Thirty days in milliseconds, beyond the longest delay a runtime timer takes. */
const MONTH = 30 * 24 * 60 * 60 * 1000;

/** Read the occurrence time of a schedule trigger's run, live or finished. */
function scheduledAt(entry: { readonly id: string; readonly event: object.Run["event"] }): number {
    const { event } = entry;
    if (event === null || !("at" in event)) {
        throw new TypeError(`run ${entry.id} is no occurrence of a schedule`);
    }

    return event.at;
}

/** Read a finished run as its live row's fields, its outcome as its state. */
function finishedRun({ id, keys, data }: object.FinishedRun) {
    return { id, state: keys.outcome, ...data };
}

/** Settle an attempt to the error message it fails with, or to accepted. */
function message(attempt: Promise<unknown>): Promise<string> {
    return attempt.then(
        () => "accepted",
        (error: { message: string }) => error.message,
    );
}

/** Describe finished then live runs by occurrence, state and attempts. */
async function describeRuns(fixture: RunFixture) {
    const finished = (await fixture.finished()).map((entry) => ({
        ...finishedRun(entry),
        ending: endingOf(entry.data.outcome),
    }));
    const live = (await fixture.runs()).map((entry) => ({
        ...entry,
        ending: entry.failure?.code ?? null,
    }));

    return [...finished, ...live]
        .toSorted((left, right) => scheduledAt(left) - scheduledAt(right))
        .map((entry) => [scheduledAt(entry) - T0, entry.state, entry.attempts, entry.ending]);
}

test.each(TEST_DIALECTS)(
    "run the latest missed occurrence within the deadline once and skip the others on %s",
    async (dialect) => {
        await using fixture = await RunFixture.open(dialect);
        const minutely = await fixture.trigger("minutely", {
            schedule: {
                timing: { timing: "interval", interval: 60_000, startsAt: T0 },
                concurrency: "allow",
                deadline: 90_000,
                call: pin,
            },
        });

        // run the first occurrence and fire five minutes later twice
        fixture.clock.set(T0 + 10_000);
        await fixture.fire(minutely);
        await fixture.attempt();
        fixture.clock.set(T0 + 310_000);
        await fixture.fire(minutely);
        await fixture.fire(minutely);
        await fixture.attempt();

        // skip the late and superseded occurrences and run the latest once as the installation
        expect(await describeRuns(fixture)).toEqual([
            [0, "succeeded", 1, null],
            [60_000, "skipped", 0, "late"],
            [120_000, "skipped", 0, "late"],
            [180_000, "skipped", 0, "late"],
            [240_000, "skipped", 0, "superseded"],
            [300_000, "succeeded", 1, null],
        ]);
        const runs = await fixture.finished();
        expect(fixture.pushed).toEqual(
            [aligned(runs, 0), aligned(runs, 5)].map((entry) => ({
                installation: entry.keys.installationId,
                mutation: { id: RunFixture.requestId(entry), calls: [entry.data.call] },
                exchange: null,
            })),
        );
    },
);

test.each(TEST_DIALECTS)(
    "run a calendar occurrence at nine in the schedule's time zone on %s",
    async (dialect) => {
        await using fixture = await RunFixture.open(dialect);
        const morning = await fixture.trigger("morning", {
            schedule: {
                timing: { timing: "cron", cron: "0 9 * * *", timezone: "Europe/Zurich" },
                concurrency: "forbid",
                deadline: 60_000,
                call: pin,
            },
        });

        // run nine o'clock summer time at seven o'clock UTC
        fixture.clock.set(Date.UTC(2026, 8, 27, 7, 0, 30));
        await fixture.fire(morning);
        await fixture.attempt();
        expect(
            (await fixture.finished()).map((entry) =>
                new Date(scheduledAt(finishedRun(entry))).toISOString(),
            ),
        ).toEqual(["2026-09-27T07:00:00.000Z"]);
    },
);

/** How each concurrency policy overlaps a running occurrence. */
const OVERLAPS = [
    [
        "allow",
        {
            started: [0, 60_000],
            aborted: [],
            runs: [
                [0, "succeeded", 1, null],
                [60_000, "succeeded", 1, null],
            ],
        },
    ],
    [
        "forbid",
        {
            started: [0],
            aborted: [],
            runs: [
                [0, "succeeded", 1, null],
                [60_000, "skipped", 0, "overlap"],
            ],
        },
    ],
    [
        "replace",
        {
            started: [0, 60_000],
            aborted: [0],
            runs: [
                [0, "skipped", 1, "replaced"],
                [60_000, "succeeded", 1, null],
            ],
        },
    ],
] as const;

test.each(
    TEST_DIALECTS.flatMap((dialect) =>
        OVERLAPS.map(([concurrency, expected]) => [concurrency, dialect, expected] as const),
    ),
)(
    "overlap a running occurrence under the %s policy on %s",
    async (concurrency, dialect, expected) => {
        // keep each pushed call waiting until released or cancelled
        const started: number[] = [];
        const aborted: number[] = [];
        const releases: (() => void)[] = [];
        const running = [Promise.withResolvers<void>(), Promise.withResolvers<void>()] as const;
        const cancelled = Promise.withResolvers<void>();
        await using fixture = await RunFixture.open(
            dialect,
            (_mutation, signal) =>
                new Promise((resolve, reject) => {
                    const at = fixture.clock.now() - T0;
                    started.push(at);
                    releases.push(() => resolve({ value: [null] }));
                    signal.addEventListener("abort", () => {
                        aborted.push(at);
                        reject(signal.reason);
                        cancelled.resolve();
                    });
                    aligned(running, started.length - 1).resolve();
                }),
        );
        const minutely = await fixture.trigger("minutely", {
            schedule: {
                timing: { timing: "interval", interval: 60_000, startsAt: T0 },
                concurrency,
                deadline: 30_000,
                call: pin,
            },
        });

        // start the first occurrence and move to the next
        const first = fixture.fire(minutely).then(() => fixture.attempt());
        await running[0].promise;
        fixture.clock.set(T0 + 60_000);

        // fire the next occurrence while the first runs and await a replaced call's cancel
        const second = fixture.fire(minutely).then(() => fixture.attempt());
        await (expected.started.length === 2 ? running[1].promise : second);
        await (expected.aborted.length === 1 ? cancelled.promise : undefined);

        // release every started call
        for (const release of releases) {
            release();
        }
        await Promise.all([first, second]);
        expect([started, aborted, await describeRuns(fixture)]).toEqual([
            expected.started,
            expected.aborted,
            expected.runs,
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "record at most one hundred missed occurrences after a long outage, summarising the earlier ones on %s",
    async (dialect) => {
        await using fixture = await RunFixture.open(dialect);
        const minutely = await fixture.trigger("minutely", {
            schedule: {
                timing: { timing: "interval", interval: 60_000, startsAt: T0 },
                concurrency: "allow",
                deadline: 30_000,
                call: pin,
            },
        });

        // run the first occurrence and fire after five hundred missed minutes
        fixture.clock.set(T0 + 6_000);
        await fixture.fire(minutely);
        await fixture.attempt();
        fixture.clock.set(T0 + 30_006_000);
        await fixture.fire(minutely);
        await fixture.attempt();

        // record the first run, a summary of the omitted occurrences and the latest ninety-nine with all but the last past their deadline
        const late = Array.from({ length: 98 }, (_, index) => [
            (402 + index) * 60_000,
            "skipped",
            0,
            "late",
        ]);
        expect(await describeRuns(fixture)).toEqual([
            [0, "succeeded", 1, null],
            [401 * 60_000, "skipped", 0, "missed"],
            ...late,
            [500 * 60_000, "succeeded", 1, null],
        ]);
        expect(aligned(await fixture.finished(), 1).data.outcome).toEqual({
            kind: "skipped",
            skip: { reason: "missed", earlier: 400 },
        });
    },
);

test.each(TEST_DIALECTS)(
    "sleep until an occurrence beyond the longest timer, firing once meanwhile on %s",
    async (dialect) => {
        await using fixture = await RunFixture.open(dialect);
        await fixture.trigger("monthly", {
            schedule: {
                timing: { timing: "once", startsAt: T0 + MONTH },
                concurrency: "forbid",
                deadline: 60_000,
                call: pin,
            },
        });

        // run the controllers until the schedule is looked at once
        const controller = present(
            fixture.store.controllers().find((entry) => entry.name === "trigger"),
            "the trigger controller",
        );
        const looked = Promise.withResolvers<void>();
        const reconcileOnce = controller.reconcile.bind(controller);
        const reconcile = vi
            .spyOn(controller, "reconcile")
            .mockImplementation(async (key, reconciliation) => {
                const delay = await reconcileOnce(key, reconciliation);
                looked.resolve();

                return delay;
            });
        const loop = new ControlLoop(fixture.database, [controller], {
            clock: Clock.system,
            report: (_controller, _key, error) => fixture.reports.push(error),
        });
        const stop = new AbortController();
        const running = loop.run(stop.signal);
        await looked.promise;

        // sleep through the rest without a run
        expect([reconcile.mock.calls.length, await fixture.runs(), fixture.reports]).toEqual([
            1,
            [],
            [],
        ]);
        stop.abort();
        await running;
    },
);

test.each(TEST_DIALECTS)(
    "leave the schedules of a suspended installation until it is enabled again on %s",
    async (dialect) => {
        await using fixture = await RunFixture.open(dialect);
        const minutely = await fixture.trigger("minutely", {
            schedule: {
                timing: { timing: "interval", interval: 60_000, startsAt: T0 },
                concurrency: "allow",
                deadline: 90_000,
                call: pin,
            },
        });

        // fire nothing while the installation is suspended and the due occurrence once enabled
        await fixture.suspend(true);
        fixture.clock.set(T0 + 10_000);
        expect(await fixture.fire(minutely)).toBeUndefined();
        await fixture.suspend(false);
        await fixture.fire(minutely);
        expect(await describeRuns(fixture)).toEqual([[0, "pending", 0, null]]);
    },
);

test.each(TEST_DIALECTS)(
    "keep declared schedule triggers under their stack's control on %s",
    async (dialect) => {
        await using fixture = await RunFixture.open(dialect);
        fixture.clock.set(T0 + 10_000);
        const timing = { timing: "interval", interval: 60_000, startsAt: T0 } as const;
        const schedule = { timing, concurrency: "allow", deadline: 90_000, call: pin } as const;
        const declared = await fixture.trigger("daily", { schedule });

        // create a trigger of the same name as the installation beside the declared one
        const { store } = fixture;
        const notes = principal.installation.reference(ids.space, ids.notes);
        const created = aligned(
            await store.mutate(Caller.of(notes, fixture.clock.now()), {
                id: RequestId.create(fixture.clock.now()),
                calls: [
                    object.trigger.call("create", {
                        ...object.trigger.in(ids.space),
                        installationId: ids.notes,
                        name: "daily",
                        on: { schedule },
                    }),
                ],
            }),
            0,
        );

        // refuse creating a webhook trigger
        const unscheduled = await message(
            store.mutate(Caller.of(notes, fixture.clock.now()), {
                id: RequestId.create(fixture.clock.now()),
                calls: [
                    object.trigger.call("create", {
                        ...object.trigger.in(ids.space),
                        installationId: ids.notes,
                        name: "pushes",
                        on: { webhook: { verification: "github", path: "/" } },
                    }),
                ],
            }),
        );
        // refuse changing the declared one and pause it
        const refused = await message(
            store.mutate(Caller.of(notes, fixture.clock.now()), {
                id: RequestId.create(fixture.clock.now()),
                calls: [
                    object.trigger.call("update", {
                        ...object.trigger.target(declared),
                        on: { schedule: { ...schedule, call: { ...pin, input: {} } } },
                    }),
                ],
            }),
        );
        await store.mutate(Caller.of(notes, fixture.clock.now()), {
            id: RequestId.create(fixture.clock.now()),
            calls: [object.trigger.call("pause", object.trigger.target(declared))],
        });

        // fire both with the paused one recording nothing and delete the created one with its waiting run
        await fixture.fire(declared);
        await fixture.fire(created);
        await store.mutate(Caller.of(notes, fixture.clock.now()), {
            id: RequestId.create(fixture.clock.now()),
            calls: [object.trigger.call("delete", object.trigger.target(created))],
        });

        expect([
            refused,
            await fixture.runs(),
            (await fixture.finished()).map((entry) => [
                entry.keys.triggerId,
                entry.data.triggerName,
                scheduledAt(finishedRun(entry)) - T0,
                entry.keys.outcome,
                entry.data.outcome,
            ]),
        ]).toEqual([
            "trigger daily is declared by its build, which changes it",
            [],
            [
                [
                    created.id,
                    "daily",
                    0,
                    "skipped",
                    { kind: "skipped", skip: { reason: "cancelled" } },
                ],
            ],
        ]);
        expect(unscheduled).toEqual("a created trigger fires on a schedule");
    },
);

test("refuse a created schedule trigger occurring more than once a minute", async () => {
    // serve the space keeping the notes installation
    const fixture = await StackFixture.open();
    const now = Date.now();
    await fixture.database.insert(object.installation.table).values({
        id: ids.notes,
        scope: ids.space,
        packageId: ids.package,
        role: "application",
        name: "notes",
        selection: { kind: "release", version: "2026.9.0" },
        createdAt: now,
        updatedAt: now,
    });
    const client = await fixture.serve();

    // refuse a schedule occurring every second and create a daily one
    const create = (timing: ScheduleTiming) =>
        refusal(
            client(ids.notes).trigger.create({
                spaceId: ids.space,
                requestId: RequestId.create(Date.now()),
                installationId: ids.notes,
                name: "digest",
                on: { schedule: { timing, concurrency: "forbid", deadline: 60_000, call: pin } },
            }),
        ).then((refused) => (refused === "done" ? "created" : refused));

    expect([
        await create({ timing: "interval", interval: 1_000, startsAt: 0 }),
        await create({ timing: "cron", cron: "0 9 * * *", timezone: "Europe/Vienna" }),
    ]).toEqual([
        [
            "BAD_REQUEST",
            "invalid input: on.schedule.timing.interval: too small: expected number to be >=60000",
        ],
        "created",
    ]);
});
