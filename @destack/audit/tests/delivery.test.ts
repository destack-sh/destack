import { test, expect, refusal } from "@destack/test";
import { testCallKey } from "@destack/service/test";
import { ControlLoop } from "@destack/service/control";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { AuditError } from "../src/error/index.ts";
import type { AuditCall } from "../src/record/index.ts";
import { AuditRecorder, Journal } from "../src/server/index.ts";
import { journal } from "../src/stack/index.ts";
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

test("relay an instance's journal with its installation and instance as provenance, refusing an invalid batch, calls of another space or package, and calls naming a provenance", async () => {
    const storage = await AuditStorage.open();
    try {
        // record a finished rename in the instance's space
        const space = schema
            .identifier("space")
            .parse("space-01996ab0-0000-7000-8000-000000000001");
        const recorder = new AuditRecorder(
            {
                caller: { type: "system", name: "document" },
                package: renameDocument.package,
                service: "document",
                scope: space,
            },
            storage.journal,
        );
        const call = recorder.finish(recorder.begin(renameDocument, rename), { kind: "success" });
        const provenance = {
            scope: space,
            packageId: renameDocument.package.id,
            installationId: schema
                .identifier("installation")
                .parse("installation-01996ab0-0000-7000-8000-000000000002"),
            instanceId: schema
                .identifier("instance")
                .parse("instance-01996ab0-0000-7000-8000-000000000003"),
        };
        const relay = (relayed: AuditCall) =>
            refusal(storage.history.relay({ calls: [relayed] }, provenance));
        const context = (fields: Partial<AuditCall["execution"]["context"]>): AuditCall => ({
            ...call,
            execution: { ...call.execution, context: { ...call.execution.context, ...fields } },
        });

        // refuse an empty batch, another space, another package and a forged provenance, then relay the call twice
        const other = PackageId.parse("package-01996ab0-0000-7000-8000-000000000009");
        expect([
            await refusal(storage.history.relay({ calls: [] }, provenance)),
            await relay(context({ scope: "space-01996ab0-0000-7000-8000-000000000008" })),
            await relay(context({ package: { ...renameDocument.package, id: other } })),
            await relay(context({ installationId: provenance.installationId })),
            await relay(context({ instanceId: provenance.instanceId })),
            await relay(call),
            await relay(call),
        ]).toEqual([
            ["INVALID_EVENT", "invalid journal batch"],
            [
                "FORBIDDEN",
                `installation ${provenance.installationId} records no calls of ${renameDocument.package.id} in space-01996ab0-0000-7000-8000-000000000008`,
            ],
            [
                "FORBIDDEN",
                `installation ${provenance.installationId} records no calls of ${other} in ${space}`,
            ],
            ["FORBIDDEN", "only the host records a call's provenance"],
            ["FORBIDDEN", "only the host records a call's provenance"],
            "done",
            "done",
        ]);

        // keep the call once, with the installation and the instance the host verified
        const { items } = await storage.history.list({ scope: space, limit: 100 });
        expect(items.map((record) => record.call.execution.context)).toEqual([
            {
                ...call.execution.context,
                installationId: provenance.installationId,
                instanceId: provenance.instanceId,
            },
        ]);
    } finally {
        await storage.close();
    }
});
