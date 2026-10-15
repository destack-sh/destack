import { PackageMount } from "@destack/runtime";
import { type Insert, eq } from "@destack/db";
import { expect, refusal, test } from "@destack/test";
import { CallError } from "@destack/runtime/error";
import { RequestId, VERSION_HEADER } from "@destack/runtime/request";
import { RunEvent } from "@destack/runtime/trigger";
import type { ExchangeAsk } from "@destack/runtime/authentication";
import { v7 } from "uuid";
import { Identifier, aligned, found, present, schema } from "@destack/schema";
import { QUERIES_PUBLICATION } from "@destack/object";
import * as object from "../../object/index.ts";
import { installation } from "../../object/index.ts";
import {
    endingOf,
    pin,
    ids,
    reconcile,
    type Answer,
    RunFixture,
    T0,
    StackFixture,
    WAIT_MILLISECONDS,
} from "../../test/index.ts";
import { TEST_DIALECTS } from "@destack/db/test";
import { principal, Caller } from "@destack/access";
import { pushThrough } from "../index.ts";
import type * as sync from "@destack/sync";

/** The daily schedule's interval, its first occurrence a day after the test starts. */
const DAY = 86_400_000;

/** Answer a refusal as a server writes it on the wire. */
function wire(error: CallError<string, unknown>): Response {
    return Response.json(error.toJSON(), { status: error.status });
}

/** Keep the notes installation and its triggers in a space its owner owns, and serve the space. */
async function open() {
    // keep the notes installation
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const now = Date.now();
    await database.insert(installation.table).values({
        id: ids.notes,
        scope: ids.space,
        packageId: ids.package,
        role: "application",
        name: "notes",
        selection: { kind: "release", version: "2026.9.0" },
        createdAt: now,
        updatedAt: now,
    });

    // declare its webhook, paused, change and schedule triggers
    await database.insert(object.trigger.table).values(declaredTriggers(now));

    return { database, client: await fixture.serve() };
}

/** Declare the notes triggers: two webhooks, one paused, a change and a schedule due tomorrow. */
function declaredTriggers(now: number): Insert<typeof object.trigger.table>[] {
    const declared = { scope: ids.space, installationId: ids.notes, origin: "declared" as const };

    return [
        {
            ...declared,
            id: Identifier.create("trigger"),
            name: "pushes",
            on: { webhook: { verification: "github", path: "/" } },
            createdAt: now,
            updatedAt: now,
        },
        {
            ...declared,
            id: Identifier.create("trigger"),
            name: "muted",
            on: { webhook: { verification: "github", path: "/muted" } },
            isPaused: true,
            createdAt: now,
            updatedAt: now,
        },
        {
            ...declared,
            id: Identifier.create("trigger"),
            name: "pinned",
            on: {
                change: {
                    object: { packageId: ids.package, name: "note" },
                    operations: ["create"],
                    from: "now",
                    maxLag: 60_000,
                },
            },
            createdAt: now,
            updatedAt: now,
        },
        {
            ...declared,
            id: Identifier.create("trigger"),
            name: "daily",
            on: {
                schedule: {
                    timing: { timing: "interval", interval: DAY, startsAt: now + DAY },
                    concurrency: "allow",
                    deadline: 60_000,
                    call: pin,
                },
            },
            createdAt: now,
            updatedAt: now,
        },
    ];
}

