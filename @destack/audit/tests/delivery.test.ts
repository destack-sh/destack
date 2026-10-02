import { test, expect } from "@destack/test";
import { testCallKey } from "@destack/service/test";
import { ControlLoop } from "@destack/service/control";
import { AuditError } from "../src/error/index.ts";
import { Journal, journal } from "../src/index.ts";
import { AuditStorage, renameDocument, rename } from "./storage.ts";

test("deliver a batch again after its acceptance was lost, the history holding each call once", async () => {
    let storage = await AuditStorage.open();
    try {
        // lose a delivery's acknowledgement
        const call = storage.recorder.begin(renameDocument, rename);
        await storage.recorder.append(call);
        const failure = new AuditError("UNAVAILABLE", "acceptance lost");
        await expect(
            storage.journal.deliver({
                ingest: async (batch) => {
                    await storage.history.ingest(batch);
                    throw failure;
                },
            }),
        ).rejects.toBe(failure);

        // deliver it again after a restart, and nothing more after
        storage = await storage.reopen();
        expect(await storage.journal.deliver(storage.history)).toBe(1);
        expect(await storage.journal.deliver(storage.history)).toBe(0);
        const scope = "universe";
        expect(
            (await storage.history.list({ scope, limit: 100 })).items.map((record) => record.call),
        ).toEqual([call]);
    } finally {
        await storage.close();
    }
});

test("deliver calls and their outcomes as they commit through control loops of competing senders", async () => {
    const storage = await AuditStorage.open();
    const connection = await storage.connection();
    const stopping = new AbortController();
    const failures: unknown[] = [];
    const report = (_controller: unknown, _key: string, error: unknown) => failures.push(error);
    const loops = [storage.journal, new Journal(connection, testCallKey)].map((source) =>
        new ControlLoop(source.database, [source.controller(storage.history)], { report }).run(
            stopping.signal,
        ),
    );
    try {
        // record a running call and its outcome while both senders run
        const running = storage.recorder.begin(renameDocument, rename);
        await storage.recorder.append(running);
        const finished = storage.recorder.finish(running, { kind: "success" });
        await storage.recorder.append(finished);

        // wait until the history holds the outcome
        const waiting = AbortSignal.any([stopping.signal, AbortSignal.timeout(1000)]);
        const history = async () =>
            (await storage.history.list({ scope: "universe", limit: 100 })).items.map(
                (record) => record.call,
            );
        await storage.database.log.until(
            async () => (await history())[0]?.execution.outcome !== undefined,
            waiting,
        );
        expect([await history(), failures]).toEqual([[finished], []]);

        // refuse a finished call with other contents
        await expect(
            storage.history.ingest({
                calls: [
                    {
                        ...finished,
                        execution: { ...finished.execution, details: { name: "other" } },
                    },
                ],
            }),
        ).rejects.toMatchObject({
            code: "CONFLICT",
            message: "audited call has conflicting contents",
        });
    } finally {
        stopping.abort();
        await Promise.all(loops);
        await connection.close();
        await storage.close();
    }
});

test("refuse a second outcome of a call in the journal and in the history", async () => {
    const storage = await AuditStorage.open();
    try {
        // deliver a call and its outcome, leaving nothing running
        const running = storage.recorder.begin(renameDocument, rename);
        await storage.recorder.append(running);
        const finished = storage.recorder.finish(running, { kind: "success" });
        await storage.recorder.append(finished);
        expect(await storage.journal.deliver(storage.history)).toBe(1);
        const scope = "universe";
        expect(await storage.history.list({ scope, isRunning: true, limit: 100 })).toEqual({
            items: [],
            cursor: null,
        });

        // refuse a second outcome in both places
        const conflicting = storage.recorder.finish(running, {
            kind: "failure",
            error: { code: "FAILED", status: 500, message: "failed" },
        });
        await expect(storage.recorder.append(conflicting)).rejects.toMatchObject({
            code: "CONFLICT",
            message: `call ${running.execution.id} has conflicting contents`,
        });
        await expect(storage.history.ingest({ calls: [conflicting] })).rejects.toMatchObject({
            code: "CONFLICT",
            message: "audited call has conflicting contents",
        });
    } finally {
        await storage.close();
    }
});

test("remove delivered calls once past their lifetime, keeping a call the history has not received", async () => {
    const storage = await AuditStorage.open({ milliseconds: 50 });
    const stopping = new AbortController();
    const failures: unknown[] = [];
    const report = (_controller: unknown, _key: string, error: unknown) => failures.push(error);
    try {
        // deliver one call, and record another the history never receives
        const delivered = storage.recorder.begin(renameDocument, rename);
        await storage.recorder.append(delivered);
        expect(await storage.journal.deliver(storage.history)).toBe(1);
        const pending = storage.recorder.begin(renameDocument, rename);
        await storage.recorder.append(pending);

        // let the journal's control loop remove the delivered call once its lifetime ends
        const loop = new ControlLoop(storage.database, [storage.journal.controller()], {
            report,
        }).run(stopping.signal);
        const remaining = async () =>
            (await storage.database.select({ id: journal.id }).from(journal)).map((row) => row.id);
        await storage.database.log.until(
            async () => (await remaining()).length === 1,
            AbortSignal.timeout(2000),
        );
        stopping.abort();
        await loop;
        expect([await remaining(), failures]).toEqual([[pending.execution.id], []]);
    } finally {
        stopping.abort();
        await storage.close();
    }
});
