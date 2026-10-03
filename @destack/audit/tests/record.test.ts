import { test, expect } from "@destack/test";
import { AuditError } from "../src/error/index.ts";
import { AuditStorage, renameDocument, rename } from "./storage.ts";
import { AuditCall, defineAuditAction } from "../src/index.ts";
import { document } from "./stack/index.ts";
import { schema } from "@destack/schema";
import { AuditRecorder } from "../src/record/recorder.ts";

test("keep each call in the history of its scope", async () => {
    const storage = await AuditStorage.open();
    try {
        // record a running call of a space
        const scope = schema
            .identifier("space")
            .parse("space-01996ab0-0000-7000-8000-000000000001");
        const recorder = new AuditRecorder(
            {
                caller: { type: "system" as const, name: "integration" },
                package: renameDocument.package,
                service: "document",
                scope,
            },
            storage.journal,
        );
        const recorded = recorder.begin(renameDocument, rename);
        await recorder.append(recorded);
        expect(await storage.journal.deliver(storage.history)).toBe(1);

        // read it in the space's history and nowhere else
        expect([
            (await storage.history.list({ scope, limit: 100 })).items.map((record) => record.call),
            await storage.history.list({ scope: "universe", limit: 100 }),
        ]).toEqual([[recorded], { items: [], cursor: null }]);
    } finally {
        await storage.close();
    }
});

test("commit and roll back application changes with their recorded calls", async () => {
    let storage = await AuditStorage.open();
    try {
        // roll back application state and recorded calls together
        const failure = new AuditError("CONFLICT", "cancel change");
        await expect(
            storage.database.transaction(async (transaction) => {
                await transaction.insert(document).values({ name: "rolled back" });
                await storage.recorder.record(transaction, renameDocument, {
                    ...rename,
                    outcome: { kind: "success" },
                });
                throw failure;
            }),
        ).rejects.toBe(failure);
        expect(await storage.database.select().from(document)).toEqual([]);
        expect(await storage.journal.read()).toEqual([]);

        // keep committed changes and calls across a restart
        const call = await storage.database.transaction(async (transaction) => {
            await transaction.insert(document).values({ name: "renamed" });

            return storage.recorder.record(transaction, renameDocument, {
                ...rename,
                outcome: { kind: "success" },
            });
        });
        storage = await storage.reopen();
        expect(await storage.database.select().from(document)).toEqual([{ name: "renamed" }]);
        expect(await storage.journal.read()).toEqual([call]);
    } finally {
        await storage.close();
    }
});

test("record a running call, then its outcome once, accepting repeats and refusing changes", async () => {
    let storage = await AuditStorage.open();
    try {
        // persist a prepared call only on append, then its outcome in the same record
        const running = storage.recorder.begin(renameDocument, rename);
        expect(await storage.journal.read()).toEqual([]);
        await storage.recorder.append(running);
        const finished = storage.recorder.finish(running, { kind: "success" });
        await storage.recorder.append(finished);
        storage = await storage.reopen();
        await storage.recorder.append(AuditCall.parse(JSON.parse(JSON.stringify(finished))));
        expect(await storage.journal.read()).toEqual([finished]);

        // refuse a finished call with other contents
        await expect(
            storage.recorder.append({
                ...finished,
                execution: { ...finished.execution, details: { name: "different" } },
            }),
        ).rejects.toMatchObject({
            code: "CONFLICT",
            message: `call ${finished.execution.id} has conflicting contents`,
        });
    } finally {
        await storage.close();
    }
});

test("record a read as one access call naming what only its value tells", async () => {
    const storage = await AuditStorage.open();
    try {
        // read a document with its name from the value
        const value = await storage.recorder.read(
            renameDocument,
            rename,
            async () => ({ name: "chosen" }),
            (renamed) => ({ name: renamed.name }),
        );
        const calls = await storage.journal.read();
        expect([
            value,
            calls.map((call) => [
                call.execution.category,
                call.execution.outcome,
                call.execution.details,
            ]),
        ]).toEqual([{ name: "chosen" }, [["access", { kind: "success" }, { name: "chosen" }]]]);
    } finally {
        await storage.close();
    }
});

test("leave sensitive values out of running and read details", async () => {
    const storage = await AuditStorage.open();
    try {
        // declare a sign-in with a sensitive code and an account result
        const signIn = defineAuditAction(
            {
                name: "account.signIn",
                targets: renameDocument.targets,
                details: schema.object({
                    method: schema.string(),
                    code: schema.sensitive(schema.string()).exactOptional(),
                    account: schema
                        .object({ id: schema.string(), secret: schema.sensitive(schema.string()) })
                        .exactOptional(),
                }),
            },
            { package: renameDocument.package },
        );

        // keep the method in the running call's details
        const targets = rename.targets;
        const running = storage.recorder.begin(signIn, {
            targets,
            details: { method: "code", code: "123456" },
        });
        expect(running.execution.details).toEqual({ method: "code" });

        // keep the account's identifier in the read's details
        const read = await storage.recorder.read(
            signIn,
            { targets, details: { method: "code" } },
            async () => ({ id: "account-2", secret: "hunter3" }),
            (account) => ({ method: "code", account }),
        );
        const calls = await storage.journal.read();
        expect([read, calls.map((call) => call.execution.details)]).toEqual([
            { id: "account-2", secret: "hunter3" },
            [{ method: "code", account: { id: "account-2" } }],
        ]);
    } finally {
        await storage.close();
    }
});
