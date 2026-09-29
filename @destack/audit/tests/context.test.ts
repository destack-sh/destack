import { expect, test } from "@destack/test";
import { Caller } from "@destack/service/authentication";
import { ServiceContext } from "@destack/service/server";
import { ResourceContext } from "@destack/resource/context";
import { ServiceError } from "@destack/service";
import { identifier } from "@destack/schema";
import { AuditRecorder } from "../src/record/index.ts";
import { AuditStorage, renameDocument, rename } from "./storage.ts";
import { principal } from "@destack/access";

test("persist verified caller identities and tell apart identities of different scopes", async () => {
    const storage = await AuditStorage.open();
    try {
        // use the same identifier in two scopes
        const origin = { package: renameDocument.package, service: "document", scope: "global" };
        const now = Date.now();
        const represented = principal.user.reference("global", "person");
        const actor = principal.installation.reference("space-example", "agent");
        const deploymentId = identifier("deployment").parse(
            "deployment-01996ab0-0000-7000-8000-000000000001",
        );
        const requests = [
            { subject: represented },
            { subject: { ...represented, scope: "host-example" } },
            {
                subject: represented,
                deployments: [
                    {
                        subject: { ...actor, scope: "another-space" },
                        id: identifier("deployment").parse(
                            "deployment-01996ab0-0000-7000-8000-000000000002",
                        ),
                    },
                    { subject: actor, id: deploymentId },
                ],
                delegates: [{ subject: actor, authority: "lent" as const }],
            },
            { subject: principal.installation.reference("space-example", "share") },
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
            const request = new ServiceContext(new Request("https://example.test"), {
                audience: origin.package.id,
                scope: "global",
                caller,
                resources: new ResourceContext(),
            });
            const recorder = AuditRecorder.from(request.caller, storage.outbox, {
                ...origin,
                requestId: request.requestId,
            });
            const event = recorder.begin(renameDocument, rename);
            await recorder.append(event);
            events.push(event);
        }

        // attribute an attempt without authentication to no caller
        const rejected = new ServiceContext(new Request("https://example.test"), {
            audience: origin.package.id,
            scope: "global",
            caller: null,
            resources: new ResourceContext(),
            authenticationError: new ServiceError("UNAUTHORIZED", {
                message: "invalid bearer credential",
            }),
        });
        const recorder = AuditRecorder.from(rejected.caller, storage.outbox, {
            ...origin,
            requestId: rejected.requestId,
        });
        const anonymous = recorder.begin(renameDocument, rename);
        await recorder.append(anonymous);
        events.push(anonymous);

        // compare the full recorded identity
        const person = { type: "subject", subject: represented };
        const local = { type: "subject", subject: { ...represented, scope: "host-example" } };
        const software = { type: "subject", subject: actor };
        const share = {
            type: "subject",
            subject: principal.installation.reference("space-example", "share"),
        };
        expect(events.map(({ context: { requestId: _requestId, ...context } }) => context)).toEqual(
            [
                { ...origin, actor: person, subject: person.subject, delegation: [] },
                { ...origin, actor: local, subject: local.subject, delegation: [] },
                {
                    ...origin,
                    actor: software,
                    subject: person.subject,
                    delegation: [person],
                    deploymentId,
                },
                { ...origin, actor: share, subject: share.subject, delegation: [] },
                { ...origin, actor: { type: "anonymous" }, delegation: [] },
            ],
        );

        // read the identities back through delivery and history queries
        expect(await storage.outbox.deliver(storage.history)).toBe(5);
        for (const event of events) {
            const page = await storage.history.list({
                scope: "global",
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
