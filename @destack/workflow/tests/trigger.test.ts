import { ResourceContext } from "@destack/resource/context";
import { testKeys } from "@destack/runtime/test";
import { schema } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { defineWorkflow, StandardSignature } from "../src/index.ts";
import { note, openWorkflow, START } from "./fixture.ts";

/** A minute, in milliseconds. */
const MINUTE = 60_000;

/** The secret the webhook deliveries are signed with, in the Standard Webhooks form. */
const SECRET = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";

/** The input of a tick's run: when it fired. */
const TickInput = schema.object({ at: schema.number() });

/** Tick every minute, skipping an occurrence while the previous tick still runs. */
const minutely = defineWorkflow({
    name: "minutely",
    input: TickInput,
    triggers: {
        "every-minute": {
            kind: "schedule",
            schedule: {
                timing: { timing: "interval", interval: MINUTE, startsAt: START + MINUTE },
                concurrency: "forbid",
                deadline: 10_000,
            },
            input: (occurrence) => ({ at: occurrence.at }),
        },
    },
});

/** Tick on the schedules a space's managers create. */
const scheduled = defineWorkflow({ name: "scheduled", input: TickInput });

/** Tick for each note titled urgent. */
const urgent = defineWorkflow({
    name: "urgent",
    input: TickInput,
    triggers: {
        urgent: {
            kind: "change",
            change: { object: note, where: { title: "urgent" }, operations: ["create"] },
            input: (change) => ({ at: change.position.sequence }),
        },
    },
});

/** Tick for each signed delivery. */
const hooked = defineWorkflow({
    name: "hooked",
    input: TickInput,
    triggers: {
        hooks: {
            kind: "webhook",
            webhook: { signature: new StandardSignature(), path: "/", secret: async () => SECRET },
            input: (delivery) => ({ at: delivery.receivedAt }),
        },
    },
});

/** A schedule every minute from the next minute on, its runs allowed to overlap. */
const EVERY_MINUTE = {
    timing: { timing: "interval", interval: MINUTE, startsAt: START + MINUTE },
    concurrency: "allow",
    deadline: 10_000,
} as const;

test("fire a declared schedule's occurrences once each, skipping one that overlaps a running run", async () => {
    // sleep ninety seconds in each run, so the second occurrence overlaps the first run
    const fixture = await openWorkflow(minutely, async (step) => {
        await step.sleep("work", { seconds: 90 });
    });

    // run four minutes in ten-second steps
    for (let at = 0; at < 4 * 6; at++) {
        await fixture.until(() => true);
        fixture.clock.advance(10_000);
    }
    const runs = await fixture.until(() => true);
    const [declared] = await fixture.store.database.select().from(minutely.trigger.table);

    // start the first and third occurrences, skip the second and fourth as overlapping
    expect({
        inputs: runs.map((row) => row["input"]),
        skipped: declared?.["skipped"],
        lastSkip: declared?.["lastSkip"],
    }).toEqual({
        inputs: [{ at: START + MINUTE }, { at: START + 3 * MINUTE }],
        skipped: 2,
        lastSkip: { at: START + 4 * MINUTE, reason: "overlap" },
    });
});

test("fire a schedule a space's manager creates with its stored input, and none while paused", async () => {
    const fixture = await openWorkflow(scheduled, async () => {});

    // create a schedule every minute as alice, and pause it after its first occurrence
    const [created] = await fixture.asAlice([
        scheduled.trigger.call("create", {
            spaceId: fixture.spaceId,
            name: "mine",
            schedule: EVERY_MINUTE,
            input: { at: 0 },
        }),
    ]);
    fixture.clock.advance(MINUTE);
    const first = await fixture.until((rows) => rows.length === 1);
    const { id } = schema.looseObject({ id: schema.string() }).parse(created);
    await fixture.asAlice([
        scheduled.trigger.call("pause", scheduled.trigger.target({ scope: fixture.spaceId, id })),
    ]);
    fixture.clock.advance(2 * MINUTE);
    const paused = await fixture.until(() => true);
    expect([first.map((row) => row["input"]), paused.length]).toEqual([[{ at: 0 }], 1]);
});

test("refuse a created schedule whose input the workflow refuses", async () => {
    const fixture = await openWorkflow(scheduled, async () => {});

    // create a schedule whose runs' input has no time
    const created = await refusal(
        fixture.asAlice([
            scheduled.trigger.call("create", {
                spaceId: fixture.spaceId,
                name: "broken",
                schedule: EVERY_MINUTE,
                input: {},
            }),
        ]),
    );
    expect(created).toEqual([
        "BAD_REQUEST",
        "invalid input: at: invalid input: expected number, received undefined",
    ]);
});

test("start one run per note a change trigger admits, from now on", async () => {
    // start a tick for each note titled urgent created after the trigger begins
    const fixture = await openWorkflow(urgent, async () => {});
    await fixture.until(() => true);

    // write an urgent note, a quiet one, and another urgent one
    for (const title of ["urgent", "quiet", "urgent"]) {
        await fixture.asAlice([note.call("create", { spaceId: fixture.spaceId, title })]);
    }
    const runs = await fixture.until(
        (rows) => rows.length >= 2 && rows.every((row) => row["state"] === "completed"),
    );
    expect(runs.length).toBe(2);
});

test("start one run per verified webhook delivery, refusing a forged one", async () => {
    // start a tick per signed delivery to the hooks trigger
    const fixture = await openWorkflow(hooked, async () => {}, {
        server: { resources: new ResourceContext(), keys: await testKeys() },
    });
    await fixture.until(() => true);

    // deliver a signed message twice and a forged one
    const body = '{"type":"ping"}';
    const message = { id: "delivery-1", event: "ping", body, sentAt: START };
    const headers = await new StandardSignature().sign(message, SECRET);
    const deliver = async (signed: Headers) =>
        (
            await fixture.served.webhooks(fixture.store).fetch(
                new Request("https://hooks.test/hooks", {
                    method: "POST",
                    headers: signed,
                    body,
                }),
            )
        ).status;
    const statuses = [await deliver(headers), await deliver(headers), await deliver(new Headers())];
    const runs = await fixture.until(
        (rows) => rows.length === 1 && rows[0]?.["state"] === "completed",
    );
    expect([statuses, runs.length]).toEqual([[202, 202, 401], 1]);
});
