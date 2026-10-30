import { testSimulation } from "../src/test/index.ts";
import { AccessContext, Caller, none, through } from "@destack/access";
import { call, journal, defineObject, field } from "../src/index.ts";
import { defineDatabase, TABLE, ControlLoop } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { schema, Uuid } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { startTelemetry } from "@destack/telemetry/bun";
import { OtlpExporter } from "@destack/telemetry/otlp";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { auditedCalls } from "./fixture/audit.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** An API key a caller hands a method, which nothing derived from the call stores. */
const ApiKey = schema.sensitive(schema.string().regex(/^sk-/u));

/** A key holder taking a sensitive key it stores nowhere and a token stored in its row. */
const holder = defineObject(
    {
        name: "holder",
        plural: "holders",
        scope: space,
        fields: { label: field.string(), token: field.string().sensitive().optional() },
        permissions: { read: through("space", "read"), provision: none() },
    },
    (method) => ({
        create: method.create("read", {
            fields: ["label", "token"],
            input: schema.object({ key: ApiKey }),
        }),
        /** Create a holder as the space. */
        provision: method.create("provision", {
            visibility: "package",
            fields: ["label"],
            input: schema.object({ key: ApiKey }),
        }),
    }),
);

test("leave a sensitive input out of the rows, the journal, the audit events, the telemetry and the error of a refused call, and a sensitive column in its row alone", async () => {
    // run in a world of its own
    const simulation = testSimulation();
    const world = simulation.environment("test");

    // serve the holders, recording audit events and exporting telemetry to a list
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { holder }, tables: [] }],
                follows: [],
                tables: [space.table],
            }),
        }),
        { environment: world, isMigrated: true },
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
        world,
    );
    onTestFinished(() => telemetry.shutdown());
    const traced = { ...world, telemetry };
    const spaceId = `space-${Uuid.v7(world)}`;
    const store = await TestObjectStore.at(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: traced,
        principal: spacePrincipal(spaceId),
        servers: serversOf([holder]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: holder.package },
    });
    await openSpace(storage.database, world, spaceId);

    // create a holder with a valid key, and refuse one with a key of another shape
    const valid = "sk-valid-0123456789";
    const token = "token-column-0123456789";
    const invalid = "pk-refused-0123456789";
    const create = (key: string) =>
        store
            .mutate(userContext(users.alice, spaceId, { environment: world }).access(), {
                id: RequestId.create(world),
                calls: [holder.call("create", { spaceId, label: "deploy", key, token })],
            })
            .then(([written]) => written);
    await create(valid);
    const refused = await create(invalid).then(
        () => undefined,
        (error: unknown) => error,
    );
    const refusedSystem = await store
        .mutate(AccessContext.of(Caller.of(spacePrincipal(spaceId)), world.clock.now()), {
            id: RequestId.create(world),
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

    // run the store's controllers until they settle, then flush the telemetry
    const failures: unknown[] = [];
    const stopping = new AbortController();
    const loop = new ControlLoop(store.database, store.controllers(), {
        clock: world.clock,
        report: (_controller, _key, error) => failures.push(error),
    });
    const running = loop.run(stopping.signal);
    await loop.idle();
    stopping.abort();
    await running;
    await telemetry.flush();
    expect(failures).toEqual([]);

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
        isTokenTraced: traces.rows.includes(token),
    }).toEqual({
        leaking: [],
        refusal: [
            "BAD_REQUEST",
            "INPUT_INVALID",
            "invalid input: key: invalid string: must match pattern /^sk-/u",
        ],
        systemRefusal: [
            "BAD_REQUEST",
            "INPUT_INVALID",
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
        isTokenTraced: true,
    });
});
