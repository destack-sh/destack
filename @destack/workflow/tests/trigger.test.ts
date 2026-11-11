import { ResourceHandle, Secret } from "@destack/resource";
import { ResourceContext } from "@destack/resource/context";
import { schema } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { Change } from "@destack/db";
import { expect, refusal, test } from "@destack/test";
import { defineWebhook, defineWorkflow, StandardSignature } from "../src/index.ts";
import {
    alice,
    note,
    notesPackage,
    openServed,
    openWorkflow,
    type TestWorkflow,
} from "./workflow.ts";
import { TEST_START } from "@destack/object/test";
import { NOTES } from "./package.ts";
import { NO_OUTPUT } from "../src/index.ts";

/** A minute, in milliseconds. */
const MINUTE = 60_000;

/** A day, in milliseconds. */
const DAY = 24 * 60 * MINUTE;

/** The secret a source signs its deliveries with, in the Standard Webhooks form. */
const SOURCE_SECRET = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";

/** The URL of the notes installation, which its webhooks are below. */
const NOTES_URL = "https://notes.test";

/** The secret a rotation of the bound secret makes current, in Standard Webhooks form. */
const ROTATED_SECRET = "whsec_c2Vjb25kIHNlY3JldA==";

/** A secret neither the bound secret nor the source issued. */
const OTHER_SECRET = "whsec_b3RoZXIgc2VjcmV0IGtleQ==";

/** The body of a ping delivery. */
const PING = '{"type":"ping"}';

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

/** The secret a stack binds for a sender it configures, such as a GitHub App. */
const appSecret = new ResourceHandle<Secret>(NOTES.package, "app-webhook");

/** The deliveries of a sender the installation's bound secret signs. */
const app = defineWebhook("app", { signature: new StandardSignature(), secret: appSecret }, NOTES);

/** Tick for each delivery of a webhook. */
function hookedOn(webhook: typeof app) {
    return defineWorkflow(
        {
            name: "hooked",
            input: TickInput,
            triggers: {
                delivered: {
                    kind: "webhook",
                    webhook,
                    input: (delivery) => ({ at: delivery.receivedAt }),
                },
            },
            output: NO_OUTPUT,
        },
        NOTES,
    );
}

/** Sign a ping as the Standard Webhooks sender does with a secret, now. */
async function sign(fixture: TestWorkflow, secret: string, id: string): Promise<Headers> {
    const message = { id, event: "ping", body: PING, sentAt: fixture.simulation.clock.now() };

    return new StandardSignature().sign(message, secret, fixture.served.environment.crypto);
}

/** Deliver a ping with headers to a webhook of the fixture's installation, answering its status. */
async function deliver(fixture: TestWorkflow, webhook: string, headers: Headers): Promise<number> {
    // send the delivery and read its whole answer, closing the request
    const url = `${NOTES_URL}/webhooks/${webhook}`;
    const request = new Request(url, { method: "POST", headers, body: PING });
    const response = await fixture.fetch(request);
    await response.arrayBuffer();

    return response.status;
}

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
    await fixture.simulation.run(TEST_START + 4 * MINUTE);
    const runs = await fixture.waitUntil(() => true);
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
            "PRECONDITION_FAILED",
            "DECLARATION_BLOCKED",
            `cannot change '${minutely.trigger.name}': it is managed by its declaration, so detach it first`,
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
    const first = await fixture.waitUntil((rows) => rows.length === 1);
    const { id } = schema.looseObject({ id: schema.string() }).parse(created);
    await triggers.call(scheduled.trigger.name, "pause", { id });
    await fixture.simulation.jump(TEST_START + 3 * MINUTE);
    const paused = await fixture.waitUntil(() => true);
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
    await fixture.waitUntil(() => true);

    // write an urgent note, a quiet one, and another urgent one
    const notes = fixture.served.as(alice).client(notesPackage).note;
    for (const title of ["urgent", "quiet", "urgent"]) {
        await notes.create({
            spaceId: fixture.served.scope,
            requestId: RequestId.create(fixture.environment),
            title,
        });
    }
    const runs = await fixture.waitUntil(
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
    await fixture.waitUntil(() => true);

    // write an urgent note, which both triggers admit
    await fixture.served
        .as(alice)
        .client(notesPackage)
        .note.create({
            spaceId: fixture.served.scope,
            requestId: RequestId.create(fixture.environment),
            title: "urgent",
        });
    const runs = await fixture.waitUntil((rows) => rows[0]?.["state"] === "completed");
    const alarms = await fixture.served.database.select().from(alarm.run.table);
    expect([runs.length, alarms.map((row) => row["state"])]).toEqual([1, ["completed"]]);
});

