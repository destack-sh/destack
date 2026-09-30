import { TEST_DIALECTS } from "@destack/db/test";
import { expect, test } from "@destack/test";
import { vi } from "vitest";
import { ServiceError } from "@destack/service/error";
import { VERSION_HEADER } from "@destack/service/request";
import { SystemCall } from "@destack/object/server";
import { eq } from "@destack/db";
import { principal } from "@destack/access";
import { v7 } from "uuid";
import * as object from "../src/object/index.ts";
import { pushThrough } from "../src/server/index.ts";
import type * as sync from "@destack/sync";
import { ids, reconcile } from "./fixture/space.ts";
import { type Answer, pin, RunFixture, T0 } from "./fixture/run.ts";

/** Pin one note, told apart by its identifier. */
function pinning(id: string) {
    return { ...pin, input: { id, pinned: true } };
}

/** Read the note a pushed mutation pins. */
function pinned(mutation: sync.Mutation): string {
    return mutation.calls[0]!.input.id as string;
}

test.each(TEST_DIALECTS)(
    "push each run's call as its installation under the run's own request, retrying transient failures and failing final ones on %s",
    async (dialect) => {
        // succeed, fail once transiently, refuse finally, and stay unreachable
        const tries = new Map<string, number>();
        const answer: Answer = async (mutation) => {
            const note = pinned(mutation);
            tries.set(note, (tries.get(note) ?? 0) + 1);
            if (note === "flaky" && tries.get(note) === 1) {
                throw new ServiceError("UNAVAILABLE", { message: "the instance is starting" });
            } else if (note === "refused") {
                return {
                    error: { code: "FORBIDDEN", status: 403, message: "permission denied: update" },
                };
            } else if (note === "unreachable") {
                throw new ServiceError("UNAVAILABLE", {
                    message: "no instance serves the release",
                });
            }

            return { value: [null] };
        };
        await using fixture = await RunFixture.open(dialect, answer, { maximumAttempts: 2 });
        for (const note of ["first", "flaky", "refused", "unreachable"]) {
            await fixture.send({ call: pinning(note) });
        }

        // attempt them until every run finished
        fixture.serve();
        await fixture.settle();
        const runs = await fixture.runs();
        expect(
            runs.map((entry) => [
                pinned({ id: "", calls: [entry.call] }),
                entry.state,
                entry.attempts,
                entry.error?.code ?? null,
            ]),
        ).toEqual([
            ["first", "succeeded", 1, null],
            ["flaky", "succeeded", 2, null],
            ["refused", "failed", 1, "FORBIDDEN"],
            ["unreachable", "failed", 2, "UNAVAILABLE"],
        ]);

        // push every attempt of a run under the run's own request, to its installation
        const requests = new Map(
            runs.map((entry) => [
                pinned({ id: "", calls: [entry.call] }),
                RunFixture.requestOf(entry),
            ]),
        );
        expect(
            fixture.pushed
                .map(({ installation, mutation }) => [
                    installation,
                    pinned(mutation),
                    mutation.id === requests.get(pinned(mutation)),
                ])
                .sort((left, right) => left.join().localeCompare(right.join())),
        ).toEqual(
            ["first", "flaky", "flaky", "refused", "unreachable", "unreachable"].map((note) => [
                ids.notes,
                note,
                true,
            ]),
        );
    },
);

