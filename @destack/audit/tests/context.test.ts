import { expect, test } from "@destack/test";
import { Authentication } from "@destack/service/authentication";
import { ServiceContext } from "@destack/service/server";
import { ResourceContext } from "@destack/resource/context";
import { ServiceError } from "@destack/service";
import { schema } from "@destack/schema";
import { AuditCaller } from "../src/record/index.ts";
import { AuditRecorder } from "../src/server/index.ts";
import { AuditStorage, documentRename, rename } from "./storage.ts";
import { principal } from "@destack/access";

test("persist verified caller identities and tell apart identities of different scopes", async () => {
    const storage = await AuditStorage.open();
    try {
        // use the same identifier in two scopes
        const origin = { package: documentRename.package, service: "document", scope: "universe" };
        const now = Date.now();
        const represented = principal.user.reference("universe", "person");
        const actor = principal.installation.reference("space-example", "agent");
        const deploymentId = schema
            .identifier("deployment")
            .parse("deployment-01996ab0-0000-7000-8000-000000000001");
        const requests = [
            { subject: represented },
            { subject: { ...represented, scope: "host-example" } },
            {
                subject: represented,
                deployments: [
                    {
                        subject: { ...actor, scope: "another-space" },
                        id: schema
                            .identifier("deployment")
                            .parse("deployment-01996ab0-0000-7000-8000-000000000002"),
                    },
                    { subject: actor, id: deploymentId },
                ],
                delegates: [{ subject: actor, authority: "lent" as const }],
            },
            { subject: principal.installation.reference("space-example", "share") },
        ];
        const calls = [];
        const credential = { kind: "token", id: "token-1", secret: "must never be recorded" };
        for (const claims of requests) {
            const authentication = new Authentication({
                ...claims,
                credential,
                audience: origin.package.id,
                subjects: [claims.subject],
                verifiedAt: now,
                expiresAt: now + 60000,
            });
            const request = new ServiceContext(new Request("https://example.test"), {
                audience: origin.package.id,
                scope: "universe",
                authentication,
                resources: new ResourceContext(),
            });
            const recorder = AuditRecorder.from(
                request.authentication,
                storage.journal,
                origin,
                request.requestId,
            );
            const event = recorder.begin(documentRename, rename);
            await recorder.append(event);
            calls.push(event);
        }

        // attribute an attempt without authentication to no caller
        const rejected = new ServiceContext(new Request("https://example.test"), {
            audience: origin.package.id,
            scope: "universe",
            authentication: null,
            resources: new ResourceContext(),
            authenticationError: new ServiceError("UNAUTHORIZED", {
                message: "invalid bearer credential",
            }),
        });
        const recorder = AuditRecorder.from(
            rejected.authentication,
            storage.journal,
            origin,
            rejected.requestId,
        );
        const anonymous = recorder.begin(documentRename, rename);
        await recorder.append(anonymous);
        calls.push(anonymous);

        // compare the full recorded identity
        const person = { type: "subject", subject: represented };
        const local = { type: "subject", subject: { ...represented, scope: "host-example" } };
        const share = {
            type: "subject",
            subject: principal.installation.reference("space-example", "share"),
        };
        expect(calls.map((call) => call.execution.context)).toEqual([
            { ...origin, caller: person },
            { ...origin, caller: local },
            {
                ...origin,
                caller: { ...person, delegates: [{ subject: actor, authority: "lent" }] },
                deploymentId,
            },
            { ...origin, caller: share },
            { ...origin, caller: { type: "anonymous" } },
        ]);

        // read the identities back through delivery and history queries
        expect(await storage.journal.deliver(storage.history)).toBe(5);
        for (const event of calls) {
            const page = await storage.history.list({
                scope: "universe",
                actor: AuditCaller.actor(event.execution.context.caller),
                limit: 10,
            });
            expect(page.items.map((record) => record.call)).toEqual([event]);
            expect(page.cursor).toBeNull();
        }
    } finally {
        await storage.close();
    }
});
