import { test, expect } from "@destack/test";
import { AuditError } from "../src/error/index.ts";
import { AuditStorage, renameDocument, rename } from "./storage.ts";
import { defineAuditAction } from "../src/index.ts";
import { document } from "./stack/index.ts";
import { identifier, schema } from "@destack/schema";
import { AuditRecorder } from "../src/record/recorder.ts";

test("keep each event in the history of its scope", async () => {
    const storage = await AuditStorage.open();
    try {
        // record an event of a space
        const scope = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");
        const recorder = new AuditRecorder(
            {
                actor: { type: "system" as const, name: "integration" },
                delegation: [],
                package: renameDocument.package,
                service: "document",
                scope,
            },
            storage.outbox,
        );
        const recorded = recorder.begin(renameDocument, rename);
        await recorder.append(recorded);
        expect(await storage.outbox.flush(storage.history)).toBe(1);

        // read it in the space's history and nowhere else
        expect([
            (await storage.history.list({ scope, limit: 100 })).items.map((record) => record.event),
            await storage.history.list({ scope: "global", limit: 100 }),
        ]).toEqual([[recorded], { items: [], cursor: null }]);
    } finally {
        await storage.close();
    }
});

test("commit and roll back application changes with their audit events", async () => {
    let storage = await AuditStorage.open();
    try {
        // roll back application state and audit records together
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

        // keep committed changes and events across a restart
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
        // persist prepared events only on append, including after serialization
        const attempt = storage.recorder.begin(renameDocument, rename);
        expect(await storage.outbox.read()).toEqual([]);
        await storage.recorder.append(attempt);
        const result = storage.recorder.complete(attempt, { outcome: "success" });
        await storage.recorder.append(result);
        storage = await storage.reopen();
        await storage.recorder.append(JSON.parse(JSON.stringify(result)));
        expect(await storage.outbox.read()).toEqual([attempt, result]);
        expect(result.attemptId).toBe(attempt.id);

        // reject a reused identity with different contents before delivery
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

test("name in a result what only the action's value tells, beside the attempt's details", async () => {
    const storage = await AuditStorage.open();
    try {
        // attempt a rename, naming the new name its value tells
        const value = await storage.recorder.attempt(
            renameDocument,
            rename,
            async () => ({ name: "chosen" }),
            (renamed) => ({ name: renamed.name }),
        );
        const events = await storage.outbox.read();
        expect([value, events.map((event) => [event.result.stage, event.details])]).toEqual([
            { name: "chosen" },
            [
                ["attempt", { name: "renamed" }],
                ["result", { name: "chosen" }],
            ],
        ]);
    } finally {
        await storage.close();
    }
});

test("leave sensitive values out of attempt and result details", async () => {
    const storage = await AuditStorage.open();
    try {
        // declare a sign-in whose code is sensitive, and whose result names the account without its secret
        const signIn = defineAuditAction(
            {
                name: "Account.signIn",
                version: 1,
                targets: renameDocument.targets,
                details: schema.object({
                    method: schema.string(),
                    code: schema.sensitive(schema.string()).optional(),
                    account: schema
                        .object({ id: schema.string(), secret: schema.sensitive(schema.string()) })
                        .optional(),
                }),
            },
            { package: renameDocument.package },
        );

        // keep the method, and not the code, in the attempt's details
        const targets = rename.targets;
        const attempt = storage.recorder.begin(signIn, {
            targets,
            details: { method: "code", code: "123456" },
        });
        expect(attempt.details).toEqual({ method: "code" });

        // keep the account's identifier, and not its secret, in the details the result derives
        const attempted = await storage.recorder.attempt(
            signIn,
            { targets, details: { method: "code" } },
            async () => ({ id: "account-2", secret: "hunter3" }),
            (account) => ({ method: "code", account }),
        );
        const events = await storage.outbox.read();
        expect([attempted, events.map((event) => event.details)]).toEqual([
            { id: "account-2", secret: "hunter3" },
            [{ method: "code" }, { method: "code", account: { id: "account-2" } }],
        ]);
    } finally {
        await storage.close();
    }
});
