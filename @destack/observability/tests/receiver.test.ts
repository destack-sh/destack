import { space } from "@destack/account/object";
import { expect, refusal, test } from "@destack/test";
import { testCallKey } from "@destack/service/test";
import { accessRole, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { Journal } from "@destack/audit/server";
import { schema } from "@destack/schema";
import { setting } from "@destack/setting/object";

import { startTelemetry } from "@destack/telemetry/bun";
import { OtlpExporter } from "@destack/telemetry/otlp";
import { HOUR, log, span, telemetry, telemetryRetention } from "../src/index.ts";
import { client, eventClient, ids, notes, serveObservability } from "./fixture/observability.ts";
import { claimingLogs, logsExport, NOTES_EMITTER, NOTES_MANIFEST } from "./fixture/notes.ts";

test("query, follow, fold and trace an instance's logs, spans and metric points, stamped with its installation, instance and build, before and after they flush", async () => {
    const { server, eventServer, receive, database, failures, clock, settle } =
        await serveObservability();
    const alice = client(server, "alice");
    const reader = eventClient(eventServer, "alice");
    const selection = { scope: ids.space, object: ids.notes };

    // follow the installation's warnings before anything arrives
    const tail = await reader.tail({ ...selection, kind: "log", where: "severity >= 13" });
    const followed = (async () => {
        for await (const tailed of tail) {
            return log.parseKeys(tailed.keys).name;
        }

        return undefined;
    })();

    // export a span, a record inside it, a warning, a counter and a histogram, as the notes workload's machine relays them
    const exporter = new OtlpExporter(
        (signal, body) =>
            receive(NOTES_EMITTER, signal, JSON.parse(new TextDecoder().decode(body))),
        (error) => failures.push(error),
    );
    const sdk = await startTelemetry(exporter.options(notes));
    try {
        const instruments = sdk.scope(notes);
        await instruments.span("note.render", { blocks: 12 }, () =>
            instruments.log.info("note.saved", { length: 5 }),
        );
        instruments.log.warn("note.conflict", { attempts: 2 });
        instruments.meter.createCounter("note.saves").add(2);
        instruments.meter.createHistogram("note.render.duration", { unit: "ms" }).record(5);
        await sdk.flush();
    } finally {
        await sdk.shutdown();
    }

    // read the records newest first, the trace of the saved one, and fold the metrics
    const read = async () => {
        const page = await reader.query({ ...selection, kind: "log" });

        return page.events.map((event) => {
            const keys = log.parseKeys(event.keys);
            const data = log.parseData(event.data);

            return {
                name: keys.name,
                severity: keys.severity,
                build: keys.build,
                attributes: keys.attributes,
                instance: data.instance,
                isTraced: keys.trace !== null,
            };
        });
    };
    const before = await read();
    const saved = await reader.query({ ...selection, kind: "log", where: 'name = "note.saved"' });
    const trace = log.parseKeys(saved.events[0]?.keys).trace ?? "";
    const traced = await alice.trace({ ...selection, trace });
    const range = { ...selection, from: 0, before: (Date.now() + 1) * 1000 };
    const saves = await reader.series({
        ...range,
        kind: "metric",
        where: 'name = "note.saves"',
        measure: "value",
        fold: "sum",
        group: [],
    });
    const renders = await alice.percentiles({
        ...range,
        where: 'name = "note.render.duration"',
        fold: "p99",
        group: [],
    });

    // flush the hour's events and read the records again
    clock.now += 2 * HOUR;
    await settle();
    const after = await read();
    const hot = await database.select().from(log.table);

    // refuse a reader without a grant on the space
    const denied = await refusal(
        eventClient(eventServer, "bob").query({ ...selection, kind: "log" }),
    );

    // find the records, the trace, the folds, the followed warning, and nothing left hot
    const conflict = {
        name: "note.conflict",
        severity: 13,
        build: NOTES_MANIFEST,
        attributes: { attempts: 2 },
        instance: ids.instance,
        isTraced: false,
    };
    const record = {
        ...conflict,
        name: "note.saved",
        severity: 9,
        attributes: { length: 5 },
        isTraced: true,
    };
    expect({
        before,
        traced: {
            spans: traced.spans.map((each) => span.parseKeys(each.keys).name),
            logs: traced.logs.map((each) => log.parseKeys(each.keys).name),
        },
        saves: saves.series.map((series) => series.steps.map((step) => step.value)),
        renders: renders.series.map((series) => series.steps.map((step) => step.value)),
        followed: await followed,
        after,
        hot,
        denied,
        failures,
    }).toEqual({
        before: [conflict, record],
        traced: { spans: ["note.render"], logs: ["note.saved"] },
        saves: [[2]],
        renders: [[5]],
        followed: "note.conflict",
        after: [conflict, record],
        hot: [],
        denied: ["FORBIDDEN", "permission denied: read"],
        failures: [],
    });
});

test("refuse an export whose resource claims its own instance or build, and an installation exporting over HTTP to the service", async () => {
    const { server, receive, events } = await serveObservability();

    // claim another instance and another build in the resource of an export the machine relays
    const instance = await refusal(
        receive(NOTES_EMITTER, "logs", claimingLogs("service.instance.id", "instance-forged")),
    );
    const build = await refusal(
        receive(NOTES_EMITTER, "logs", claimingLogs("destack.build.manifest", "c".repeat(64))),
    );

    // export straight to the service as the notes installation
    const direct = await server.fetch(
        new Request("https://observability.test/v1/logs", {
            method: "POST",
            headers: { authorization: "Bearer notes", "content-type": "application/json" },
            body: JSON.stringify(claimingLogs("service.name", notes.name)),
        }),
    );

    // refuse both claims and find no route for the direct export, keeping nothing
    expect({
        instance,
        build,
        direct: direct.status,
        kept: (await events.query(log, { scope: ids.space })).events,
    }).toEqual({
        instance: [
            "FORBIDDEN",
            "only the receiver records an export's emitter, found service.instance.id",
        ],
        build: [
            "FORBIDDEN",
            "only the receiver records an export's emitter, found destack.build.manifest",
        ],
        direct: 404,
        kept: [],
    });
});

test("keep each record of an export its machine relays twice once", async () => {
    const { receive, events } = await serveObservability();
    const now = Date.now();

    // relay one export twice, as an exporter retrying a lost answer does
    const exported = logsExport([
        { time: now, name: "note.saved", severity: 9, attributes: { length: 5 } },
        { time: now, name: "note.saved", severity: 9, attributes: { length: 6 } },
    ]);
    await receive(NOTES_EMITTER, "logs", exported);
    await receive(NOTES_EMITTER, "logs", exported);

    // keep the two records once each
    const page = await events.query(log, { scope: ids.space });
    expect(page.events.map((event) => log.parseKeys(event.keys).attributes)).toEqual([
        { length: 5 },
        { length: 6 },
    ]);
});

test("seal personal attributes under their person, mask them for readers who may not unmask them, audit an unmasked read, drop those of no person, and forget them", async () => {
    const { eventServer, receive, database, events, clock, settle } = await serveObservability();
    const now = Date.now();

    // let carol read the space's telemetry without unmasking it
    await new AccessFixture(database).copyRole(
        space.reference(ids.account, ids.space),
        {
            name: "reader",
            description: "Reads telemetry.",
            permissions: [telemetry.permission("read")],
        },
        principal.user.reference("universe", "carol"),
    );
    await expect.poll(async () => (await database.select().from(accessRole)).length).toBe(2);

    // record an invitation dana sent naming an address, and a token naming no person
    await receive(
        NOTES_EMITTER,
        "logs",
        logsExport([
            {
                time: now,
                name: "user.invited",
                severity: 9,
                attributes: {
                    "enduser.id": "dana",
                    "sensitive.email": "erin@example.com",
                    role: "editor",
                },
            },
            {
                time: now + 1,
                name: "token.issued",
                severity: 9,
                attributes: { "sensitive.token": "secret-token" },
            },
        ]),
    );

    // read as carol and as alice who owns the space, and the stored rows
    const read = async (name: string) =>
        (
            await eventClient(eventServer, name).query({
                kind: "log",
                scope: ids.space,
                object: ids.notes,
                order: "ascending",
            })
        ).events.map((event) => log.parseData(event.data).personal);
    const masked = await read("carol");
    const unmasked = await read("alice");
    const stored = JSON.stringify(await database.select().from(log.table));

    // forget dana once the records flushed, and read again
    clock.now += 2 * HOUR;
    await settle();
    await events.forget("dana");
    const forgotten = await read("alice");

    // mask the address for carol, show it to alice, keep it sealed at rest, drop the token, and record only alice's read
    expect({
        masked,
        unmasked,
        isSealed: !stored.includes("erin@example.com") && !stored.includes("secret-token"),
        forgotten,
        audits: (await new Journal(database, testCallKey).read()).map(({ method, execution }) => ({
            method,
            category: execution.category,
            details: execution.details,
        })),
    }).toEqual({
        masked: [{ email: "****" }, undefined],
        unmasked: [{ email: "erin@example.com" }, undefined],
        isSealed: true,
        forgotten: [undefined, undefined],
        audits: [
            {
                method: "event.unmask",
                category: "access",
                details: { operation: "query", kind: "log" },
            },
            {
                method: "event.unmask",
                category: "access",
                details: { operation: "query", kind: "log" },
            },
        ],
    });
});

test("expire a space's telemetry past the retention its settings place", async () => {
    const { receive, database, events, clock, settle } = await serveObservability();

    // keep the space's telemetry for one day
    await database.insert(setting.table).values({
        id: schema.identifier("setting").parse("setting-01996ab0-0000-7000-8000-000000000013"),
        scope: ids.space,
        packageId: telemetryRetention.package.id,
        name: telemetryRetention.name,
        mode: "set",
        value: 1,
        release: telemetryRetention.package.version,
        createdAt: clock.now,
        updatedAt: clock.now,
    });

    // record a log, flush it after an hour, and read it after two days
    await receive(NOTES_EMITTER, "logs", logsExport([{ time: clock.now, name: "note.saved" }]));
    clock.now += 2 * HOUR;
    await settle();
    const kept = (await events.query(log, { scope: ids.space })).events.length;
    clock.now += 2 * 24 * HOUR;
    await settle();
    const expired = (await events.query(log, { scope: ids.space })).events.length;

    // keep the record a day, then expire it
    expect({ kept, expired }).toEqual({ kept: 1, expired: 0 });
});
