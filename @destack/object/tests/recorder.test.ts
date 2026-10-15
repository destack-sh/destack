import { test, expect } from "@destack/test";
import { ObjectError } from "../src/index.ts";
import { AuditStorage, documentRename, rename, INTEGRATION } from "./fixture/journal.ts";
import { CallEvent, defineAuditAction } from "../src/index.ts";
import { document } from "./fixture/journal.ts";
import { schema } from "@destack/schema";
import { AuditRecorder } from "../src/server/index.ts";

import { journal } from "../src/index.ts";
import type { JournalEntry } from "../src/index.ts";
import { RequestId } from "@destack/runtime/request";

test("keep each call in the history of its scope", async () => {
    const storage = await AuditStorage.open();
    try {
        // record a call of a space
        const scope = schema
            .identifier("space")
            .parse("space-01996ab0-0000-7000-8000-000000000001");
        const recorder = new AuditRecorder(
            {
                ...INTEGRATION,
                package: documentRename.package,
                scope,
            },
            storage.journal,
        );
        const recorded = await recorder.record(undefined, documentRename, {
            ...rename,
            outcome: { kind: "success" },
        });

        // read it in the space's history and nowhere else
        expect([await storage.calls(scope), await storage.calls("universe")]).toEqual([
            [recorded],
            [],
        ]);
    } finally {
        await storage.close();
    }
});

test("commit and roll back application changes with their recorded calls", async () => {
    let storage = await AuditStorage.open();
    try {
        // roll back application state and recorded calls together
        const failure = new ObjectError("CONFLICT", "cancel change");
        await expect(
            storage.database.transaction(async (transaction) => {
                await transaction.insert(document).values({ name: "rolled back" });
                await storage.recorder.record(transaction, documentRename, {
                    ...rename,
                    outcome: { kind: "success" },
                });
                throw failure;
            }),
        ).rejects.toBe(failure);
        expect(await storage.database.select().from(document)).toEqual([]);
        expect(await storage.calls("universe")).toEqual([]);

        // keep committed changes and calls across a restart
        const call = await storage.database.transaction(async (transaction) => {
            await transaction.insert(document).values({ name: "renamed" });

            return storage.recorder.record(transaction, documentRename, {
                ...rename,
                outcome: { kind: "success" },
            });
        });
        storage = await storage.reopen();
        expect(await storage.database.select().from(document)).toEqual([{ name: "renamed" }]);
        expect(await storage.calls("universe")).toEqual([call]);
    } finally {
        await storage.close();
    }
});

test("record a running call, then its outcome once, accepting repeats and refusing changes", async () => {
    let storage = await AuditStorage.open();
    try {
        // persist a prepared call only on append, with its outcome in the same record
        const running = storage.recorder.begin(documentRename, rename);
        expect(await journaled(storage)).toEqual([]);
        await storage.recorder.append(running);
        const finished = storage.recorder.finish(running, { kind: "success" });
        await storage.recorder.append(finished);
        storage = await storage.reopen();
        await storage.recorder.append(CallEvent.parse(JSON.parse(JSON.stringify(finished))));
        expect(await journaled(storage)).toEqual([finished]);

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
            documentRename,
            rename,
            async () => ({ name: "chosen" }),
            (renamed) => ({ name: renamed.name }),
        );
        const calls = await storage.calls("universe");
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
        const accountSignIn = defineAuditAction(
            {
                name: "account.signIn",
                target: documentRename.target,
                details: schema.object({
                    method: schema.string(),
                    code: schema.sensitive(schema.string()).exactOptional(),
                    account: schema
                        .object({ id: schema.string(), secret: schema.sensitive(schema.string()) })
                        .exactOptional(),
                }),
            },
            { package: documentRename.package },
        );

        // keep the method in the running call's details
        const target = rename.target;
        const running = storage.recorder.begin(accountSignIn, {
            target,
            details: { method: "code", code: "123456" },
        });
        expect(running.execution.details).toEqual({ method: "code" });

        // keep the account's identifier in the read's details
        const read = await storage.recorder.read(
            accountSignIn,
            { target, details: { method: "code" } },
            async () => ({ id: "account-2", secret: "hunter3" }),
            (account) => ({ method: "code", account }),
        );
        const calls = await storage.calls("universe");
        expect([read, calls.map((call) => call.execution.details)]).toEqual([
            { id: "account-2", secret: "hunter3" },
            [{ method: "code", account: { id: "account-2" } }],
        ]);
    } finally {
        await storage.close();
    }
});

test("replay derived requests of one key at different times as one request, and another key as another", async () => {
    const storage = await AuditStorage.open();
    try {
        // record a call under a request derived for a key
        const now = Date.now();
        const first = await RequestId.derive(now - 60_000, "trigger-1 event-1");
        const later = await RequestId.derive(now, "trigger-1 event-1");
        const other = await RequestId.derive(now, "trigger-1 event-2");
        const request = { caller: "machine-1", position: 0, digest: "digest-1" };
        await storage.recorder.record(
            undefined,
            documentRename,
            { ...rename, outcome: { kind: "success", value: "renamed" } },
            "activity",
            { ...request, requestId: first },
        );

        // replay the key's later request, and run another key's
        const replayed = await Promise.all(
            [later, other].map((requestId) =>
                storage.journal.replay(
                    { caller: request.caller, scope: "universe", requestId },
                    request.digest,
                ),
            ),
        );
        expect(replayed).toEqual([["renamed"], undefined]);
    } finally {
        await storage.close();
    }
});

/** Read the calls the journal keeps for replay, running or ended. */
async function journaled(storage: AuditStorage): Promise<JournalEntry[]> {
    const rows = await storage.database.select({ call: journal.call }).from(journal);

    return rows.map((row) => row.call);
}
