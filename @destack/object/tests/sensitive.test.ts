import { none, through, Caller } from "@destack/access";
import { call, journal } from "../src/index.ts";
import { defineDatabase, TABLE } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { schema, Clock } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { reconciliation, testCallKey } from "@destack/runtime/test";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { startTelemetry } from "@destack/telemetry/bun";
import { OtlpExporter } from "@destack/telemetry/otlp";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { v7 } from "uuid";
import { defineObject, field } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { auditedCalls } from "./fixture/audit.ts";

/** An API key a caller hands a method, which nothing derived from the call keeps. */
const ApiKey = schema.sensitive(schema.string().regex(/^sk-/u));

/** A key holder taking a sensitive key it keeps nowhere and a token kept in its row. */
const holder = defineObject({
    name: "holder",
    plural: "holders",
    scope: space,
    fields: { label: field.string(), token: field.string().sensitive().optional() },
    permissions: { read: through("space", "read"), provision: none() },
}).methods((method) => ({
    create: method.create("read", {
        fields: ["label", "token"],
        input: schema.object({ key: ApiKey }),
    }),
    /** Create a holder as the space. */
    provision: method.create("provision", {
        isInternal: true,
        fields: ["label"],
        input: schema.object({ key: ApiKey }),
    }),
}));

test("keep a sensitive input out of the rows, the journal, the audit events, the telemetry and the error of a refused call, and a sensitive column in its row alone", async () => {
    // serve the holders, recording audit events and exporting telemetry to a list
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({ name: "main", tables: [...holder.tables, space.table, ...callTables] }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const exported: string[] = [];
    const telemetry = await startTelemetry(
        new OtlpExporter(
            async (_signal, body) => {
                exported.push(JSON.stringify(body));
            },
            () => {},
        ).options(holder.package),
    );
    onTestFinished(() => telemetry.shutdown());
    const spaceId = `space-${v7()}`;
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([holder]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: holder.package },
    });
    await openSpace(storage.database, spaceId);

    // create a holder with a valid key, and refuse one with a key of another shape
    const valid = "sk-valid-0123456789";
    const token = "token-column-0123456789";
    const invalid = "pk-refused-0123456789";
    const create = (key: string) =>
        store
            .mutate(userContext(users.alice, spaceId).caller(), {
                id: RequestId.create(Date.now()),
                calls: [holder.call("create", { spaceId, label: "deploy", key, token })],
            })
            .then(([written]) => written);
    await create(valid);
    const refused = await create(invalid).then(
        () => undefined,
        (error: unknown) => error,
    );
    const refusedSystem = await store
        .mutate(Caller.of(spacePrincipal(spaceId), Date.now()), {
            id: RequestId.create(Date.now()),
            calls: [
                holder.call("provision", {
                    ...holder.in(spaceId),
                    label: "deploy",
                    key: invalid,
                }),
            ],
        })
        .then(
            () => undefined,
            (error: unknown) => error,
        );
    for (const controller of store.controllers()) {
        for (const key of await controller.list()) {
            await controller.reconcile(key, reconciliation());
        }
    }
    await telemetry.flush();

    // read every place a call leaves a trace
    const audited = await auditedCalls(storage.database);
    const rows: Record<string, unknown> = {};
    const calls = call.eventKind().table;
    for (const table of [...holder.tables, journal, calls]) {
        rows[table[TABLE].name] = await storage.database.select().from(table);
    }
    const traces = {
        rows: JSON.stringify(rows),
        audited: JSON.stringify(audited),
        telemetry: exported.join("\n"),
        refused: JSON.stringify([
            refused,
            refused instanceof Error ? refused.message : undefined,
            refusedSystem,
            refusedSystem instanceof Error ? refusedSystem.message : undefined,
            ...[refused, refusedSystem].map((error) =>
                error instanceof Error && "data" in error ? error.data : undefined,
            ),
        ]),
    };

    // list the traces with a key, and the token anywhere but its row
    const leaking = Object.entries(traces).flatMap(([name, text]) =>
        [valid, invalid, ...(name === "rows" ? [] : [token])]
            .filter((key) => text.includes(key))
            .map((key) => `${name}: ${key}`),
    );

    expect({
        leaking,
        refusal: await refusal(Promise.reject(refused)),
        systemRefusal: await refusal(Promise.reject(refusedSystem)),
        audited: audited.map(({ method, input, execution }) => ({
            method,
            input,
            caller: execution.context.caller.subject.type,
            target: execution.target.type,
            outcome:
                execution.outcome.kind === "success" ? "success" : execution.outcome.error.code,
        })),
        isTokenKept: traces.rows.includes(token),
    }).toEqual({
        leaking: [],
        refusal: ["BAD_REQUEST", "invalid input: key: invalid string: must match pattern /^sk-/u"],
        systemRefusal: [
            "BAD_REQUEST",
            "invalid input: key: invalid string: must match pattern /^sk-/u",
        ],
        audited: [
            {
                method: "holder.create",
                input: {},
                caller: "user",
                target: "holder",
                outcome: "success",
            },
            {
                method: "holder.create",
                input: {},
                caller: "user",
                target: "holders",
                outcome: "BAD_REQUEST",
            },
            {
                method: "holder.provision",
                input: {},
                caller: "space",
                target: "holders",
                outcome: "BAD_REQUEST",
            },
        ],
        isTokenKept: true,
    });
});
