import { ResourceContext } from "@destack/resource/context";
import { testKeys } from "@destack/object/test";
import { schema } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { Change } from "@destack/db";
import { expect, refusal, test } from "@destack/test";
import { defineWorkflow, StandardSignature } from "../src/index.ts";
import { alice, note, notesPackage, openServed, openWorkflow } from "./workflow.ts";
import { TEST_START } from "@destack/object/test";
import { NOTES } from "./package.ts";
import { NO_OUTPUT } from "../src/index.ts";

/** A minute, in milliseconds. */
const MINUTE = 60_000;

/** A day, in milliseconds. */
const DAY = 24 * 60 * MINUTE;

/** The secret the webhook deliveries are signed with, in the Standard Webhooks form. */
const SECRET = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";

/** The input of a tick's run: when it fired. */
const TickInput = schema.object({ at: schema.number() });

/** Tick every minute, skipping an occurrence while the previous tick still runs. */
const minutely = defineWorkflow(
    {
        name: "minutely",
        input: TickInput,
        triggers: {
            "every-minute": {
                kind: "schedule",
                schedule: {
                    timing: { timing: "interval", interval: MINUTE, startsAt: TEST_START + MINUTE },
                    concurrency: "forbid",
                    deadline: 10_000,
                },
                input: (occurrence) => ({ at: occurrence.at }),
            },
        },
        output: NO_OUTPUT,
    },
    NOTES,
);

/** Tick on the schedules a space's managers create. */
const scheduled = defineWorkflow({ name: "scheduled", input: TickInput, output: NO_OUTPUT }, NOTES);

/** Tick for each note titled urgent. */
const urgent = defineWorkflow(
    {
        name: "urgent",
        input: TickInput,
        triggers: {
            urgent: {
                kind: "change",
                change: { object: note, where: { title: "urgent" }, operations: ["create"] },
                input: (change) => ({ at: change.position.sequence }),
            },
        },
        output: NO_OUTPUT,
    },
    NOTES,
);

/** Tick for each note titled urgent, under the trigger name urgent's workflow uses. */
const alarm = defineWorkflow(
    {
        name: "alarm",
        input: TickInput,
        triggers: {
            urgent: {
                kind: "change",
                change: { object: note, where: { title: "urgent" }, operations: ["create"] },
                input: (change) => ({ at: change.position.sequence }),
            },
        },
        output: NO_OUTPUT,
    },
    NOTES,
);

/** Tick for each signed delivery. */
const hooked = defineWorkflow(
    {
        name: "hooked",
        input: TickInput,
        triggers: {
            hooks: {
                kind: "webhook",
                webhook: {
                    signature: new StandardSignature(),
                    path: "/",
                    secret: async () => SECRET,
                },
                input: (delivery) => ({ at: delivery.receivedAt }),
            },
        },
        output: NO_OUTPUT,
    },
    NOTES,
);

/** A schedule every minute from the next minute on, its runs allowed to overlap. */
const EVERY_MINUTE = {
    timing: { timing: "interval", interval: MINUTE, startsAt: TEST_START + MINUTE },
    concurrency: "allow",
    deadline: 10_000,
} as const;

test("fire a declared schedule's occurrences once each, skipping one that overlaps a running run, and refuse changing it as a space's manager", async () => {
    // sleep ninety seconds in each run, so the second occurrence overlaps the first run
    const fixture = await openWorkflow(minutely, async (step) => {
        await step.sleep("work", { seconds: 90 });
    });

    // run four minutes
    await fixture.simulation.clock.sleep(TEST_START + 4 * MINUTE - fixture.simulation.clock.now());
    await fixture.simulation.idle();
    const runs = await fixture.until(() => true);
    const [declared] = await fixture.served.database.select().from(minutely.trigger.table);

    // refuse alice changing the declared trigger, which its package's installation manages
    const changed = fixture.objects(alice).call(minutely.trigger.name, "update", {
        id: declared?.["id"],
        schedule: EVERY_MINUTE,
    });

    // start the first and third occurrences, skip the second and fourth as overlapping
    expect({
        inputs: runs.map((row) => row["input"]),
        skipped: declared?.["skipped"],
        lastSkip: declared?.["lastSkip"],
        manager: declared?.["managerName"],
        refused: await refusal(changed),
    }).toEqual({
        inputs: [{ at: TEST_START + MINUTE }, { at: TEST_START + 3 * MINUTE }],
        skipped: 2,
        lastSkip: { at: TEST_START + 4 * MINUTE, reason: "overlap" },
        manager: "minutely-triggers/every-minute",
        refused: [
            "CONFLICT",
            "MANAGED",
            `${minutely.trigger.name} is managed by its declaration: detach it before changing it`,
        ],
    });
});

