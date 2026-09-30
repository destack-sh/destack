import { TEST_DIALECTS } from "@destack/db/test";
import { expect, test } from "@destack/test";
import { vi } from "vitest";
import { ControlLoop } from "@destack/service/control";
import { SystemCall } from "@destack/object/server";
import * as object from "../src/object/index.ts";
import { ids } from "./fixture/space.ts";
import { pin, RunFixture, T0 } from "./fixture/run.ts";

/** Thirty days, beyond the longest delay a runtime timer takes, in milliseconds. */
const MONTH = 30 * 24 * 60 * 60 * 1000;

/** Describe runs by their occurrence, state, attempts and error code. */
async function describeRuns(fixture: RunFixture) {
    return (await fixture.runs()).map((entry) => [
        entry.scheduledAt! - T0,
        entry.state,
        entry.attempts,
        entry.error?.code ?? null,
    ]);
}

test.each(TEST_DIALECTS)(
    "run the latest missed occurrence within the deadline once and skip the others on %s",
    async (dialect) => {
        await using fixture = await RunFixture.open(dialect);
        const minutely = await fixture.schedule({
            name: "minutely",
            timing: { timing: "interval", interval: 60_000, startsAt: T0 },
            concurrency: "allow",
            deadline: 90_000,
        });

        // run the first occurrence, then fire five minutes later twice
        fixture.now = T0 + 10_000;
        await fixture.fire(minutely);
        await fixture.attempt();
        fixture.now = T0 + 310_000;
        await fixture.fire(minutely);
        await fixture.fire(minutely);
        await fixture.attempt();

        // skip the late and superseded occurrences, running the latest once as the installation
        expect(await describeRuns(fixture)).toEqual([
            [0, "succeeded", 1, null],
            [60_000, "skipped", 0, "DEADLINE_EXCEEDED"],
            [120_000, "skipped", 0, "DEADLINE_EXCEEDED"],
            [180_000, "skipped", 0, "DEADLINE_EXCEEDED"],
            [240_000, "skipped", 0, "SUPERSEDED"],
            [300_000, "succeeded", 1, null],
        ]);
        const runs = await fixture.runs();
        expect(fixture.pushed).toEqual(
            [runs[0]!, runs[5]!].map((entry) => ({
                installation: entry.installation,
                mutation: { id: RunFixture.requestOf(entry), calls: [entry.call] },
            })),
        );
    },
);

