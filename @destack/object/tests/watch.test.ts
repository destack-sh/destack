import { expect, test } from "@destack/test";
import { TEST_DIALECTS } from "@destack/db/test";
import { Condition } from "@destack/db/query";
import type { RunRequest } from "@destack/service/trigger";
import { defineWatch } from "@destack/service/watch";
import { RequestId } from "@destack/service/request";
import { v7 } from "uuid";
import { WatchController } from "../src/server/watch.ts";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook } from "./fixture/notes.ts";

/** Pin each note entering or changing among the titled ones. */
const titled = defineWatch({
    name: "titled",
    object: note,
    where: Condition.ne("title", ""),
    on: ["create", "update"],
    from: "snapshot",
    call: (change) => note.calls().update({ id: String(change.after!.id), pinned: true }),
});

/** Read the key a watch controller follows a watch under. */
function keyOf(watch: {
    readonly package: { readonly id: string };
    readonly name: string;
}): string {
    return `${watch.package.id}/${watch.name}`;
}

/** A watched change's run request. */
type Watched = Extract<RunRequest, { readonly cause: "watch" }>;

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
    "record a run for each matching row of a watch's snapshot and each admitted change after it, once, resuming after its slot on %s",
    async (dialect) => {
        // keep two notes before the watch starts, one of them untitled
        const { connect, database } = await serveNotes(dialect);
        const alice = connect("alice");
        const first = await alice.note.create({
            spaceId,
            title: "Packing",
            requestId: RequestId.create(),
        });
        await alice.note.create({ spaceId, title: "", requestId: RequestId.create() });

        // record the snapshot's titled note, failing one send to retry it
        const recorded: Watched[] = [];
        let isFailing = true;
        const runs = {
            send: async (request: Watched) => {
                if (isFailing && request.key === undefined) {
                    isFailing = false;
                    throw new TypeError("the cell is unreachable");
                }
                recorded.push(request);
            },
        };
        const reports: unknown[] = [];
        const report = (error: unknown) => reports.push(error);
        const controller = new WatchController(database, [titled], runs, report);
        await controller.reconcile(keyOf(titled));
        const [snapshot] = recorded;

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
        await new WatchController(database, [titled], runs, report).reconcile(keyOf(titled));

        const cause = { cause: "watch", packageId: note.package.id, trigger: "titled" };
        expect(
            recorded.map(({ cause: kind, packageId, trigger, call, key }) => [
                { cause: kind, packageId, trigger },
                call,
                key ?? null,
            ]),
        ).toEqual([
            [cause, note.calls().update({ id: first.id, pinned: true }), first.id],
            [cause, note.calls().update({ id: second.id, pinned: true }), null],
        ]);
        expect(recorded[1]!.sequence! > snapshot!.sequence!).toBe(true);
        expect(reports).toEqual([]);
    },
);

test.each(TEST_DIALECTS)(
    "see rows entering a watch's condition as created, changing within it as updated and leaving it as deleted, in log order on %s",
    async (dialect) => {
        // watch the titled notes from now on, keeping each change the watch builds a call of
        const { connect, database } = await serveNotes(dialect);
        const alice = connect("alice");
        const seen: [string, string | null, string | null][] = [];
        const watch = defineWatch({
            name: "retitled",
            object: note,
            where: Condition.ne("title", ""),
            on: ["create", "update", "delete"],
            call: (change) => {
                seen.push([
                    change.operation,
                    change.before === undefined ? null : String(change.before.title),
                    change.after === undefined ? null : String(change.after.title),
                ]);

                return note.calls().update({ id: String((change.after ?? change.before)!.id) });
            },
        });
        const controller = new WatchController(
            database,
            [watch],
            { send: async () => {} },
            (error) => {
                throw error;
            },
        );
        await controller.reconcile(keyOf(watch));

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
        await controller.reconcile(keyOf(watch));

        expect(seen).toEqual([
            ["create", null, "Packing"],
            ["update", "Packing", "Tickets"],
            ["delete", "Tickets", null],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "advance a quiet watch's slot, pass over a change it builds no call of, and continue at the head once its lag lost changes on %s",
    async (dialect) => {
        // watch every note from now on, a minute of lag, failing to build a call of an untitled note
        const { connect, database } = await serveNotes(dialect);
        const alice = connect("alice");
        const lagging = defineWatch({
            name: "lagging",
            object: note,
            on: ["create"],
            maxLag: 60_000,
            call: (change) => {
                if (change.after!.title === "") {
                    throw new TypeError("an untitled note has nothing to pin");
                }

                return note.calls().update({ id: String(change.after!.id), pinned: true });
            },
        });
        const recorded: string[] = [];
        const reports: string[] = [];
        let now = Date.now();
        const controller = new WatchController(
            database,
            [lagging],
            {
                send: async (request) => {
                    recorded.push(request.call.input.id as string);
                },
            },
            (error) => reports.push((error as Error).message),
            () => now,
        );
        const create = async (title: string) =>
            (await alice.note.create({ spaceId, title, requestId: RequestId.create() })).id;

        // advance the slot of a quiet watch before it lapses, so compaction keeps the changes after it
        const delay = await controller.reconcile(keyOf(lagging));
        now += 50_000;
        await controller.reconcile(keyOf(lagging));
        const kept = await create("Kept");
        await database.log.compact(Date.now() + 1, now + 20_000);
        await controller.reconcile(keyOf(lagging));

        // pass over a change the watch builds no call of, reporting it
        const untitled = await create("");
        await controller.reconcile(keyOf(lagging));

        // continue at the head once a lapsed slot let compaction take changes, reporting the loss
        const lost = await create("Lost");
        await database.log.compact(Date.now() + 1, now + 10 * 60_000);
        await controller.reconcile(keyOf(lagging));
        const next = await create("Next");
        await controller.reconcile(keyOf(lagging));

        expect([delay, recorded]).toEqual([20_000, [kept, next]]);
        expect(reports.map((message) => message.replace(/\d+/g, "N"))).toEqual([
            "watch lagging builds no call of change N",
            "watch lagging lost the changes after N to compaction",
        ]);
        expect([untitled, lost].some((id) => recorded.includes(id))).toBe(false);
    },
);

test("refuse two watches of one package under one name", async () => {
    const { database } = await serveNotes("sqlite");

    expect(
        () => new WatchController(database, [titled, titled], { send: async () => {} }, () => {}),
    ).toThrow(`${titled.package.name} declares two watches named titled`);
});