test("start no run while a change trigger is paused, and catch up on its changes once resumed", async () => {
    // pause the urgent trigger once it began
    const fixture = await openWorkflow(urgent, async () => {});
    await fixture.waitUntil(() => true);
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
    const paused = await fixture.waitUntil(() => true);
    await triggers.call(urgent.trigger.name, "resume", { id });
    const resumed = await fixture.waitUntil(
        (rows) => rows.length >= 2 && rows.every((each) => each["state"] === "completed"),
    );

    // start the paused changes' runs once resumed, and none before
    expect([paused.length, resumed.length]).toEqual([0, 2]);
});

test("keep an idle change trigger's checkpoint past a week, starting a run for a later note", async () => {
    // let an active trigger idle past a week a day at a time, and compact the log
    const fixture = await openWorkflow(urgent, async () => {});
    await fixture.waitUntil(() => true);
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
    const runs = await fixture.waitUntil((rows) => rows[0]?.["state"] === "completed");
    const [row] = await fixture.served.database.select().from(urgent.trigger.table);
    expect([row?.["state"], runs.length]).toEqual(["active", 1]);
});

test("stop a change trigger paused past its slot's retention, and start over from now once reset", async () => {
    // pause the urgent trigger once it began, and write an urgent note while it is paused
    const fixture = await openWorkflow(urgent, async () => {});
    await fixture.waitUntil(() => true);
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
    await fixture.waitUntil(() => true);
    await write();
    const runs = await fixture.waitUntil((rows) => rows[0]?.["state"] === "completed");

    // stop on the compacted note, then start a run for the note after the reset alone
    expect([stopped?.["state"], stopped?.["failure"], runs.length]).toEqual([
        "stopped",
        {
            code: "NOT_FOUND",
            reason: "CHANGES_NOT_FOUND",
            domain: "@destack/workflow",
            message: "changes of trigger 'urgent' removed by compaction not found",
        },
        1,
    ]);
});

test("start one run per delivery verified with the bound secret, refusing a forged one", async () => {
    // serve a tick per delivery signed with the secret the installation binds
    const resources = new ResourceContext().bind(appSecret, Secret.of(SOURCE_SECRET));
    const fixture = await openWorkflow(
        hookedOn(app),
        async () => {},
        async () => ({
            server: { resources },
        }),
    );

    // deliver a message signed with the bound secret twice, and an unsigned one
    const headers = await sign(fixture, SOURCE_SECRET, "delivery-1");
    const statuses = [
        await deliver(fixture, "app", headers),
        await deliver(fixture, "app", headers),
        await deliver(fixture, "app", new Headers()),
    ];
    const runs = await fixture.waitUntil(
        (rows) => rows.length === 1 && rows[0]?.["state"] === "completed",
    );

    // start one run for the repeated delivery
    expect([statuses, runs.length]).toEqual([[202, 202, 401], 1]);
});

test("verify a delivery signed with a rotated secret's previous version while it still retires", async () => {
    // bind a secret rotated once, its previous version retiring beside the current one
    const previous = { version: 1, value: { encoding: "text" as const, value: SOURCE_SECRET } };
    const current = { version: 2, value: { encoding: "text" as const, value: ROTATED_SECRET } };
    const rotated: Secret = {
        read: async () => current,
        versions: async () => [current, previous],
    };
    const resources = new ResourceContext().bind(appSecret, rotated);
    const fixture = await openWorkflow(
        hookedOn(app),
        async () => {},
        async () => ({
            server: { resources },
        }),
    );

    // deliver messages signed with the previous and the current version, and one with another secret
    const statuses = [
        await deliver(fixture, "app", await sign(fixture, SOURCE_SECRET, "delivery-1")),
        await deliver(fixture, "app", await sign(fixture, ROTATED_SECRET, "delivery-2")),
        await deliver(fixture, "app", await sign(fixture, OTHER_SECRET, "delivery-3")),
    ];

    expect(statuses).toEqual([202, 202, 401]);
});
