import { expect, test } from "@destack/test";
import { TEST_DIALECTS } from "@destack/db/test";
import {
    defineTrigger,
    Trigger,
    type ChangeTrigger,
    type RunRequest,
} from "@destack/service/trigger";
import { present, schema } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { v7 } from "uuid";
import { ChangeController } from "../src/server/change.ts";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook } from "./fixture/note.ts";

/** Pin each note entering or changing among the titled ones. */
const titled = changeTrigger(
    defineTrigger({
        name: "titled",
        on: {
            change: {
                object: note,
                where: { title: { ne: "" } },
                operations: ["create", "update"],
                from: "snapshot",
            },
        },
        call: (change) =>
            note.calls().update({ id: present(change.after, "the changed note").id, pinned: true }),
    }),
);

/** Read the key a change controller follows a trigger under. */
function keyOf(trigger: {
    readonly package: { readonly id: string };
    readonly name: string;
}): string {
    return `${trigger.package.id}/${trigger.name}`;
}

/** Read a trigger firing on changes, refusing one of another kind. */
function changeTrigger(trigger: Trigger): ChangeTrigger {
    return present(Trigger.of(trigger, "change"), `change trigger ${trigger.name}`);
}

/** Read the message of a reported error, refusing a report of anything else. */
function messageOf(error: unknown): string {
    if (!(error instanceof Error)) {
        throw new TypeError("a trigger reported something other than an error");
    }

    return error.message;
}

/** A change trigger's run request. */
type Fired = Extract<RunRequest, { readonly triggerName: string }>;

test("build a call of an object's method, leaving the scope to the one it is pushed to", async () => {
    const { connect } = await serveNotes("sqlite");
    const alice = connect("alice");
    const { id } = await alice.notebook.create({
        spaceId,
        name: "Travel",
        requestId: RequestId.create(),
    });

    // run a call that leaves out its space in the space it is pushed to
    const call = notebook.calls().update({ id, name: "Trips" });
    const { outcomes } = await alice.replica.push({
        scope: spaceId,
        mutations: [{ id: v7(), calls: [call] }],
    });

    expect([call, outcomes.map((entry) => "value" in entry.outcome)]).toEqual([
        { method: "notebook.update", input: { id, name: "Trips" }, release: note.package.version },
        [true],
    ]);
    expect((await alice.notebook.get({ spaceId, id })).name).toBe("Trips");
});

test.each(TEST_DIALECTS)(
    "record a run for each matching row of a trigger's snapshot and each admitted change after it, once, resuming after its slot on %s",
    async (dialect) => {
        // keep two notes before the trigger starts, one of them untitled
        const { connect, database } = await serveNotes(dialect);
        const alice = connect("alice");
        const first = await alice.note.create({
            spaceId,
            title: "Packing",
            requestId: RequestId.create(),
        });
        await alice.note.create({ spaceId, title: "", requestId: RequestId.create() });

        // record the snapshot's titled note, failing one send to retry it
        const recorded: Fired[] = [];
        let isFailing = true;
        const runs = {
            send: async (request: Fired) => {
                const change = "change" in request.event ? request.event.change : undefined;
                if (isFailing && change?.key === undefined) {
                    isFailing = false;
                    throw new TypeError("the cell is unreachable");
                }
                recorded.push(request);
            },
        };
        const reports: unknown[] = [];
        const report = (error: unknown) => reports.push(error);
        const controller = new ChangeController(database, [titled], runs, report);
        await controller.reconcile(keyOf(titled));

        // record a change entering the titled notes, retrying after a failed send
        const second = await alice.note.create({
            spaceId,
            title: "Tickets",
            requestId: RequestId.create(),
        });
        await expect(controller.reconcile(keyOf(titled))).rejects.toThrow(
            "the cell is unreachable",
        );
        await controller.reconcile(keyOf(titled));

        // continue after the slot in a new controller, recording nothing twice
        await new ChangeController(database, [titled], runs, report).reconcile(keyOf(titled));

        const changes = recorded.map((request) =>
            "change" in request.event ? request.event.change : undefined,
        );
        expect(
            recorded.map(({ triggerName, call }, index) => [
                triggerName,
                call,
                changes[index]?.key ?? null,
            ]),
        ).toEqual([
            ["titled", note.calls().update({ id: first.id, pinned: true }), first.id],
            ["titled", note.calls().update({ id: second.id, pinned: true }), null],
        ]);
        const [earlier, later] = changes;
        expect(
            present(later, "the later change").position.sequence >
                present(earlier, "the earlier change").position.sequence,
        ).toBe(true);
        expect(reports).toEqual([]);
    },
);

