import { AuditStorage } from "./storage.ts";
import { test, expect } from "@destack/test";
import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service";
import { Health } from "@destack/service/health";
import { AuditRecorder } from "../src/record/index.ts";
import { defineAuditAction } from "../src/action/index.ts";
import { createAuditClient } from "../src/client/index.ts";
import { implementService, createRecorder } from "../src/server/index.ts";
import { auditList } from "../src/history/index.ts";
import { Server } from "@destack/service/server";
import { Caller } from "@destack/service/authentication";
import { ResourceContext } from "@destack/resource/context";
import { AuditContext } from "../src/event/index.ts";
import { PackageId } from "@destack/package";

/** Typed application action exercised through the HTTP service. */
const publishDocument = defineAuditAction({
    package: {
        id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000004"),
        name: "@example/document",
        version: "1.0.0",
    },
    name: "document.publish",
    version: 1,
    targets: schema.object({
        document: schema.object({ type: schema.literal("document"), id: schema.string() }),
    }),
    details: schema.object({ revision: schema.number().int() }),
});

test("authorize producers and readers, stream history, and record denied access", async () => {
    const storage = await AuditStorage.open();
    const { outbox, history } = storage;
    let producerId: string;

    try {
        // bind producer authority separately from history reader credentials
        const context = AuditContext.parse({
            actor: { type: "system", name: "document" },
            delegation: [],
            package: publishDocument.package,
            service: "document",
            accountId: "account-01995da9-7223-7000-8000-000000000001",
        });
        const recorder = new AuditRecorder(context, outbox);
        await using server = await Server.start({
            ...implementService(history, {
                record: (request) =>
                    createRecorder(request, outbox, {
                        package: publishDocument.package,
                        service: "audit",
                        accountId: context.accountId,
                    }),
                authorize: async (access) => {
                    if (access.action === "ingest") {
                        if (
                            access.producerId !== producerId ||
                            access.event.context.accountId !== context.accountId ||
                            access.event.context.service !== "document"
                        ) {
                            throw new ServiceError("FORBIDDEN");
                        }
                    } else if (
                        access.action === "prune" ||
                        access.scope.type !== "account" ||
                        access.scope.accountId !== context.accountId
                    ) {
                        throw new ServiceError("FORBIDDEN");
                    }
                },
            }),
            audience: publishDocument.package.id,
            scope: "global",
            resources: new ResourceContext(),
            health: new Health("audit"),
            authenticate: async () =>
                new Caller({
                    credential: { kind: "fixture" },
                    audience: publishDocument.package.id,
                    subject: { kind: "user", authority: "global", id: "reader" },
                    subjects: [{ kind: "user", authority: "global", id: "reader" }],
                    verifiedAt: Date.now(),
                    expiresAt: Date.now() + 60000,
                }),
            authorizeHost: async () => {},
            drainTimeout: 1000,
        });
        const client = createAuditClient({
            url: "http://audit.local",
            fetch: (request) => server.fetch(request),
        });

        // deliver a real attempt and result through serialization and acknowledgement
        const attempt = recorder.begin(publishDocument, {
            targets: { document: { type: "document", id: "one" } },
            details: { revision: 3 },
        });
        await recorder.append(attempt);
        const result = recorder.complete(attempt, { outcome: "success" });
        await recorder.append(result);
        producerId = (await outbox.next())!.producerId;
        expect(await outbox.flush(client)).toBe(2);
        const scope = { type: "account" as const, accountId: context.accountId! };
        const query = { scope, action: publishDocument.name, limit: 1 };
        const first = await client.list(query);
        expect(first.items.map((record) => record.event)).toEqual([attempt]);
        const second = await client.list({ ...query, cursor: first.cursor! });
        expect(second.items.map((record) => record.event)).toEqual([result]);
        expect(second.cursor).toBeNull();
        const exported = [];
        for await (const record of await client.export(query)) {
            exported.push(record);
        }
        expect(exported).toEqual([...first.items, ...second.items]);

        // reject both a forged producer scope and cross-account history access
        const foreign = "account-01995da9-7223-7000-8000-000000000002";
        await expect(
            client.ingest({
                producerId: "audit-producer-01995da9-7223-7000-8000-000000000010",
                sequence: 1,
                event: { ...attempt, context: { ...context, accountId: foreign } },
            }),
        ).rejects.toMatchObject({ code: "FORBIDDEN" });
        await expect(
            client.get({ scope: { type: "account", accountId: foreign }, id: attempt.id }),
        ).rejects.toMatchObject({ code: "FORBIDDEN" });
        const accesses = (await outbox.read()).filter(
            (event) => event.action.package.id === auditList.package.id,
        );
        expect(accesses.map((event) => event.action.name)).toEqual([
            "audit.list",
            "audit.list",
            "audit.list",
            "audit.list",
            "audit.export",
            "audit.export",
            "audit.get",
            "audit.get",
        ]);
        expect(accesses.map((event) => event.result)).toEqual([
            { stage: "attempt" },
            { stage: "result", outcome: "success" },
            { stage: "attempt" },
            { stage: "result", outcome: "success" },
            { stage: "attempt" },
            { stage: "result", outcome: "success" },
            { stage: "attempt" },
            { stage: "result", outcome: "denied", errorCode: "FORBIDDEN" },
        ]);
        expect(await history.get(scope, attempt.id)).toEqual(first.items[0]);

        // history read access does not grant retention administration
        await expect(
            client.prune({ scope, before: Date.now() + 1, limit: 100 }),
        ).rejects.toMatchObject({ code: "FORBIDDEN" });
        expect(await history.get(scope, attempt.id)).toEqual(first.items[0]);
    } finally {
        await storage.close();
    }
});
