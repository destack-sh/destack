import { expect, test } from "@destack/test";
import { RequestContext } from "@destack/runtime/server";
import { ResourceContext } from "@destack/resource/context";
import { CallError } from "@destack/runtime";
import { Clock, Identifier, schema } from "@destack/schema";
import { AuditCaller } from "../src/index.ts";
import { ObjectReference } from "@destack/sync";
import { AuditRecorder } from "../src/server/index.ts";
import { AuditStorage, documentRename, rename } from "./fixture/journal.ts";
import { anyone, type Caller, principal } from "@destack/access";

test("persist verified caller identities with their delegates and grant, record an unauthenticated caller as anyone, and tell apart identities of different scopes", async () => {
    const storage = await AuditStorage.open();
    try {
        // use the same identifier in two scopes
        const origin = { package: documentRename.package, scope: "universe" };
        const now = Date.now();
        const represented = principal.user.reference("universe", "person");
        const grantId = Identifier.create("grant");
        const actor = principal.installation.reference("space-example", "agent");
        const deploymentId = schema
            .identifier("deployment")
            .parse("deployment-01996ab0-0000-7000-8000-000000000001");
        const requests = [
            { subject: represented },
            { subject: { ...represented, scope: "machine-example" } },
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
                delegates: [actor],
                grant: { id: grantId, scope: represented.id, permissions: [] },
            },
            { subject: principal.installation.reference("space-example", "share") },
        ];
        const calls = [];
        const credential = { kind: "token", id: "token-1", secret: "must never be recorded" };
        for (const claims of requests) {
            const caller: Caller = {
                ...claims,
                subjects: [claims.subject],
                credential: {
                    ...credential,
                    audience: origin.package.id,
                    verifiedAt: now,
                    expiresAt: now + 60000,
                },
            };
            const request = new RequestContext(new Request("https://example.test"), {
                clock: Clock.system,
                audience: origin.package.id,
                scope: "universe",
                caller,
                resources: new ResourceContext(),
            });
            const recorder = AuditRecorder.from(
                request.caller(),
                storage.journal,
                origin,
                request.requestId,
            );
            const event = await recorder.record(undefined, documentRename, {
                ...rename,
                outcome: { kind: "success" },
            });
            calls.push(event);
        }

        // attribute an attempt without authentication to no caller
        const rejected = new RequestContext(new Request("https://example.test"), {
            clock: Clock.system,
            audience: origin.package.id,
            scope: "universe",
            resources: new ResourceContext(),
            authenticationError: new CallError("UNAUTHORIZED", {
                message: "invalid bearer credential",
            }),
        });
        const recorder = AuditRecorder.from(
            { subjects: [] },
            storage.journal,
            origin,
            rejected.requestId,
        );
        const unauthenticated = await recorder.record(undefined, documentRename, {
            ...rename,
            outcome: { kind: "success" },
        });
        calls.push(unauthenticated);

        // compare the full recorded identity
        const person = { subject: represented };
        const local = { subject: { ...represented, scope: "machine-example" } };
        const share = {
            subject: principal.installation.reference("space-example", "share"),
        };
        expect(calls.map((call) => call.execution.context)).toEqual([
            { ...origin, caller: person },
            { ...origin, caller: local },
            {
                ...origin,
                caller: { ...person, delegates: [actor] },
                deploymentId,
                grantId,
            },
            { ...origin, caller: share },
            { ...origin, caller: { subject: anyone.reference("*", "*") } },
        ]);

        // read the identities back through queries of the call events
        for (const event of calls) {
            const subject = AuditCaller.actor(event.execution.context.caller).subject;
            expect(
                await storage.calls("universe", ObjectReference.columns("actor", subject)),
            ).toEqual([event]);
        }
    } finally {
        await storage.close();
    }
});