test("fire a schedule a space's manager creates with its stored input, and none while paused", async () => {
    const fixture = await openWorkflow(scheduled, async () => {});

    // create a schedule every minute as alice, and pause it after its first occurrence
    const triggers = fixture.objects(alice);
    const created = await triggers.call(scheduled.trigger.name, "create", {
        name: "mine",
        schedule: EVERY_MINUTE,
        input: { at: 0 },
    });
    await fixture.simulation.jump(TEST_START + MINUTE);
    const first = await fixture.until((rows) => rows.length === 1);
    const { id } = schema.looseObject({ id: schema.string() }).parse(created);
    await triggers.call(scheduled.trigger.name, "pause", { id });
    await fixture.simulation.jump(TEST_START + 3 * MINUTE);
    const paused = await fixture.until(() => true);
    expect([first.map((row) => row["input"]), paused.length]).toEqual([[{ at: 0 }], 1]);
});

test("refuse a created schedule whose input the workflow refuses", async () => {
    const fixture = await openWorkflow(scheduled, async () => {});

    // create a schedule whose runs' input has no time
    const created = await refusal(
        fixture.objects(alice).call(scheduled.trigger.name, "create", {
            name: "broken",
            schedule: EVERY_MINUTE,
            input: {},
        }),
    );
    expect(created).toEqual([
        "BAD_REQUEST",
        "INPUT_INVALID",
        "invalid input: at: invalid input: expected number, received undefined",
    ]);
});

test("start one run per note a change trigger admits, from now on", async () => {
    // start a tick for each note titled urgent created after the trigger begins
    const fixture = await openWorkflow(urgent, async () => {});
    await fixture.until(() => true);

    // write an urgent note, a quiet one, and another urgent one
    const notes = fixture.served.as(alice).client(notesPackage).note;
    for (const title of ["urgent", "quiet", "urgent"]) {
        await notes.create({
            spaceId: fixture.served.scope,
            requestId: RequestId.create(fixture.environment),
            title,
        });
    }
    const runs = await fixture.until(
        (rows) => rows.length >= 2 && rows.every((row) => row["state"] === "completed"),
    );

    // start one run per urgent note, at the sequence its creation took in the log
    const page = await fixture.served.database.log.read({ tables: [note.table], after: 0 });
    const urgentAt = page.changes.flatMap((change) =>
        Change.image(change).title === "urgent" ? [{ at: change.sequence }] : [],
    );
    const inputs = runs.map((row) => TickInput.parse(row["input"]));
    expect(inputs.toSorted((left, right) => left.at - right.at)).toEqual(urgentAt);
});

test("start a run in each workflow whose trigger of one name admits a change", async () => {
    // serve urgent and alarm, both with a trigger named urgent
    const fixture = await openServed(
        urgent,
        urgent.implement(async () => {}),
        [{ workflow: alarm, served: alarm.implement(async () => {}) }],
    );
    await fixture.until(() => true);

    // write an urgent note, which both triggers admit
    await fixture.served
        .as(alice)
        .client(notesPackage)
        .note.create({
            spaceId: fixture.served.scope,
            requestId: RequestId.create(fixture.environment),
            title: "urgent",
        });
    const runs = await fixture.until((rows) => rows[0]?.["state"] === "completed");
    const alarms = await fixture.served.database.select().from(alarm.run.table);
    expect([runs.length, alarms.map((row) => row["state"])]).toEqual([1, ["completed"]]);
});

