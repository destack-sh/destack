import { test, expect } from "@destack/test";
import { ControlLoop } from "@destack/service/control";
import { AuditError } from "../src/error/index.ts";
import { AuditOutbox } from "../src/outbox/index.ts";
import { AuditStorage, renameDocument, rename } from "./storage.ts";

test("deliver a batch again after its acceptance was lost, the history holding each event once", async () => {
    let storage = await AuditStorage.open();
    try {
        // lose a delivery's acknowledgement
        const event = storage.recorder.begin(renameDocument, rename);
        await storage.recorder.append(event);
        const failure = new AuditError("UNAVAILABLE", "acceptance lost");
        await expect(
            storage.outbox.deliver({
                ingest: async (batch) => {
                    await storage.history.ingest(batch);
                    throw failure;
                },
            }),
        ).rejects.toBe(failure);
        expect(await storage.outbox.read()).toEqual([event]);

        // deliver it again after a restart
        storage = await storage.reopen();
        expect(await storage.outbox.deliver(storage.history)).toBe(1);
        expect(await storage.outbox.read()).toEqual([]);
        const scope = "global";
        expect(
            (await storage.history.list({ scope, limit: 100 })).items.map((record) => record.event),
        ).toEqual([event]);
    } finally {
        await storage.close();
    }
});

test("deliver events as they commit through control loops of competing senders, each once and in order", async () => {
    const storage = await AuditStorage.open();
    const connection = await storage.connection();
    const stopping = new AbortController();
    const failures: unknown[] = [];
    const report = (_controller: unknown, _key: string, error: unknown) => failures.push(error);
    const loops = [storage.outbox, new AuditOutbox(connection)].map((outbox) =>
        new ControlLoop(outbox.database, [outbox.controller(storage.history)], { report }).run(
            stopping.signal,
        ),
    );
    try {
        // record an attempt and its result while both senders run
        const attempt = storage.recorder.begin(renameDocument, rename);
        await storage.recorder.append(attempt);
        const result = storage.recorder.complete(attempt, { outcome: "success" });
        await storage.recorder.append(result);

        // wait until the outbox is empty
        const waiting = AbortSignal.any([stopping.signal, AbortSignal.timeout(1000)]);
        await storage.database.log.until(
            async () => (await storage.outbox.read()).length === 0,
            waiting,
        );
        expect([
            (await storage.history.list({ scope: "global", limit: 100 })).items.map(
                (record) => record.event,
            ),
            failures,
        ]).toEqual([[attempt, result], []]);

        // reject a held identifier with other contents
        await expect(
            storage.history.ingest({ events: [{ ...attempt, details: {} }] }),
        ).rejects.toMatchObject({
            code: "CONFLICT",
            message: "audit event identifier has conflicting contents",
        });
    } finally {
        stopping.abort();
        await Promise.all(loops);
        await connection.close();
        await storage.close();
    }
});

test("keep a batch the history rejects pending, and reject a second result of an attempt", async () => {
    const storage = await AuditStorage.open();
    try {
        // deliver the attempt and its first result
        const attempt = storage.recorder.begin(renameDocument, rename);
        await storage.recorder.append(attempt);
        const result = storage.recorder.complete(attempt, { outcome: "success" });
        await storage.recorder.append(result);
        expect(await storage.outbox.deliver(storage.history)).toBe(2);
        const scope = "global";
        expect(await storage.history.list({ scope, unresolved: true, limit: 100 })).toEqual({
            items: [],
            cursor: null,
        });

        // keep an incompatible second outcome pending
        const conflicting = storage.recorder.complete(attempt, {
            outcome: "failure",
            errorCode: "FAILED",
        });
        await storage.recorder.append(conflicting);
        await expect(storage.outbox.deliver(storage.history)).rejects.toMatchObject({
            code: "CONFLICT",
            message: "audit attempt already has a result",
        });
        expect(await storage.outbox.read()).toEqual([conflicting]);
    } finally {
        await storage.close();
    }
});