test.each(TEST_DIALECTS)(
    "attempt a watch's runs one at a time in the order recorded on %s",
    async (dialect) => {
        // keep each pushed call waiting until released
        const releases = new Map<string, () => void>();
        const answer: Answer = (mutation) =>
            new Promise((resolve) =>
                releases.set(pinned(mutation), () => resolve({ value: [null] })),
            );
        await using fixture = await RunFixture.open(dialect, answer);
        for (const [sequence, note] of ["first", "second", "third"].entries()) {
            await fixture.send({
                call: pinning(note),
                cause: "watch",
                packageId: ids.package,
                trigger: "pinned",
                epoch: "epoch-1",
                sequence,
            });
        }

        // push each queued call only once the one before it finished
        fixture.serve();
        const order: string[] = [];
        for (const note of ["first", "second", "third"]) {
            await vi.waitFor(() => expect([...releases.keys()]).toEqual([...order, note]));
            order.push(note);
            releases.get(note)!();
        }
        await fixture.settle();
        expect((await fixture.runs()).map((entry) => entry.state)).toEqual([
            "succeeded",
            "succeeded",
            "succeeded",
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "wait for a run until its time, with the runs of a suspended installation until it is enabled again, and until an instance runs on %s",
    async (dialect) => {
        await using fixture = await RunFixture.open(dialect);
        const later = await fixture.send({ at: T0 + 60_000 });

        // wait until the run's time, and leave it while the installation is suspended
        const attempt = () => reconcile(fixture.server, "run", { id: later.id });
        const early = await attempt();
        fixture.now = T0 + 60_000;
        await fixture.suspend(true);
        const suspended = await attempt();
        const waiting = fixture.pushed.length;

        // leave it while the installation runs no instance
        await fixture.suspend(false);
        await fixture.stop(true);
        const stopped = await attempt();
        const idle = fixture.pushed.length;

        // push it once its instance runs again
        await fixture.stop(false);
        await attempt();
        expect([early, suspended, waiting, stopped, idle, fixture.pushed.length]).toEqual([
            60_000,
            undefined,
            0,
            undefined,
            0,
            1,
        ]);
        expect((await fixture.runs()).map((entry) => entry.state)).toEqual(["succeeded"]);
    },
);

test.each(TEST_DIALECTS)(
    "attempt runs recorded while the loop runs: a sent call, a fired schedule and a watch's queue past a failed change on %s",
    async (dialect) => {
        // refuse the call pinning the second watched note for good
        const answer: Answer = async (mutation) =>
            pinned(mutation) === "refused"
                ? {
                      error: {
                          code: "FORBIDDEN",
                          status: 403,
                          message: "permission denied: update",
                      },
                  }
                : { value: [null] };
        await using fixture = await RunFixture.open(dialect, answer);
        fixture.now = Date.now();
        fixture.serve();

        // send a call, fire a schedule, and queue three watched changes, all while the loop runs
        await fixture.send({ call: pinning("sent") });
        await fixture.schedule({
            name: "soon",
            timing: { timing: "once", startsAt: fixture.now },
            concurrency: "allow",
            deadline: 60_000,
        });
        for (const [sequence, note] of ["first", "refused", "third"].entries()) {
            await fixture.send({
                call: pinning(note),
                cause: "watch",
                packageId: ids.package,
                trigger: "pinned",
                epoch: "epoch-1",
                sequence,
            });
        }
        await vi.waitFor(async () => expect((await fixture.runs()).length).toBe(5));
        await fixture.settle();

        const described = (await fixture.runs()).map((entry) => [
            entry.cause,
            pinned({ id: "", calls: [entry.call] }),
            entry.state,
            entry.error?.code ?? null,
        ]);
        expect(described.sort((left, right) => left.join().localeCompare(right.join()))).toEqual([
            ["schedule", "note-1", "succeeded", null],
            ["send", "sent", "succeeded", null],
            ["watch", "first", "succeeded", null],
            ["watch", "refused", "failed", "FORBIDDEN"],
            ["watch", "third", "succeeded", null],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "fail a run whose attempts ran out without finishing, and abort a running attempt once the run is cancelled on %s",
    async (dialect) => {
        // keep each push waiting until aborted
        const aborted: string[] = [];
        const answer: Answer = (mutation, signal) =>
            new Promise((_resolve, reject) => {
                signal.addEventListener("abort", () => {
                    aborted.push(pinned(mutation));
                    reject(signal.reason);
                });
            });
        await using fixture = await RunFixture.open(dialect, answer, { maximumAttempts: 2 });

        // leave a run as two attempts that never returned left it
        const stranded = await fixture.send({ call: pinning("stranded") });
        await fixture.database
            .update(object.run.table)
            .set({ state: "running", attempts: 2 })
            .where(eq(object.run.table.id, stranded.id));
        await reconcile(fixture.server, "run", { id: stranded.id });

        // cancel a run while its call runs
        const cancelled = await fixture.send({ call: pinning("cancelled") });
        const attempting = reconcile(fixture.server, "run", { id: cancelled.id });
        await vi.waitFor(() => expect(fixture.pushed.length).toBe(1));
        const [running] = (await fixture.runs()).filter((entry) => entry.id === cancelled.id);
        await fixture.server.executeAsSystem(
            object.run,
            "cancel",
            [SystemCall.of(running!)],
            Date.now(),
        );
        await attempting;

        expect([
            (await fixture.runs()).map((entry) => [entry.state, entry.error?.code ?? null]),
            aborted,
        ]).toEqual([
            [
                ["failed", "ATTEMPTS_EXHAUSTED"],
                ["skipped", "CANCELLED"],
            ],
            ["cancelled"],
        ]);
    },
);

test("push a call through a host's ingress as the installation, answering refusals as final and other failures as transient", async () => {
    // answer each push with one of the recorded responses
    const responses: Response[] = [];
    const requests: {
        readonly path: string;
        readonly caller: string;
        readonly release: string | null;
    }[] = [];
    const push = pushThrough({
        ingress: async (_installation, path, request, caller) => {
            requests.push({
                path,
                caller: caller.authentication.subject.id,
                release: request.headers.get(VERSION_HEADER),
            });

            return responses.shift()!;
        },
    });
    const target = {
        id: ids.notes,
        scope: ids.space,
        packageId: ids.package,
    } as object.Installation;
    const mutation = { id: v7(), calls: [pin] };
    const outcome = () =>
        push(target, mutation, new AbortController().signal).then(
            (answered) => answered,
            (error: ServiceError<string, unknown>) => ({
                thrown: [error.code, error.status, error.message],
            }),
        );

    // answer an executed call, a final refusal, a throttle, a failure without a body, and an unavailable host
    responses.push(
        Response.json({
            outcomes: [{ id: mutation.id, outcome: { value: [null] } }],
            watermark: { scope: ids.space, epoch: "e", sequence: 1 },
        }),
        Response.json({ code: "FORBIDDEN", message: "permission denied: update" }, { status: 403 }),
        Response.json({ code: "TOO_MANY_REQUESTS", message: "slow down" }, { status: 429 }),
        new Response("bad gateway", { status: 502 }),
        Response.json({ code: "UNAVAILABLE", message: "no running deployment" }, { status: 503 }),
    );
    const answered = [
        await outcome(),
        await outcome(),
        await outcome(),
        await outcome(),
        await outcome(),
    ];

    expect(answered).toEqual([
        { value: [null] },
        { error: { code: "FORBIDDEN", status: 403, message: "permission denied: update" } },
        { thrown: ["TOO_MANY_REQUESTS", 429, "slow down"] },
        { thrown: ["HTTP_502", 502, "bad gateway"] },
        { thrown: ["UNAVAILABLE", 503, "no running deployment"] },
    ]);
    expect(requests[0]).toEqual({ path: "/replica/push", caller: ids.notes, release: pin.release });
});

test.each(TEST_DIALECTS)(
    "run a sent call on the lent authority of the person whose call sent it, and record it failed when the lending is invalid on %s",
    async (dialect) => {
        await using fixture = await RunFixture.open(dialect);

        // send one call lent to the notes installation, one lent to another, and one with an unverifiable token
        const lent = await fixture.send({
            call: pinning("lent"),
            delegation: `lent:ada:${ids.notes}`,
        });
        const other = await fixture.send({
            call: pinning("other"),
            delegation: "lent:ada:installation-01996ab0-0000-7000-8000-000000000099",
        });
        const forged = await fixture.send({ call: pinning("forged"), delegation: "forged" });
        await fixture.attempt();

        const ada = principal.user.reference("universe", "ada");
        const authority = { subject: ada, subjects: [ada] };
        expect([
            [lent.onBehalfOf, other.state, other.error?.code, forged.state, forged.error?.code],
            fixture.pushed.map((entry) => [pinned(entry.mutation), entry.onBehalfOf]),
        ]).toEqual([
            [authority, "failed", "DELEGATION_REFUSED", "failed", "DELEGATION_REFUSED"],
            [["lent", authority]],
        ]);
    },
);
