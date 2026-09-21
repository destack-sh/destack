import { test, expect } from "@destack/test";
import { AuditError } from "../src/error/index.ts";
import { AuditStorage, renameDocument, rename } from "./storage.ts";
import { document } from "./stack/index.ts";

test("commit and roll back application changes with their audit events", async () => {
    let storage = await AuditStorage.open();
    try {
        // a rejected transaction leaves neither application state nor audit records
        const failure = new AuditError("CONFLICT", "cancel change");
        await expect(
            storage.database.transaction(async (transaction) => {
                await transaction.insert(document).values({ name: "rolled back" });
                await storage.recorder.record(transaction, renameDocument, {
                    ...rename,
                    outcome: "success",
                });
                throw failure;
            }),
        ).rejects.toBe(failure);
        expect(await storage.database.select().from(document)).toEqual([]);
        expect(await storage.outbox.read()).toEqual([]);

        // committed changes and events survive closing every runtime object
        const event = await storage.database.transaction(async (transaction) => {
            await transaction.insert(document).values({ name: "renamed" });

            return storage.recorder.record(transaction, renameDocument, {
                ...rename,
                outcome: "success",
            });
        });
        storage = await storage.reopen();
        expect(await storage.database.select().from(document)).toEqual([{ name: "renamed" }]);
        expect(await storage.outbox.read()).toEqual([event]);
        expect(event.attemptId).toBeUndefined();
    } finally {
        await storage.close();
    }
});

test("persist prepared attempts and outcomes without recreating events on retry", async () => {
    let storage = await AuditStorage.open();
    try {
        // construction performs no persistence and the prepared event survives serialization
        const attempt = storage.recorder.begin(renameDocument, rename);
        expect(await storage.outbox.read()).toEqual([]);
        await storage.recorder.append(attempt);
        const result = storage.recorder.complete(attempt, { outcome: "success" });
        await storage.recorder.append(result);
        storage = await storage.reopen();
        await storage.recorder.append(JSON.parse(JSON.stringify(result)));
        expect(await storage.outbox.read()).toEqual([attempt, result]);
        expect(result.attemptId).toBe(attempt.id);

        // reusing an identity with different contents fails before delivery
        await expect(
            storage.recorder.append({ ...result, details: { name: "different" } }),
        ).rejects.toMatchObject({
            code: "CONFLICT",
            message: "audit event identifier has conflicting contents",
        });
        await expect(storage.outbox.append(result, storage.database)).rejects.toMatchObject({
            code: "INVALID_EVENT",
            message: "audit recording requires a transaction on this database",
        });
    } finally {
        await storage.close();
    }
});