test.each(TEST_DIALECTS)(
    "see rows entering a trigger's condition as created, changing within it as updated and leaving it as deleted, in log order on %s",
    async (dialect) => {
        // follow the titled notes from now on, keeping each change the trigger builds a call of
        const { connect, database } = await serveNotes(dialect);
        const alice = connect("alice");
        const seen: [string, string | null, string | null][] = [];
        const trigger = changeTrigger(
            defineTrigger({
                name: "retitled",
                on: {
                    change: {
                        object: note,
                        where: { title: { ne: "" } },
                        operations: ["create", "update", "delete"],
                    },
                },
                call: (change) => {
                    seen.push([
                        change.operation,
                        change.before === undefined ? null : change.before.title,
                        change.after === undefined ? null : change.after.title,
                    ]);

                    return note.calls().update({
                        id: present(change.after ?? change.before, "the changed note").id,
                    });
                },
            }),
        );
        const controller = new ChangeController(
            database,
            [trigger],
            { send: async () => {} },
            (error) => {
                throw error;
            },
        );
        await controller.reconcile(keyOf(trigger));

        // title a note, retitle it, clear its title, and write an untitled note
        const { id, revision } = await alice.note.create({
            spaceId,
            title: "Packing",
            requestId: RequestId.create(),
        });
        const update = (title: string, at: number) =>
            alice.note.update({ spaceId, id, title, revision: at, requestId: RequestId.create() });
        await update("Tickets", revision);
        await update("", revision + 1);
        await alice.note.create({ spaceId, title: "", requestId: RequestId.create() });
        await controller.reconcile(keyOf(trigger));

        expect(seen).toEqual([
            ["create", null, "Packing"],
            ["update", "Packing", "Tickets"],
            ["delete", "Tickets", null],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "advance a quiet trigger's slot, pass over a change it builds no call of, and continue at the head once its lag lost changes on %s",
    async (dialect) => {
        // follow every note from now on, a minute of lag, failing to build a call of an untitled note
        const { connect, database } = await serveNotes(dialect);
        const alice = connect("alice");
        const lagging = changeTrigger(
            defineTrigger({
                name: "lagging",
                on: { change: { object: note, operations: ["create"], maxLag: 60_000 } },
                call: (change) => {
                    // refuse an untitled note
                    const created = present(change.after, "the created note");
                    if (created.title === "") {
                        throw new TypeError("an untitled note has nothing to pin");
                    }

                    return note.calls().update({ id: created.id, pinned: true });
                },
            }),
        );
        const recorded: string[] = [];
        const reports: string[] = [];
        let now = Date.now();
        const controller = new ChangeController(
            database,
            [lagging],
            {
                send: async (request) => {
                    recorded.push(schema.string().parse(request.call.input["id"]));
                },
            },
            (error) => reports.push(messageOf(error)),
            () => now,
        );
        const create = async (title: string) =>
            (await alice.note.create({ spaceId, title, requestId: RequestId.create() })).id;

        // advance the slot of a quiet trigger before it lapses, so compaction keeps the changes after it
        const delay = await controller.reconcile(keyOf(lagging));
        now += 50_000;
        await controller.reconcile(keyOf(lagging));
        const kept = await create("Kept");
        await database.log.compact(Date.now() + 1, now + 20_000);
        await controller.reconcile(keyOf(lagging));

        // pass over a change the trigger builds no call of, reporting it
        const untitled = await create("");
        await controller.reconcile(keyOf(lagging));

        // continue at the head once a lapsed slot let compaction take changes, reporting the loss
        const lost = await create("Lost");
        await database.log.compact(Date.now() + 1, now + 10 * 60_000);
        await controller.reconcile(keyOf(lagging));
        const next = await create("Next");
        await controller.reconcile(keyOf(lagging));

        expect([delay, recorded]).toEqual([20_000, [kept, next]]);
        expect(reports.map((message) => message.replace(/\d+/gu, "N"))).toEqual([
            "trigger lagging builds no call of change N",
            "trigger lagging lost the changes after N to compaction",
        ]);
        expect([untitled, lost].some((id) => recorded.includes(id))).toBe(false);
    },
);
