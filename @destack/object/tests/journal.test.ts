import { test, expect } from "@destack/test";
import { ControlLoop } from "@destack/runtime/control";
import { Clock } from "@destack/schema";
import { journal } from "../src/index.ts";
import { AuditStorage, documentRename, rename } from "./fixture/journal.ts";

test("append an ended audited call as one call event, a repeat after a restart appending none", async () => {
    let storage = await AuditStorage.open();
    try {
        // record an ended call
        const recorded = await storage.recorder.record(undefined, documentRename, {
            ...rename,
            outcome: { kind: "success" },
        });

        // record it again after a restart, with the one event
        storage = await storage.reopen();
        await storage.recorder.append(recorded);
        expect(await storage.calls("universe")).toEqual([recorded]);
    } finally {
        await storage.close();
    }
});

test("append a call event once a running call ends, none while it runs", async () => {
    const storage = await AuditStorage.open();
    try {
        // record a running call, appending no event
        const running = storage.recorder.begin(documentRename, rename);
        await storage.recorder.append(running);
        const whileRunning = await storage.calls("universe");

        // record its outcome, appending the ended call
        const finished = storage.recorder.finish(running, { kind: "success" });
        await storage.recorder.append(finished);
        expect([whileRunning, await storage.calls("universe")]).toEqual([[], [finished]]);
    } finally {
        await storage.close();
    }
});

test("refuse a second outcome of a call, its call event recording the first", async () => {
    const storage = await AuditStorage.open();
    try {
        // record a call and its outcome, leaving nothing running
        const running = storage.recorder.begin(documentRename, rename);
        await storage.recorder.append(running);
        const finished = storage.recorder.finish(running, { kind: "success" });
        await storage.recorder.append(finished);

        // refuse a second outcome, with the first as the call event
        const conflicting = storage.recorder.finish(running, {
            kind: "failure",
            error: { code: "FAILED", status: 500, message: "failed" },
        });
        await expect(storage.recorder.append(conflicting)).rejects.toMatchObject({
            code: "CONFLICT",
            message: `call ${running.execution.id} has conflicting contents`,
        });
        expect(await storage.calls("universe")).toEqual([finished]);
    } finally {
        await storage.close();
    }
});

test("remove calls past their lifetime through the journal's controller, ended or running, leaving their call events", async () => {
    const storage = await AuditStorage.open({ milliseconds: 50 });
    const stopping = new AbortController();
    const failures: unknown[] = [];
    const report = (_controller: unknown, _key: string, error: unknown) => failures.push(error);
    try {
        // record an ended call and a running one
        const recorded = await storage.recorder.record(undefined, documentRename, {
            ...rename,
            outcome: { kind: "success" },
        });
        const pending = storage.recorder.begin(documentRename, rename);
        await storage.recorder.append(pending);

        // let the journal's control loop remove both once their lifetime ends
        const loop = new ControlLoop(storage.database, [storage.journal.controller()], {
            clock: Clock.system,
            report,
        }).run(stopping.signal);
        const remaining = async () =>
            (await storage.database.select({ id: journal.id }).from(journal)).map((row) => row.id);
        await storage.database.log.until(
            async () => (await remaining()).length === 0,
            AbortSignal.timeout(2000),
        );
        stopping.abort();
        await loop;

        // retain the ended call's event past the journal's lifetime
        expect([await remaining(), await storage.calls("universe"), failures]).toEqual([
            [],
            [recorded],
            [],
        ]);
    } finally {
        stopping.abort();
        await storage.close();
    }
});
