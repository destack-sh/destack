import { none, through, Caller } from "@destack/access";
import type { AuditCall } from "@destack/audit";
import { journal } from "@destack/audit/stack";
import { defineDatabase, TABLE } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { reconciliation, testCallKey } from "@destack/service/test";
import { startTelemetry } from "@destack/telemetry/bun";
import { OtlpExporter } from "@destack/telemetry/otlp";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { v7 } from "uuid";
import { defineObject, field } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext } from "./fixture/user.ts";

/** An API key a caller hands a method, which nothing derived from the call keeps. */
const ApiKey = schema.sensitive(schema.string().regex(/^sk-/u));

/** A key holder whose creation takes a sensitive key it keeps nowhere, and a sensitive token it keeps in its row alone. */
const holder = defineObject({
    name: "holder",
    plural: "holders",
    scope: space,
    fields: { label: field.string(), token: field.string().sensitive().optional() },
    permissions: { read: through("space", "read"), provision: none() },
    methods: (method) => ({
        get: method.get("read"),
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
    }),
});

test("keep a sensitive input out of the rows, the journal, the audit events, the telemetry and the error of a refused call, and a sensitive column in its row alone", async () => {
    // serve the holders, delivering audit events and exporting telemetry to lists
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({ name: "main", tables: [...holder.tables, space.table, journal] }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const audited: AuditCall[] = [];
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
        principal: spacePrincipal(spaceId),
        objects: { holder },
        database: storage.database,
        callKey: testCallKey,
        origin: { package: holder.package, service: "test" },
        history: {
            ingest: async (batch: { readonly calls: AuditCall[] }) => {
                audited.push(...batch.calls);

                return batch.calls.length;
            },
        },
    });
    await openSpace(storage.database, spaceId);

    // create a holder with a valid key, and refuse one with a key of another shape
    const valid = "sk-valid-0123456789";
    const token = "token-column-0123456789";
    const invalid = "pk-refused-0123456789";
    const create = (key: string) =>
        store
            .mutate(userContext("alice", spaceId).caller(), {
                id: RequestId.create(),
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
            id: RequestId.create(),
            calls: [
                holder.call("provision", {
                    ...holder.target({ scope: spaceId }),
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

    // deliver the journal's audit events, then read every place a call leaves a trace
    const rows: Record<string, unknown> = {};
    for (const table of [...holder.tables, journal]) {
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

    // list the traces holding a key, and the token anywhere but its row
    const leaking = Object.entries(traces).flatMap(([name, text]) =>
        [valid, invalid, ...(name === "rows" ? [] : [token])]
            .filter((key) => text.includes(key))
            .map((key) => `${name}: ${key}`),
    );

    expect({
        leaking,
        refusal: await refusal(Promise.reject(refused)),
        systemRefusal: await refusal(Promise.reject(refusedSystem)),
        isAudited: audited.length > 0,
        isTokenKept: traces.rows.includes(token),
    }).toEqual({
        leaking: [],
        refusal: ["BAD_REQUEST", "invalid input: key: invalid string: must match pattern /^sk-/u"],
        systemRefusal: [
            "BAD_REQUEST",
            "invalid input: key: invalid string: must match pattern /^sk-/u",
        ],
        isAudited: true,
        isTokenKept: true,
    });
});
