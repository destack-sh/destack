import { expect, test } from "@destack/test";
import { Caller } from "@destack/service/authentication";
import { ServiceContext } from "@destack/service/server";
import { ResourceContext } from "@destack/resource/context";
import { ServiceError } from "@destack/service";
import { identifier } from "@destack/schema";
import { createRecorder } from "../src/server/index.ts";
import { AuditStorage, renameDocument, rename } from "./storage.ts";

test("persist verified caller identities and isolate history by authority", async () => {
    const storage = await AuditStorage.open();
    try {
        // use the same identifier under two independent identity authorities
        const origin = { package: renameDocument.package, service: "document" };
        const now = Date.now();
        const represented = { kind: "user" as const, authority: "global", id: "person" };
        const actor = { kind: "service-account" as const, authority: "space-example", id: "agent" };
        const deploymentId = identifier("deployment").parse(
            "deployment-01996ab0-0000-7000-8000-000000000001",
        );
        const requests = [
            { subject: represented },
            { subject: { ...represented, authority: "host-example" } },
            {
                subject: represented,
                actor,
                deployments: [
                    {
                        subject: { ...actor, authority: "another-space" },
                        id: identifier("deployment").parse(
                            "deployment-01996ab0-0000-7000-8000-000000000002",
                        ),
                    },
                    { subject: actor, id: deploymentId },
                ],
                delegations: [
                    {
                        id: "delegation-example",
                        subject: represented,
                        actor,
                        permissions: [],
                        createdAt: now,
                        expiresAt: now + 60000,
                        revokedAt: null,
                    },
                ],
            },
            { subject: { kind: "share-token" as const, authority: "space-example", id: "share" } },
        ];
        const events = [];
        for (const authentication of requests) {
            const caller = new Caller({
                ...authentication,
                credential: { secret: "must never be recorded" },
                audience: origin.package.id,
                subjects: [authentication.subject],
                verifiedAt: now,
                expiresAt: now + 60000,
            });
            const request = new ServiceContext(
                new Request("https://example.test"),
                origin.package.id,
                "global",
                caller,
                new ResourceContext(),
            );
            const recorder = createRecorder(request, storage.outbox, origin);
            const event = recorder.begin(renameDocument, rename);
            await recorder.append(event);
            events.push(event);
        }

        // failed authentication must not attribute the attempt to a retained caller
        const rejected = new ServiceContext(
            new Request("https://example.test"),
            origin.package.id,
            "global",
            new Caller({
                credential: {},
                audience: origin.package.id,
                subject: represented,
                subjects: [represented],
                verifiedAt: now,
                expiresAt: now + 60000,
            }),
            new ResourceContext(),
            new ServiceError("UNAUTHORIZED"),
        );
        const recorder = createRecorder(rejected, storage.outbox, origin);
        const anonymous = recorder.begin(renameDocument, rename);
        await recorder.append(anonymous);
        events.push(anonymous);

        // compare the full recorded identity, without credential or request payload fields
        const person = { type: "user", authority: "global", id: "person" };
        const local = { ...person, authority: "host-example" };
        const software = { type: "service-account", authority: "space-example", id: "agent" };
        const share = { type: "share-token", authority: "space-example", id: "share" };
        expect(events.map(({ context: { requestId: _requestId, ...context } }) => context)).toEqual(
            [
                { ...origin, actor: person, subject: person, delegation: [] },
                { ...origin, actor: local, subject: local, delegation: [] },
                { ...origin, actor: software, subject: person, delegation: [person], deploymentId },
                { ...origin, actor: share, subject: share, delegation: [] },
                { ...origin, actor: { type: "anonymous" }, delegation: [] },
            ],
        );

        // retain exact identities through durable delivery and indexed history queries
        expect(await storage.outbox.flush(storage.history)).toBe(5);
        for (const event of events) {
            const page = await storage.history.list({
                scope: { type: "global" },
                actor: event.context.actor,
                limit: 10,
            });
            expect(page.items.map((record) => record.event)).toEqual([event]);
            expect(page.cursor).toBeNull();
        }
    } finally {
        await storage.close();
    }
});
