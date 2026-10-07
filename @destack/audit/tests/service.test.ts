import { AuditContext } from "../src/record/index.ts";
import { AuditStorage } from "./storage.ts";
import { test, expect } from "@destack/test";
import { schema } from "@destack/schema";
import { Health } from "@destack/service/health";
import { defineAuditAction } from "../src/declare/index.ts";
import { createAuditClient } from "../src/client/index.ts";
import { AuditRecorder, implementAudit } from "../src/server/index.ts";
import { Server } from "@destack/service/server";
import { Authentication } from "@destack/service/authentication";
import { ResourceContext } from "@destack/resource/context";
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
import { AccessFixture } from "@destack/access/test";
import { type AuditCall, call } from "../src/record/index.ts";

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
const documentPublish = defineAuditAction(
    {
        name: "document.publish",
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

/** Read a call as the history keeps it once delivered from the account inside the owner's scope. */
function delivered(recorded: AuditCall): AuditCall {
    const context = { ...recorded.execution.context, chain: ["universe", "owner"] };

    return { ...recorded, execution: { ...recorded.execution, context } };
}

test("authorize readers, list and stream history, record denied access, and take no calls to record", async () => {
    const storage = await AuditStorage.open();
    const { journal, history } = storage;

    try {
        // bind the producer context
        const context = AuditContext.parse({
            caller: { type: "system", name: "document" },
            package: documentPublish.package,
            service: "document",
            scope: "account-01995da9-7223-7000-8000-000000000001",
        });
        const recorder = new AuditRecorder(context, journal);

        // let the reader read but not prune
        const database = storage.database;
        const accountObject = account.reference("owner", context.scope);
        const authorizer = new Authorizer(
            [call, account],
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
        await new AccessFixture(database).copyScope(principal.user.reference("universe", "owner"));
        await database.insert(accountRecord).values({ id: context.scope, scope: "owner" });
        const [subject] = owner.subjects;
        if (subject === undefined) {
            throw new TypeError("the owner has no subject");
        }
        await asOwner.create(accountObject, { owner: subject });
        const reader = await asOwner.createRole(accountObject, {
            name: "reader",
            description: "Reads the account's history",
            permissions: [call.permission("read")],
        });
        await asOwner.grant({
            object: accountObject,
            role: reader.id,
            subject: principal.user.reference("universe", "reader"),
        });
        await using server = Server.start({
            ...implementAudit({
                history,
                access: { authorizer, database },
                record: (request) =>
                    AuditRecorder.from(
                        request.authentication,
                        journal,
                        {
                            package: documentPublish.package,
                            service: "audit",
                            scope: context.scope,
                        },
                        request.requestId,
                    ),
            }),
            audience: documentPublish.package.id,
            scope: "universe",
            resources: new ResourceContext(),
            health: new Health("audit"),
            authenticate: async () =>
                new Authentication({
                    credential: { kind: "fixture", id: "fixture" },
                    audience: documentPublish.package.id,
                    subject: principal.user.reference("universe", "reader"),
                    subjects: [principal.user.reference("universe", "reader")],
                    verifiedAt: Date.now(),
                    expiresAt: Date.now() + 60000,
                }),
            authorizeMachine: async () => {},
            drainTimeout: 1000,
        });
        const client = createAuditClient({
            url: "http://audit.local",
            fetch: (request) => server.fetch(request),
        });

        // deliver two publishes to the history as one batch, and page through them
        const publish = (revision: number) =>
            recorder.record(undefined, documentPublish, {
                targets: { document: { type: "document", id: "one" } },
                details: { revision },
                outcome: { kind: "success" },
            });
        const earlier = await publish(3);
        const later = await publish(4);
        expect(await journal.deliver(history)).toBe(2);
        const scope = context.scope;
        const query = { scope, method: documentPublish.name, limit: 1 };
        const first = await history.list(query);
        expect(first.items.map((record) => record.call)).toEqual([delivered(earlier)]);
        if (first.cursor === null) {
            throw new TypeError("the first page has no cursor");
        }
        const second = await history.list({ ...query, cursor: first.cursor });
        expect(second.items.map((record) => record.call)).toEqual([delivered(later)]);
        expect(second.cursor).toBeNull();
        expect(await client.list(query)).toEqual(first);
        const exported = [];
        for await (const record of await client.export(query)) {
            exported.push(record);
        }
        expect(exported).toEqual([...first.items, ...second.items]);

        // answer no route recording calls, which only the history's own host stores
        const recorded = await server.fetch(
            new Request("http://audit.local/audit/calls", {
                method: "POST",
                headers: { "content-type": "application/json" },
                body: JSON.stringify({ calls: [earlier] }),
            }),
        );
        expect(recorded.status).toBe(404);

        // refuse cross-account access
        const foreign = "account-01995da9-7223-7000-8000-000000000002";
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
        const calls = await journal.read();
        const accesses = calls.filter((entry) => entry.execution.category === "access");
        expect(
            accesses.map((entry) => [
                entry.method,
                entry.execution.category,
                entry.execution.outcome,
            ]),
        ).toEqual([
            ["audit.list", "access", { kind: "success" }],
            ["audit.export", "access", { kind: "success" }],
        ]);

        // record each refused call once, as its procedure's denial
        const denials = calls.filter((entry) => entry.execution.category === "denial");
        expect(
            denials.map((entry) => [
                entry.method,
                entry.execution.targets,
                entry.execution.outcome,
            ]),
        ).toEqual([
            [
                "service.invoke",
                { procedure: { type: "procedure", id: "export" } },
                {
                    kind: "denied",
                    error: { code: "FORBIDDEN", status: 403, message: "permission denied: read" },
                },
            ],
        ]);
        expect(await history.get(scope, earlier.execution.id)).toEqual(first.items[0]);

        // deny pruning to a reader
        await expect(
            client.prune({ scope, before: Date.now() + 1, limit: 100 }),
        ).rejects.toMatchObject({ code: "FORBIDDEN" });
        expect(await history.get(scope, earlier.execution.id)).toEqual(first.items[0]);
    } finally {
        await storage.close();
    }
});
