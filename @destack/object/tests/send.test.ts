import { reconciliation, testCallKey } from "@destack/service/test";
import { expect, onTestFinished, test } from "@destack/test";
import { principal, relation } from "@destack/access";
import { journal } from "@destack/audit";
import { defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema, present } from "@destack/schema";

import { outbox } from "@destack/service/outbox";
import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import type { RunDelivery, RunRequest } from "@destack/service/trigger";
import { defineObject, field } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { openSpace, space } from "./fixture/space.ts";
import { userContext } from "./fixture/user.ts";

/** The space with the accounts. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000021");

/** Accounts their owners sign up, each welcomed by a mail sent after the sign-up commits. */
const member = defineObject({
    name: "member",
    plural: "members",
    scope: space,
    fields: { owner: field.reference(principal.user).caller(), name: field.string() },
    permissions: { write: relation("owner") },
    methods: (method) => ({
        create: method.create("write", { fields: ["name"] }),
        welcome: method.update("write", { fields: ["name"] }),
    }),
}).handle({
    // sign up, sending the welcome, then refuse a sign-up of the refused name after sending
    create: async (call, next) => {
        const name = call.input.name;
        await call.send({ call: member.calls().welcome({ id: "member-1", name }) });
        if (name === "Refused") {
            throw new TypeError("the sign-up is refused");
        }

        return next();
    },
});

/** The database with the members, the journal and the outbox. */
const memberDatabase = defineDatabase({
    name: "main",
    tables: [journal, outbox, ...member.tables],
});

test.each(TEST_DIALECTS)(
    "send a call from a method once its transaction commits, nothing from one that rolls back, and let go of one the cell refuses for good, on %s",
    async (dialect) => {
        // serve the members with a cell recording their runs
        const storage = await TestDatabase.create(dialect, memberDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const recorded: [RunRequest, RunDelivery | undefined][] = [];
        const reports: string[] = [];
        const server = new ObjectServer({
            objects: { member },
            database: storage.database,
            callKey: testCallKey,
            origin: {
                package: member.package,
                service: "test",
            },
            report: (error) => {
                if (!(error instanceof Error)) {
                    throw error;
                }
                reports.push(error.message);
            },
            runs: {
                // refuse the welcome of the bounced name for good
                send: async (request, delivery) => {
                    if (request.call.input["name"] === "Bounced") {
                        throw new ServiceError("BAD_REQUEST", { message: "invalid call" });
                    }
                    recorded.push([request, delivery]);
                },
            },
        });
        const context = userContext("user-1", spaceId);
        const signUp = (name: string) =>
            server.call(
                member,
                "create",
                { spaceId, requestId: RequestId.create(), name },
                context,
            );

        // sign up a name the cell refuses before another, and fail a sign-up after it sent
        await signUp("Bounced");
        await signUp("Ada");
        await expect(signUp("Refused")).rejects.toThrow("the sign-up is refused");

        // deliver the outbox through the server's controller, twice to show one delivery
        const sends = present(
            server.controllers().find((controller) => controller.name === "runs"),
            "the runs controller",
        );
        const signal = new AbortController().signal;
        await sends.reconcile("runs", reconciliation(signal));
        await sends.reconcile("runs", reconciliation(signal));

        // record Ada's welcome past the refused one, reporting the refusal once
        expect([
            recorded.map(([request, delivery]) => [request, typeof delivery?.requestId]),
            reports,
        ]).toEqual([
            [
                [
                    {
                        call: member.calls().welcome({ id: "member-1", name: "Ada" }),
                    },
                    "string",
                ],
            ],
            ["the cell refused a sent call for good"],
        ]);
    },
);