test.each(TEST_DIALECTS)(
    "run a calendar occurrence at nine in the schedule's time zone on %s",
    async (dialect) => {
        await using fixture = await RunFixture.open(dialect);
        const morning = await fixture.schedule({
            name: "morning",
            timing: { timing: "cron", cron: "0 9 * * *", timezone: "Europe/Zurich" },
            concurrency: "forbid",
            deadline: 60_000,
        });

        // run nine o'clock summer time, seven o'clock UTC
        fixture.now = Date.UTC(2026, 8, 27, 7, 0, 30);
        await fixture.fire(morning);
        await fixture.attempt();
        expect(
            (await fixture.runs()).map((entry) => new Date(entry.scheduledAt!).toISOString()),
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
                [60_000, "skipped", 0, "OVERLAP"],
            ],
        },
    ],
    [
        "replace",
        {
            started: [0, 60_000],
            aborted: [0],
            runs: [
                [0, "failed", 1, "REPLACED"],
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
        const running = [Promise.withResolvers<void>(), Promise.withResolvers<void>()];
        const cancelled = Promise.withResolvers<void>();
        await using fixture = await RunFixture.open(
            dialect,
            (_mutation, signal) =>
                new Promise((resolve, reject) => {
                    const at = fixture.now - T0;
                    started.push(at);
                    releases.push(() => resolve({ value: [null] }));
                    signal.addEventListener("abort", () => {
                        aborted.push(at);
                        reject(signal.reason);
                        cancelled.resolve();
                    });
                    running[started.length - 1]!.resolve();
                }),
        );
        const minutely = await fixture.schedule({
            name: "minutely",
            timing: { timing: "interval", interval: 60_000, startsAt: T0 },
            concurrency,
            deadline: 30_000,
        });

        // start the first occurrence, and move to the next
        const first = fixture.fire(minutely).then(() => fixture.attempt());
        await running[0]!.promise;
        fixture.now = T0 + 60_000;

        // fire the next occurrence while the first runs, awaiting a replaced call's cancel
        const second = fixture.fire(minutely).then(() => fixture.attempt());
        await (expected.started.length === 2 ? running[1]!.promise : second);
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
        const minutely = await fixture.schedule({
            name: "minutely",
            timing: { timing: "interval", interval: 60_000, startsAt: T0 },
            concurrency: "allow",
            deadline: 30_000,
        });

        // run the first occurrence, then fire after five hundred missed minutes
        fixture.now = T0 + 6_000;
        await fixture.fire(minutely);
        await fixture.attempt();
        fixture.now = T0 + 30_006_000;
        await fixture.fire(minutely);
        await fixture.attempt();

        // record the first run, a summary of the omitted occurrences, then the latest ninety-nine, all but the last past their deadline
        const late = Array.from({ length: 98 }, (_, index) => [
            (402 + index) * 60_000,
            "skipped",
            0,
            "DEADLINE_EXCEEDED",
        ]);
        expect(await describeRuns(fixture)).toEqual([
            [0, "succeeded", 1, null],
            [401 * 60_000, "skipped", 0, "MISSED_OCCURRENCES"],
            ...late,
            [500 * 60_000, "succeeded", 1, null],
        ]);
        expect((await fixture.runs())[1]!.error!.message).toBe(
            "401 occurrences were missed up to this one",
        );
    },
);

test.each(TEST_DIALECTS)(
    "sleep until an occurrence beyond the longest timer, firing once meanwhile on %s",
    async (dialect) => {
        await using fixture = await RunFixture.open(dialect);
        await fixture.schedule({
            name: "monthly",
            timing: { timing: "once", startsAt: T0 + MONTH },
            concurrency: "forbid",
            deadline: 60_000,
        });

        // run the controllers until the schedule is looked at once
        const controller = fixture.server.controllers().find((entry) => entry.name === "schedule")!;
        const reconcile = vi.spyOn(controller, "reconcile");
        const loop = new ControlLoop(fixture.database, [controller], {
            report: (_controller, _key, error) => fixture.reports.push(error),
        });
        const stop = new AbortController();
        const running = loop.run(stop.signal);
        await vi.waitFor(() => expect(reconcile).toHaveBeenCalledTimes(1));

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
        const minutely = await fixture.schedule({
            name: "minutely",
            timing: { timing: "interval", interval: 60_000, startsAt: T0 },
            concurrency: "allow",
            deadline: 90_000,
        });

        // fire nothing while the installation is suspended, then the due occurrence once enabled
        await fixture.suspend(true);
        fixture.now = T0 + 10_000;
        expect(await fixture.fire(minutely)).toBeUndefined();
        await fixture.suspend(false);
        await fixture.fire(minutely);
        expect(await describeRuns(fixture)).toEqual([[0, "pending", 0, null]]);
    },
);

test.each(TEST_DIALECTS)(
    "keep declared and created schedules apart, refuse changing a declared one, pause it, and keep a deleted schedule's runs on %s",
    async (dialect) => {
        await using fixture = await RunFixture.open(dialect);
        fixture.now = T0 + 10_000;
        const timing = { timing: "interval", interval: 60_000, startsAt: T0 } as const;
        const fields = { name: "daily", timing, concurrency: "allow", deadline: 90_000 } as const;
        const declared = await fixture.schedule(fields);

        // create a schedule of the same name as the installation, beside the declared one
        const system = (name: string, calls: readonly SystemCall[]) =>
            fixture.server.executeAsSystem(object.schedule, name, calls, fixture.now);
        const [created] = (await system("create", [
            { scope: ids.space, input: { installation: ids.notes, call: pin, ...fields } },
        ])) as object.Schedule[];

        // refuse changing the declared one, and pause it
        const refusal = await system("update", [
            { ...SystemCall.of(declared), input: { call: { ...pin, input: {} } } },
        ]).then(
            () => "changed",
            (error: { message: string }) => error.message,
        );
        await system("pause", [{ ...SystemCall.of(declared), input: { isPaused: true } }]);

        // fire both, the paused one recording nothing, then delete the created one keeping its run
        await fixture.fire(declared);
        await fixture.fire(created!);
        await system("delete", [SystemCall.of({ ...created!, revision: created!.revision })]);

        expect([
            refusal,
            (await fixture.runs()).map((entry) => [
                entry.cause,
                entry.schedule,
                entry.scheduledAt! - T0,
            ]),
        ]).toEqual([
            "schedule daily is declared by its build, which changes it",
            [["schedule", null, 0]],
        ]);
    },
);
