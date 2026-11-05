import { Failure } from "@destack/error";
import { expect, refusal, test } from "@destack/test";
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
            error: Failure.http("INTERNAL_SERVER_ERROR", "failed"),
        });
        expect(await refusal(storage.recorder.append(conflicting))).toEqual([
            "CONFLICT",
            "CALL_CONTENTS_DIFFER",
            `call ${running.execution.id} has conflicting contents`,
        ]);
        expect(await storage.calls("universe")).toEqual([finished]);
    } finally {
        await storage.close();
    }
});

test("remove calls past their lifetime as the journal writes again, ended or running, leaving their call events", async () => {
    const storage = await AuditStorage.open({ milliseconds: 50 });
    try {
        // record an ended call and a running one
        const recorded = await storage.recorder.record(undefined, documentRename, {
            ...rename,
            outcome: { kind: "success" },
        });
        const pending = storage.recorder.begin(documentRename, rename);
        await storage.recorder.append(pending);

        // record a third call once both outlive their lifetime
        const { simulation } = storage;
        await simulation.jump(simulation.clock.now() + 50);
        const third = await storage.recorder.record(undefined, documentRename, {
            ...rename,
            outcome: { kind: "success" },
        });

        // keep the third call alone, and the ended calls' events past the journal's lifetime
        const remaining = await storage.database.select({ id: journal.id }).from(journal);
        expect([remaining.map((row) => row.id), await storage.calls("universe")]).toEqual([
            [third.execution.id],
            [recorded, third],
        ]);
    } finally {
        await storage.close();
    }
});