test("list and follow an installation's runs as its readers, refusing users changing them", async () => {
    // keep one running run
    const { database, client } = await open();
    const now = Date.now();
    await database.insert(object.run.table).values({
        id: ids.run,
        scope: ids.space,
        createdAt: now,
        updatedAt: now,
        installationId: ids.notes,
        call: pin,
        at: now,
        requestId: RequestId.create(Date.now()),
        state: "running",
        concurrency: "allow",
        attempts: 1,
        startedAt: now,
        traceId: "4bf92f3577b34da6a3ce929d0e0e4736",
    });

    // list the run as the owner reading the installation, and hide the space from a stranger
    const listed = async (user: string) =>
        (await client(user).run.list({ spaceId: ids.space })).items.map((item) => [
            item.id,
            item.state,
            item.traceId,
        ]);
    expect(await listed(ids.owner)).toEqual([
        [ids.run, "running", "4bf92f3577b34da6a3ce929d0e0e4736"],
    ]);
    expect(await refusal(listed("stranger"))).toEqual(["NOT_FOUND", `no scope ${ids.space}`]);

    // follow the run in the owner's session
    const controller = new AbortController();
    const pages = await client(ids.owner).replica.stream(
        {
            name: "objects",
            publication: QUERIES_PUBLICATION,
            scope: ids.space,
            below: ids.space,
            parameters: { queries: { runs: { object: "run" } } },
        },
        { signal: controller.signal },
    );
    const page = await pages[Symbol.asyncIterator]().next();
    controller.abort();
    if (page.done === true) {
        throw new Error("the sync ended before its first page");
    }
    expect(page.value.changes.map((change) => [change.table, change.row["id"]])).toEqual([
        ["destack__stack__run", ids.run],
    ]);

    // refuse the owner sending a call or writing runs as the system does
    expect(
        await refusal(
            client(ids.owner).run.send({
                spaceId: ids.space,
                requestId: RequestId.create(Date.now()),
                installationId: ids.notes,
                call: pin,
                at: now,
            }),
        ),
    ).toEqual(["FORBIDDEN", "permission denied: send"]);
    const { outcomes } = await client(ids.owner).replica.push({
        scope: ids.space,
        mutations: [
            {
                id: v7(),
                calls: [
                    {
                        type: object.run.typeReference,
                        method: "update",
                        release: object.run.package.version,
                        input: { spaceId: ids.space, id: ids.run, state: "failed" },
                    },
                ],
            },
        ],
    });
    expect(outcomes.map((entry) => entry.outcome)).toEqual([
        { error: { code: "BAD_REQUEST", message: "no mutating method run.update", status: 400 } },
    ]);
});

test("record sent calls and fired triggers once per request and event", async () => {
    const { client } = await open();
    const notes = client(ids.notes);
    const now = Date.now();
    const send = (fields: Readonly<Record<string, unknown>>) =>
        notes.run.send({
            spaceId: ids.space,
            requestId: RequestId.create(Date.now()),
            installationId: ids.notes,
            call: pin,
            ...fields,
        });

    // send a call later and record a delivery and a change twice each
    const later = await send({ at: now + 60_000 });
    const delivery = {
        triggerName: "pushes",
        event: { delivery: { id: "delivery-1", path: "/" } },
    };
    const delivered = [await send(delivery), await send(delivery)] as const;
    const change = {
        triggerName: "pinned",
        event: { change: { position: { epoch: "epoch-1", sequence: 7 } } },
    };
    const changed = [await send(change), await send(change)] as const;

    expect([
        [later.triggerId, later.at - now, later.state, later.concurrency],
        [delivered[1], delivered[0].concurrency],
        [changed[1], changed[0].concurrency],
    ]).toEqual([
        [null, 60_000, "pending", "allow"],
        [delivered[0], "allow"],
        [changed[0], "queue"],
    ]);

    // refuse a schedule's occurrence, an unknown trigger, a paused one and events of another kind
    const refused = (fields: Readonly<Record<string, unknown>>) => refusal(send(fields));
    expect([
        await refused({ triggerName: "daily", event: { at: now } }),
        await refused({ triggerName: "unknown", event: { at: now } }),
        await refused({ ...delivery, triggerName: "muted" }),
        await refused({ ...change, triggerName: "pushes" }),
        await refused({ ...delivery, triggerName: "pinned" }),
    ]).toEqual([
        [
            "BAD_REQUEST",
            "trigger daily fires on a schedule, whose runs are the machine's to record",
        ],
        ["NOT_FOUND", "no trigger unknown"],
        ["CONFLICT", "trigger muted is paused"],
        ["BAD_REQUEST", "trigger pushes fires on webhook deliveries"],
        ["BAD_REQUEST", "trigger pinned fires on changes"],
    ]);
});