test("start no run while a change trigger is paused, and catch up on its changes once resumed", async () => {
    // pause the urgent trigger once it began
    const fixture = await openWorkflow(urgent, async () => {});
    await fixture.until(() => true);
    const [row] = await fixture.served.database.select().from(urgent.trigger.table);
    const { id } = schema.looseObject({ id: schema.string() }).parse(row);
    const triggers = fixture.objects(alice);
    await triggers.call(urgent.trigger.name, "pause", { id });

    // write two urgent notes while it is paused, then resume it
    const notes = fixture.served.as(alice).client(notesPackage).note;
    for (const title of ["urgent", "urgent"]) {
        await notes.create({
            spaceId: fixture.served.scope,
            requestId: RequestId.create(fixture.environment),
            title,
        });
    }
    const paused = await fixture.until(() => true);
    await triggers.call(urgent.trigger.name, "resume", { id });
    const resumed = await fixture.until(
        (rows) => rows.length >= 2 && rows.every((each) => each["state"] === "completed"),
    );

    // start the paused changes' runs once resumed, and none before
    expect([paused.length, resumed.length]).toEqual([0, 2]);
});

test("renew an idle change trigger's slot past a week, starting a run for a later note", async () => {
    // let an active trigger idle past a week a day at a time, and compact the log
    const fixture = await openWorkflow(urgent, async () => {});
    await fixture.until(() => true);
    for (let day = 1; day <= 8; day++) {
        await fixture.simulation.jump(TEST_START + day * DAY);
    }
    const now = fixture.simulation.clock.now();
    await fixture.served.database.log.compact(now, now);

    // write an urgent note, which the renewed slot still follows
    await fixture.served
        .as(alice)
        .client(notesPackage)
        .note.create({
            spaceId: fixture.served.scope,
            requestId: RequestId.create(fixture.environment),
            title: "urgent",
        });
    const runs = await fixture.until((rows) => rows[0]?.["state"] === "completed");
    const [row] = await fixture.served.database.select().from(urgent.trigger.table);
    expect([row?.["state"], runs.length]).toEqual(["active", 1]);
});

test("stop a change trigger paused past its slot's retention, and start over from now once reset", async () => {
    // pause the urgent trigger once it began, and write an urgent note while it is paused
    const fixture = await openWorkflow(urgent, async () => {});
    await fixture.until(() => true);
    const { database } = fixture.served;
    const [row] = await database.select().from(urgent.trigger.table);
    const { id } = schema.looseObject({ id: schema.string() }).parse(row);
    const triggers = fixture.objects(alice);
    await triggers.call(urgent.trigger.name, "pause", { id });
    const notes = fixture.served.as(alice).client(notesPackage).note;
    const write = () =>
        notes.create({
            spaceId: fixture.served.scope,
            requestId: RequestId.create(fixture.environment),
            title: "urgent",
        });
    await write();

    // let the paused trigger's slot expire past a week, compact the log, and resume it
    await fixture.simulation.jump(TEST_START + 8 * DAY);
    const now = fixture.simulation.clock.now();
    await database.log.compact(now, now);
    await triggers.call(urgent.trigger.name, "resume", { id });
    await fixture.served.simulation.idle();
    const [stopped] = await database.select().from(urgent.trigger.table);

    // reset the stopped trigger, and write another urgent note
    await triggers.call(urgent.trigger.name, "reset", { id });
    await fixture.until(() => true);
    await write();
    const runs = await fixture.until((rows) => rows[0]?.["state"] === "completed");

    // stop on the compacted note, then start a run for the note after the reset alone
    expect([stopped?.["state"], stopped?.["failure"], runs.length]).toEqual([
        "stopped",
        {
            code: "GONE",
            reason: "CHANGES_COMPACTED",
            domain: "@destack/workflow",
            message: "trigger urgent lost changes to compaction",
        },
        1,
    ]);
});

test("start one run per verified webhook delivery, refusing a forged one", async () => {
    // start a tick per signed delivery to the hooks trigger
    const fixture = await openWorkflow(
        hooked,
        async () => {},
        async (space) => ({
            server: {
                resources: new ResourceContext(),
                keys: await testKeys(space),
                environment: space,
            },
        }),
    );
    await fixture.until(() => true);

    // deliver a signed message twice and a forged one
    const body = '{"type":"ping"}';
    const message = { id: "delivery-1", event: "ping", body, sentAt: TEST_START };
    const headers = await new StandardSignature().sign(
        message,
        SECRET,
        fixture.served.environment.crypto,
    );
    const deliver = async (signed: Headers) =>
        (
            await fixture.implementation.webhooks(fixture.served.store).fetch(
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
