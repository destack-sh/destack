import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { schema } from "@destack/schema";
import { Authentication } from "@destack/service/authentication";
import { Health } from "@destack/service/health";
import { ServiceError } from "@destack/service/error";
import { Server } from "@destack/service/server";
import { expect, refusal, test } from "@destack/test";
import { principal } from "@destack/access";
import { createAuditClient } from "../src/client/index.ts";
import { defineAuditAction } from "../src/declare/index.ts";
import { AuditContext } from "../src/record/index.ts";
import { AuditRecorder, implementAudit } from "../src/server/index.ts";
import { AuditStorage, INTEGRATION } from "./storage.ts";

/** A document action delivered through the service. */
const documentPublish = defineAuditAction(
    {
        name: "document.publish",
        target: schema.object({ type: schema.literal("document"), id: schema.string() }),
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

/** Connect to a server's audit service. */
function client(server: { fetch(request: Request): Promise<Response> }) {
    return createAuditClient({
        url: "http://audit.local",
        fetch: (request) => server.fetch(request),
    });
}

/** The account whose calls another machine delivers. */
const ACCOUNT = "account-01995da9-7223-7000-8000-000000000001";

test("take ended calls an admitted machine delivers into the history, refusing a machine the intake refuses", async () => {
    const storage = await AuditStorage.open();
    try {
        // record a publish in another machine's journal
        const recorder = new AuditRecorder(
            AuditContext.parse({
                ...INTEGRATION,
                package: documentPublish.package,
                service: "document",
                scope: ACCOUNT,
            }),
            storage.journal,
        );
        const published = await recorder.record(undefined, documentPublish, {
            target: { type: "document", id: "one" },
            details: { revision: 3 },
            outcome: { kind: "success" },
        });

        // serve the history, admitting the account's machine alone
        const admitted: string[][] = [];
        await using history = Server.start({
            ...implementAudit({
                history: storage.history,
                intake: async (_context, scopes) => {
                    admitted.push([...scopes]);
                    if (scopes.some((scope) => scope !== ACCOUNT)) {
                        throw new ServiceError("FORBIDDEN", {
                            message: "the machine keeps no calls of another account",
                        });
                    }
                },
            }),
            audience: documentPublish.package.id,
            scope: "universe",
            resources: new ResourceContext(),
            health: new Health("audit"),
            authenticate: async () =>
                new Authentication({
                    credential: { kind: "fixture", id: "fixture" },
                    audience: documentPublish.package.id,
                    subject: principal.machine.reference("universe", "machine-1"),
                    subjects: [principal.machine.reference("universe", "machine-1")],
                    verifiedAt: Date.now(),
                    expiresAt: Date.now() + 60000,
                }),
            authorizeMachine: async () => {},
            drainTimeout: 1000,
        });

        // deliver the call, then as another account's
        const stored = await client(history).ingest({ calls: [published] });
        const foreign = {
            ...published,
            execution: {
                ...published.execution,
                context: {
                    ...published.execution.context,
                    scope: "account-01995da9-7223-7000-8000-000000000002",
                },
            },
        };
        const refused = await refusal(client(history).ingest({ calls: [foreign] }));

        expect({ stored, admitted, kept: await storage.calls(ACCOUNT), refused }).toEqual({
            stored: { stored: 1 },
            admitted: [[ACCOUNT], ["account-01995da9-7223-7000-8000-000000000002"]],
            kept: [published],
            refused: ["FORBIDDEN", "the machine keeps no calls of another account"],
        });
    } finally {
        await storage.close();
    }
});