test("cancel a waiting run as the space's owner, refusing a finished one", async () => {
    const { client } = await open();
    const sent = await client(ids.notes).run.send({
        spaceId: ids.space,
        requestId: RequestId.create(Date.now()),
        installationId: ids.notes,
        call: pin,
        at: Date.now() + 60_000,
    });

    // cancel the waiting run and refuse cancelling it again
    const cancel = (revision: number) =>
        client(ids.owner).run.cancel({
            spaceId: ids.space,
            requestId: RequestId.create(Date.now()),
            id: sent.id,
            revision,
        });
    expect(await cancel(sent.revision)).toEqual({});
    expect(await refusal(cancel(sent.revision))).toEqual(["NOT_FOUND", `no run ${sent.id}`]);
});

/** Pin one note by its identifier. */
function pinning(id: string) {
    return { ...pin, input: { id, pinned: true } };
}

/** Read the note a pushed mutation pins. */
function pinned(mutation: sync.Mutation): string {
    return schema.string().parse(aligned(mutation.calls, 0).input["id"]);
}

test.each(TEST_DIALECTS)(
    "push each run's call under its own request, retrying transient failures on %s",
    async (dialect) => {
        // succeed, fail once transiently, refuse finally and stay unreachable
        const tries = new Map<string, number>();
        const answer: Answer = async (mutation) => {
            const note = pinned(mutation);
            tries.set(note, (tries.get(note) ?? 0) + 1);
            if (note === "flaky" && tries.get(note) === 1) {
                throw new CallError("SERVICE_UNAVAILABLE", {
                    message: "the instance is starting",
                });
            } else if (note === "refused") {
                return {
                    error: { code: "FORBIDDEN", status: 403, message: "permission denied: update" },
                };
            } else if (note === "unreachable") {
                throw new CallError("SERVICE_UNAVAILABLE", {
                    message: "no instance serves the release",
                });
            }

            return { value: [null] };
        };
        await using fixture = await RunFixture.open(dialect, answer, { maximumAttempts: 2 });
        for (const note of ["first", "flaky", "refused", "unreachable"]) {
            await fixture.send({ call: pinning(note) });
            fixture.clock.advance(1);
        }

        // attempt them until every run finished
        fixture.serve();
        await fixture.settle();
        const runs = await fixture.finished();
        expect(
            runs.map((entry) => [
                pinned({ id: "", calls: [entry.data.call] }),
                entry.keys.outcome,
                entry.data.attempts,
                endingOf(entry.data.outcome),
            ]),
        ).toEqual([
            ["first", "succeeded", 1, null],
            ["flaky", "succeeded", 2, null],
            ["refused", "failed", 1, "FORBIDDEN"],
            ["unreachable", "failed", 2, "SERVICE_UNAVAILABLE"],
        ]);

        // push every attempt of a run to its installation under the run's own request
        const requests = new Map(
            runs.map((entry) => [
                pinned({ id: "", calls: [entry.data.call] }),
                RunFixture.requestId(entry),
            ]),
        );
        expect(
            fixture.pushed
                .map(({ installation: target, mutation }) => [
                    target,
                    pinned(mutation),
                    mutation.id === requests.get(pinned(mutation)),
                ])
                .toSorted((left, right) => left.join().localeCompare(right.join())),
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
    "attempt a change trigger's runs one at a time in log order on %s",
    async (dialect) => {
        // keep each pushed call waiting until released, noting each push as it arrives
        const releases = new Map<string, () => void>();
        const arrivals = new Map<string, PromiseWithResolvers<void>>();
        const arrival = (note: string) => {
            const known = arrivals.get(note) ?? Promise.withResolvers<void>();
            arrivals.set(note, known);

            return known;
        };
        const answer: Answer = (mutation) =>
            new Promise((resolve) => {
                releases.set(pinned(mutation), () => resolve({ value: [null] }));
                arrival(pinned(mutation)).resolve();
            });
        await using fixture = await RunFixture.open(dialect, answer);
        await fixture.trigger("pinned", {
            change: {
                object: { packageId: ids.package, name: "note" },
                operations: ["create"],
                from: "now",
                maxLag: 60_000,
            },
        });
        for (const [sequence, note] of ["first", "second", "third"].entries()) {
            await fixture.send({
                call: pinning(note),
                triggerName: "pinned",
                event: { change: { position: { epoch: "epoch-1", sequence } } },
            });
        }

        // push each queued call only once the one before it finished
        fixture.serve();
        const order: string[] = [];
        for (const note of ["first", "second", "third"]) {
            await arrival(note).promise;
            expect([...releases.keys()]).toEqual([...order, note]);
            order.push(note);
            found(releases, note)();
        }
        await fixture.settle();
        expect((await fixture.finished()).map((entry) => entry.keys.outcome)).toEqual([
            "succeeded",
            "succeeded",
            "succeeded",
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "wait for a run's time, its installation's enabling and a running instance on %s",
    async (dialect) => {
        await using fixture = await RunFixture.open(dialect);
        const later = await fixture.send({ at: T0 + 60_000 });

        // wait until the run's time and leave it while the installation is suspended
        const attempt = () => reconcile(fixture, "run", { id: later.id });
        const early = await attempt();
        fixture.clock.set(T0 + 60_000);
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
        expect((await fixture.finished()).map((entry) => entry.keys.outcome)).toEqual([
            "succeeded",
        ]);
    },
);

test.each(TEST_DIALECTS)("attempt runs recorded while the loop runs on %s", async (dialect) => {
    // refuse the call pinning the second changed note for good
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
    fixture.clock.set(Date.now());
    fixture.serve();

    // send a call, fire a schedule and queue three changes while the loop runs
    await fixture.send({ call: pinning("sent") });
    await fixture.trigger("pinned", {
        change: {
            object: { packageId: ids.package, name: "note" },
            operations: ["create"],
            from: "now",
            maxLag: 60_000,
        },
    });
    await fixture.trigger("soon", {
        schedule: {
            timing: { timing: "once", startsAt: fixture.clock.now() },
            concurrency: "allow",
            deadline: 60_000,
            call: pin,
        },
    });
    for (const [sequence, note] of ["first", "refused", "third"].entries()) {
        await fixture.send({
            call: pinning(note),
            triggerName: "pinned",
            event: { change: { position: { epoch: "epoch-1", sequence } } },
        });
    }
    await fixture.database.log.until(
        async () => (await fixture.finished()).length === 5,
        AbortSignal.timeout(WAIT_MILLISECONDS),
    );
    await fixture.settle();

    const described = (await fixture.finished()).map(({ keys, data }) => [
        data.event === null ? "send" : Object.keys(data.event)[0] === "at" ? "schedule" : "change",
        pinned({ id: "", calls: [data.call] }),
        keys.outcome,
        endingOf(data.outcome),
    ]);
    expect(described.toSorted((left, right) => left.join().localeCompare(right.join()))).toEqual([
        ["change", "first", "succeeded", null],
        ["change", "refused", "failed", "FORBIDDEN"],
        ["change", "third", "succeeded", null],
        ["schedule", "note-1", "succeeded", null],
        ["send", "sent", "succeeded", null],
    ]);
});

test.each(TEST_DIALECTS)(
    "fail a run out of attempts and abort a cancelled run's attempt on %s",
    async (dialect) => {
        // keep each push waiting until aborted, noting the first push as it arrives
        const aborted: string[] = [];
        const pushing = Promise.withResolvers<void>();
        const answer: Answer = (mutation, signal) =>
            new Promise((_resolve, reject) => {
                pushing.resolve();
                signal.addEventListener("abort", () => {
                    aborted.push(pinned(mutation));
                    reject(signal.reason);
                });
            });
        await using fixture = await RunFixture.open(dialect, answer, { maximumAttempts: 2 });

        // leave a run as two attempts that never returned left it
        const stranded = await fixture.send({ call: pinning("stranded") });
        fixture.clock.advance(1);
        await fixture.database
            .update(object.run.table)
            .set({ state: "running", attempts: 2 })
            .where(eq(object.run.table.id, stranded.id));
        await reconcile(fixture, "run", { id: stranded.id });

        // cancel a run while its call runs
        const cancelled = await fixture.send({ call: pinning("cancelled") });
        const attempting = reconcile(fixture, "run", { id: cancelled.id });
        await pushing.promise;
        const running = (await fixture.runs()).find((entry) => entry.id === cancelled.id);
        const now = fixture.store.clock.now();
        await fixture.store.mutate(fixture.store.self, {
            id: RequestId.create(now),
            calls: [
                object.run.call("cancel", object.run.target(present(running, "the cancelled run"))),
            ],
        });
        await attempting;

        expect([
            (await fixture.finished()).map(({ keys, data }) => [
                keys.outcome,
                endingOf(data.outcome),
            ]),
            aborted,
        ]).toEqual([
            [
                ["failed", "ATTEMPTS_EXHAUSTED"],
                ["skipped", "cancelled"],
            ],
            ["cancelled"],
        ]);
    },
);

test("push a call through a machine's ingress as the installation or for its sender under their grant, refusals final and other failures transient", async () => {
    // answer each push with one of the recorded responses
    const responses: Response[] = [];
    const requests: {
        readonly path: string;
        readonly caller: string;
        readonly release: string | null;
    }[] = [];
    const push = pushThrough(
        {
            ingress: async (_installation, path, request, authentication) => {
                requests.push({
                    path,
                    caller: authentication.caller.subject.id,
                    release: request.headers.get(VERSION_HEADER),
                });

                return present(responses.shift(), "a recorded response");
            },
        },
        async (acting, exchange) => {
            const now = Date.now();

            return {
                caller: {
                    ...Caller.of(exchange.subject),
                    actors: [principal.installation.reference(acting.scope, acting.id)],
                    grant: {
                        id: exchange.grant ?? Identifier.create("grant"),
                        scope: exchange.subject.id,
                        permissions: [],
                    },
                },
                credential: {
                    kind: "fixture",
                    id: "fixture-1",
                    audience: ids.package,
                    verifiedAt: now,
                    expiresAt: now + 60_000,
                },
            };
        },
    );
    const target = { id: ids.notes, scope: ids.space, packageId: ids.package };
    const mutation = { id: v7(), calls: [pin] };
    const ada = principal.user.reference("universe", "ada");
    const outcome = (exchange: ExchangeAsk | null = null) =>
        push(target, mutation, Date.now(), new AbortController().signal, exchange).then(
            (answered) => answered,
            (error: CallError<string, unknown>) => ({
                thrown: [error.code, error.status, error.message],
            }),
        );

    // answer an executed call, a final refusal, a throttle, a bodiless failure and an unavailable host
    responses.push(
        Response.json({
            outcomes: [{ id: mutation.id, outcome: { value: [null] } }],
            watermark: { scope: ids.space, epoch: "e", sequence: 1 },
        }),
        wire(new CallError("FORBIDDEN", { message: "permission denied: update" })),
        wire(new CallError("TOO_MANY_REQUESTS", { message: "slow down" })),
        new Response("bad gateway", { status: 502 }),
        wire(new CallError("SERVICE_UNAVAILABLE", { message: "no running deployment" })),
        Response.json({
            outcomes: [{ id: mutation.id, outcome: { value: [null] } }],
            watermark: { scope: ids.space, epoch: "e", sequence: 2 },
        }),
    );
    const answered = [
        await outcome(),
        await outcome(),
        await outcome(),
        await outcome(),
        await outcome(),
        await outcome({ subject: ada, grant: Identifier.create("grant") }),
    ];

    expect(answered).toEqual([
        { value: [null] },
        { error: { code: "FORBIDDEN", status: 403, message: "permission denied: update" } },
        { thrown: ["TOO_MANY_REQUESTS", 429, "slow down"] },
        { thrown: ["HTTP_502", 502, "bad gateway"] },
        { thrown: ["SERVICE_UNAVAILABLE", 503, "no running deployment"] },
        { value: [null] },
    ]);
    // push every attempt the same way: below the package's mount, as the installation or for ada, at the pinned release
    const pushed = {
        path: `${PackageMount.path(ids.package)}/replica/push`,
        caller: ids.notes,
        release: pin.release,
    };
    expect(requests).toEqual([
        pushed,
        pushed,
        pushed,
        pushed,
        pushed,
        { ...pushed, caller: ada.id },
    ]);
});

test.each(TEST_DIALECTS)(
    "run a sent call for its sender under their grant on %s",
    async (dialect) => {
        await using fixture = await RunFixture.open(dialect);

        // send a call for ada under her grant to the notes installation
        const ada = principal.user.reference("universe", "ada");
        const exchange = { subject: ada, grant: Identifier.create("grant") };
        const sent = await fixture.send({ call: pinning("granted"), exchange });
        await fixture.attempt();

        expect([
            sent.exchange,
            fixture.pushed.map((entry) => [pinned(entry.mutation), entry.exchange]),
        ]).toEqual([exchange, [["granted", exchange]]]);
    },
);

test.each(TEST_DIALECTS)("finish runs as run events, leaving no row on %s", async (dialect) => {
    // fail the call pinning one note for good after its one attempt
    const unavailable = { code: "SERVICE_UNAVAILABLE", status: 503, message: "no instance" };
    const answer: Answer = async (mutation) => {
        if (pinned(mutation) === "fails") {
            throw new CallError("SERVICE_UNAVAILABLE", { message: unavailable.message });
        }

        return { value: [null] };
    };
    await using fixture = await RunFixture.open(dialect, answer, { maximumAttempts: 1 });

    // attempt a call that succeeds and one that fails
    const succeeded = await fixture.send({ call: pinning("succeeds"), at: T0 - 2_000 });
    const failed = await fixture.send({ call: pinning("fails"), at: T0 - 1_000 });
    await fixture.attempt();

    // fire a schedule superseding its earlier occurrence and cancel the latest one
    const minutely = await fixture.trigger("minutely", {
        schedule: {
            timing: { timing: "interval", interval: 60_000, startsAt: T0 - 60_000 },
            concurrency: "allow",
            deadline: 90_000,
            call: pin,
        },
    });
    fixture.clock.set(T0 + 10_000);
    await fixture.fire(minutely);
    const occurred = aligned(await fixture.runs(), 0);
    await fixture.store.mutate(fixture.store.self, {
        id: RequestId.create(fixture.clock.now()),
        calls: [object.run.call("cancel", object.run.target(occurred))],
    });

    // keep no row and one event per run with its complete outcome
    const finished = await fixture.finished();
    const record = { scope: ids.space };
    const sent = { installationId: ids.notes, triggerId: null, method: "note.update" };
    const scheduled = { installationId: ids.notes, triggerId: minutely.id, method: "note.update" };
    expect([await fixture.runs(), finished]).toEqual([
        [],
        [
            {
                ...record,
                id: aligned(finished, 0).id,
                time: (T0 + 10_000) * 1000,
                keys: { ...scheduled, outcome: "skipped" },
                data: {
                    call: pin,
                    triggerName: "minutely",
                    at: T0 - 60_000,
                    startedAt: null,
                    finishedAt: T0 + 10_000,
                    attempts: 0,
                    outcome: { kind: "skipped", skip: { reason: "superseded" } },
                    traceId: null,
                    requestId: RunEvent.requestId({ at: T0 - 60_000 }),
                    event: { at: T0 - 60_000 },
                },
            },
            {
                ...record,
                id: succeeded.id,
                time: T0 * 1000,
                keys: { ...sent, outcome: "succeeded" },
                data: {
                    call: pinning("succeeds"),
                    triggerName: null,
                    at: T0 - 2_000,
                    startedAt: T0,
                    finishedAt: T0,
                    attempts: 1,
                    outcome: { kind: "succeeded" },
                    traceId: null,
                    requestId: succeeded.requestId,
                    event: null,
                },
            },
            {
                ...record,
                id: failed.id,
                time: T0 * 1000,
                keys: { ...sent, outcome: "failed" },
                data: {
                    call: pinning("fails"),
                    triggerName: null,
                    at: T0 - 1_000,
                    startedAt: T0,
                    finishedAt: T0,
                    attempts: 1,
                    outcome: { kind: "failed", failure: unavailable },
                    traceId: null,
                    requestId: failed.requestId,
                    event: null,
                },
            },
            {
                ...record,
                id: occurred.id,
                time: (T0 + 10_000) * 1000,
                keys: { ...scheduled, outcome: "skipped" },
                data: {
                    call: pin,
                    triggerName: "minutely",
                    at: T0,
                    startedAt: null,
                    finishedAt: T0 + 10_000,
                    attempts: 0,
                    outcome: { kind: "skipped", skip: { reason: "cancelled" } },
                    traceId: null,
                    requestId: RunEvent.requestId({ at: T0 }),
                    event: { at: T0 },
                },
            },
        ],
    ]);
});
