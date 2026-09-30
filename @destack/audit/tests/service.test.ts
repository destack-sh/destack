import { AuditStorage } from "./storage.ts";
import { test, expect } from "@destack/test";
import { schema } from "@destack/schema";
import { Health } from "@destack/service/health";
import { AuditRecorder } from "../src/record/index.ts";
import { defineAuditAction } from "../src/action/index.ts";
import { createAuditClient } from "../src/client/index.ts";
import { implementService } from "../src/server/index.ts";
import { Server } from "@destack/service/server";
import { Caller } from "@destack/service/authentication";
import { ResourceContext } from "@destack/resource/context";
import { AuditContext } from "../src/event/index.ts";
import { PackageId } from "@destack/package";
import {
    Authorizer,
    none,
    Policy,
    principal,
    type AccessContext,
    Authorization,
} from "@destack/access";
import { accountRecord } from "./stack/index.ts";
import { copyScope } from "@destack/access/test";
import { event } from "../src/history/index.ts";

/** The accounts whose histories the test reads. */
const account = new Policy(
    {
        id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000005"),
        name: "@example/account",
        version: "2026.9.0",
    },
    {
        name: "account",
        relations: {},
        permissions: { share: none() },
        grantedBy: "share",
        scope: true,
    },
);

/** A document action recorded through the service. */
const publishDocument = defineAuditAction(
    {
        name: "Document.publish",
        targets: schema.object({
            document: schema.object({ type: schema.literal("document"), id: schema.string() }),
        }),
        details: schema.object({ revision: schema.number().int() }),
    },
    {
        package: {
            id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000004"),
            name: "@example/document",
            version: "2026.9.0",
        },
    },
);

test("authorize producers and readers, stream history, and record denied access", async () => {
    const storage = await AuditStorage.open();
    const { outbox, history } = storage;

    try {
        // bind the producer context
        const context = AuditContext.parse({
            actor: { type: "system", name: "document" },
            delegation: [],
            package: publishDocument.package,
            service: "document",
            scope: "account-01995da9-7223-7000-8000-000000000001",
        });
        const recorder = new AuditRecorder(context, outbox);

        // let the reader ingest and read but not prune
        const database = storage.database;
        const accountObject = account.reference("owner", context.scope);
        const authorizer = new Authorizer(
            [event, account],
            [
                {
                    policy: account,
                    table: accountRecord,
                    id: "id",
                    scope: "scope",
                    attributes: {},
                    relations: {},
                },
            ],
        );
        const owner: AccessContext = {
            subjects: [principal.user.reference("universe", "owner")],
            now: Date.now(),
            attributes: {},
        };
        const asOwner = new Authorization(authorizer, database, () => owner);
        await copyScope(database, principal.user.reference("universe", "owner"));
        await database.insert(accountRecord).values({ id: context.scope, scope: "owner" });
        await asOwner.create(accountObject, { owner: owner.subjects[0]! });
        const reader = await asOwner.createRole(accountObject, {
            name: "reader",
            description: "Records and reads the account's history",
            permissions: [event.permission("ingest"), event.permission("read")],
        });
        await asOwner.grant({
            object: accountObject,
            role: reader.id,
            subject: principal.user.reference("universe", "reader"),
        });
        await using server = Server.start({
            ...implementService(history, {
                access: { authorizer, database },
                record: (request) =>
                    AuditRecorder.from(request.caller, outbox, {
                        package: publishDocument.package,
                        service: "audit",
                        scope: context.scope,
                        requestId: request.requestId,
                    }),
            }),
            audience: publishDocument.package.id,
            scope: "universe",
            resources: new ResourceContext(),
            health: new Health("audit"),
            authenticate: async () =>
                new Caller({
                    credential: { kind: "fixture", id: "fixture" },
                    audience: publishDocument.package.id,
                    subject: principal.user.reference("universe", "reader"),
                    subjects: [principal.user.reference("universe", "reader")],
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

        // deliver an attempt and its result as one batch
        const attempt = recorder.begin(publishDocument, {
            targets: { document: { type: "document", id: "one" } },
            details: { revision: 3 },
        });
        await recorder.append(attempt);
        const result = recorder.complete(attempt, { outcome: "success" });
        await recorder.append(result);
        expect(await outbox.deliver(client)).toBe(2);
        const scope = context.scope;
        const query = { scope, action: publishDocument.name, limit: 1 };
        const first = await history.list(query);
        expect(first.items.map((record) => record.event)).toEqual([attempt]);
        const second = await history.list({ ...query, cursor: first.cursor! });
        expect(second.items.map((record) => record.event)).toEqual([result]);
        expect(second.cursor).toBeNull();
        const exported = [];
        for await (const record of await client.export(query)) {
            exported.push(record);
        }
        expect(exported).toEqual([...first.items, ...second.items]);

        // reject a foreign scope and cross-account access
        const foreign = "account-01995da9-7223-7000-8000-000000000002";
        await expect(
            client.ingest({
                events: [{ ...attempt, context: { ...context, scope: foreign } }],
            }),
        ).rejects.toMatchObject({ code: "FORBIDDEN" });
        const denied = async () => {
            const records = [];
            for await (const record of await client.export({
                ...query,
                scope: foreign,
            })) {
                records.push(record);
            }

            return records;
        };
        await expect(denied()).rejects.toMatchObject({ code: "FORBIDDEN" });
        const events = await outbox.read();
        const accesses = events.filter((event) => event.category === "access");
        expect(accesses.map((event) => [event.action.name, event.category, event.result])).toEqual([
            ["Audit.export", "access", { stage: "result", outcome: "success" }],
        ]);

        // record each refused call once, as its procedure's denial
        const denials = events.filter((event) => event.category === "denial");
        expect(denials.map((event) => [event.action.name, event.targets, event.result])).toEqual([
            [
                "Service.invoke",
                { procedure: { type: "procedure", id: "ingest" } },
                { stage: "result", outcome: "denied", errorCode: "FORBIDDEN" },
            ],
            [
                "Service.invoke",
                { procedure: { type: "procedure", id: "export" } },
                { stage: "result", outcome: "denied", errorCode: "FORBIDDEN" },
            ],
        ]);
        expect(await history.get(scope, attempt.id)).toEqual(first.items[0]);

        // deny pruning to a reader
        await expect(
            client.prune({ scope, before: Date.now() + 1, limit: 100 }),
        ).rejects.toMatchObject({ code: "FORBIDDEN" });
        expect(await history.get(scope, attempt.id)).toEqual(first.items[0]);
    } finally {
        await storage.close();
    }
});
