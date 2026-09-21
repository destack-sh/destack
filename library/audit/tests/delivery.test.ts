import { test, expect } from "@destack/test";
import { AuditError } from "../src/error/index.ts";
import { AuditOutbox, type AuditEntry } from "../src/outbox/index.ts";
import { AuditStorage, renameDocument, rename } from "./storage.ts";

test("resume lost acknowledgements after restart and history expiration", async () => {
    let storage = await AuditStorage.open();
    try {
        // accept a delivery while its acknowledgement is lost
        const event = storage.recorder.begin(renameDocument, rename);
        await storage.recorder.append(event);
        const entry = (await storage.outbox.next())!;
        const failure = new AuditError("UNAVAILABLE", "acknowledgement lost");
        await expect(
            storage.outbox.flush({
                ingest: async (request) => {
                    await storage.history.ingest(request);
                    throw failure;
                },
            }),
        ).rejects.toBe(failure);
        expect(await storage.outbox.read()).toEqual([event]);

        // removing retained contents does not remove delivery progress
        const scope = { type: "global" as const };
        expect(await storage.history.prune(scope, Date.now() + 1, 100)).toBe(1);
        storage = await storage.reopen();
        expect(await storage.outbox.next()).toEqual(entry);
        expect(await storage.outbox.flush(storage.history)).toBe(1);
        expect(await storage.outbox.read()).toEqual([]);
        expect(await storage.history.list({ scope, limit: 100 })).toEqual({
            items: [],
            cursor: null,
        });

        // a late first delivery receives its own acceptance time and remains queryable
        const late = storage.recorder.begin(renameDocument, rename);
        late.occurredAt = 0;
        await storage.recorder.append(late);
        expect(await storage.outbox.flush(storage.history)).toBe(1);
        expect(
            (await storage.history.list({ scope, limit: 100 })).items.map((record) => record.event),
        ).toEqual([late]);
        await expect(storage.history.ingest(entry)).rejects.toMatchObject({
            code: "CONFLICT",
            message: "audit delivery position is stale or skips events",
        });
    } finally {
        await storage.close();
    }
});

test("serialize competing senders and reject changed, skipped, or retired deliveries", async () => {
    const storage = await AuditStorage.open();
    const connection = await storage.connection();
    try {
        // independent senders resume the same persisted position
        const event = storage.recorder.begin(renameDocument, rename);
        await storage.recorder.append(event);
        const other = new AuditOutbox(connection);
        const first = await storage.outbox.next();
        const second = await other.next();
        expect(first).toEqual(second);
        const entry = first!;

        // race actual delivery loops and allow storage contention to retry through their normal path
        const controller = new AbortController();
        const failures: unknown[] = [];
        const options = {
            signal: AbortSignal.any([controller.signal, AbortSignal.timeout(1000)]),
            interval: 1,
            report: (error: unknown) => failures.push(error),
        };
        const destination = {
            ingest: async (request: AuditEntry) => {
                const accepted = await storage.history.ingest(request);
                controller.abort();

                return accepted;
            },
        };
        await Promise.all([
            storage.outbox.run(destination, options),
            other.run(destination, options),
        ]);
        expect(controller.signal.aborted).toBe(true);
        expect(await storage.outbox.read()).toEqual([]);
        expect(
            (await storage.history.list({ scope: { type: "global" }, limit: 100 })).items.map(
                (record) => record.event,
            ),
        ).toEqual([event]);

        // acceptance never advances on conflicting contents or a missing position
        await expect(
            storage.history.ingest({ ...entry, event: { ...event, details: {} } }),
        ).rejects.toMatchObject({
            code: "CONFLICT",
            message: "audit delivery position has conflicting contents",
        });
        await expect(storage.history.ingest({ ...entry, sequence: 3 })).rejects.toMatchObject({
            code: "CONFLICT",
            message: "audit delivery position is stale or skips events",
        });
        await storage.history.retire(entry.producerId);
        await expect(storage.history.ingest(entry)).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "audit producer is retired or unavailable",
        });
    } finally {
        await connection.close();
        await storage.close();
    }
});

test("retain rejected deliveries and reject competing results", async () => {
    const storage = await AuditStorage.open();
    try {
        // persist the attempt and accept its first result
        const attempt = storage.recorder.begin(renameDocument, rename);
        await storage.recorder.append(attempt);
        const result = storage.recorder.complete(attempt, { outcome: "success" });
        await storage.recorder.append(result);
        expect(await storage.outbox.flush(storage.history)).toBe(2);
        const scope = { type: "global" as const };
        expect(await storage.history.list({ scope, unresolved: true, limit: 100 })).toEqual({
            items: [],
            cursor: null,
        });

        // an incompatible second outcome cannot advance producer progress or disappear
        const conflicting = storage.recorder.complete(attempt, {
            outcome: "failure",
            errorCode: "FAILED",
        });
        await storage.recorder.append(conflicting);
        await expect(storage.outbox.flush(storage.history)).rejects.toMatchObject({
            code: "CONFLICT",
            message: "audit attempt already has a result",
        });
        expect(await storage.outbox.read()).toEqual([conflicting]);
        const entry = (await storage.outbox.next()) as AuditEntry;
        await expect(
            storage.outbox.acknowledge(entry, {
                producerId: entry.producerId,
                sequence: entry.sequence + 1,
            }),
        ).rejects.toMatchObject({
            code: "CONFLICT",
            message: "audit acknowledgement does not match the delivery",
        });
    } finally {
        await storage.close();
    }
});
