import { expect, onTestFinished, refusal, test } from "@destack/test";
import { testCallKey } from "@destack/service/test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Authorization, Authorizer, principal } from "@destack/access";
import { AuditRecorder, Journal } from "@destack/audit";
import { copyRole, copyScope } from "@destack/access/test";
import { account } from "@destack/account/object";
import { LocalBucket } from "@destack/bucket/local";
import { TestDatabase } from "@destack/db/test";
import { broadcastChannel, type DatabaseConnection } from "@destack/db";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { present, schema } from "@destack/schema";
import { Authentication } from "@destack/service/authentication";
import { createClient } from "@destack/service/client";
import { Health } from "@destack/service/health";
import { Server } from "@destack/service/server";
import { ServiceError } from "@destack/service/error";
import { installation, space } from "@destack/space/object";
import { spaceDatabase } from "@destack/space/stack";
import { startTelemetry } from "@destack/telemetry/host";
import { OtlpExporter } from "@destack/telemetry/otlp";
import { setting } from "@destack/setting/object";
import {
    entry,
    type Entry,
    Monitor,
    monitorDatabase,
    monitorSegment,
    telemetryRetention,
} from "../src/index.ts";
import { SegmentController } from "../src/monitor/controller.ts";
import { implementMonitor } from "../src/server/index.ts";
import { monitorService } from "../src/service/index.ts";

/** The identities the scenario names. */
const ids = {
    account: schema.identifier("account").parse("account-01996ab0-0000-7000-8000-000000000001"),
    space: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
    package: PackageId.parse("package-01996ab0-0000-7000-8000-000000000004"),
    notes: schema
        .identifier("installation")
        .parse("installation-01996ab0-0000-7000-8000-000000000005"),
};

/** The notes package, as its workload names itself. */
const notes = { id: ids.package, name: "@example/notes", version: "2026.9.0" };

/** Serve a space's monitor, where alice owns the space and notes is installed, authenticating by bearer name. */
async function serveMonitor() {
    // keep the space, owned by alice, with the notes installation
    const storage = await TestDatabase.create("sqlite", spaceDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await createSpace(database);

    // keep the segment catalog
    const catalog = await TestDatabase.create("sqlite", monitorDatabase, { isMigrated: true });
    onTestFinished(() => catalog.close());

    // store segments in a local bucket
    const directory = await mkdtemp(join(tmpdir(), "monitor-"));
    const bucket = await LocalBucket.open(directory, "space-test");
    onTestFinished(async () => {
        await bucket[Symbol.asyncDispose]();
        await rm(directory, { recursive: true, force: true });
    });
    const failures: unknown[] = [];
    const monitor = new Monitor(catalog.database, bucket, (error) => failures.push(error));

    // serve it under the space's policies and settings
    const authorizer = new Authorizer(
        [space.policy, installation.policy, entry],
        [space.mapping, installation.mapping],
    );
    const server = Server.start({
        ...implementMonitor({
            monitor,
            access: { authorizer, database },
            settings: database,
            record: AuditRecorder.service(new Journal(database, testCallKey), {
                package: monitorService.package,
                service: "test",
            }),
        }),
        audience: monitorService.package.id,
        drainTimeout: 1000,
        health: new Health("monitor"),
        resources: new ResourceContext(),
        authorizeHost: async () => {},
        authenticate: async (request) => authenticate(request),
    });
    onTestFinished(() => server.close());

    return { server, monitor, database, catalog: catalog.database, failures };
}

/** Record the space, owned by alice, with the notes installation. */
async function createSpace(database: DatabaseConnection): Promise<void> {
    // record the space in its account
    const now = Date.now();
    await copyScope(database, account.reference("universe", ids.account));
    await database.insert(space.table).values({
        id: ids.space,
        scope: ids.account,
        name: "personal",
        createdAt: now,
        updatedAt: now,
    });

    // make alice its owner
    const alice = principal.user.reference("universe", "alice");
    await new Authorization(new Authorizer([space.policy], [space.mapping]), database, () => ({
        subjects: [alice],
        now,
        attributes: {},
    })).create(space.reference(ids.account, ids.space), { owner: alice });

    // install notes
    await database.insert(installation.table).values({
        id: ids.notes,
        scope: ids.space,
        packageId: ids.package,
        role: "application",
        alias: "notes",
        selection: { kind: "release", version: "2026.9.0" },
        createdAt: now,
        updatedAt: now,
    });
}

/** Authenticate the installation and the users by the bearer name. */
function authenticate(request: Request): Authentication {
    // read the bearer name
    const header = request.headers.get("authorization");
    if (header === null) {
        throw new TypeError("a test request carries no authorization");
    }
    const name = header.slice("Bearer ".length);

    // authenticate notes as its installation and other names as users
    const subject =
        name === "notes"
            ? principal.installation.reference(ids.space, ids.notes)
            : principal.user.reference("universe", name);
    const verifiedAt = Date.now();

    return new Authentication({
        subject,
        subjects: [subject],
        credential: { kind: name === "notes" ? "installation" : "user", id: name },
        audience: monitorService.package.id,
        verifiedAt,
        expiresAt: verifiedAt + 60_000,
    });
}

/** Open a monitor client acting as a named caller. */
function client(server: Server, name: string) {
    return createClient(monitorService, {
        url: "https://monitor.test",
        headers: { authorization: `Bearer ${name}` },
        fetch: (request: Request) => server.fetch(request),
    });
}

test("search, follow and trace an installation's exported entries before and after sealing", async () => {
    const { server, monitor, catalog, failures } = await serveMonitor();
    const alice = client(server, "alice");
    const filter = { scope: ids.space, installation: ids.notes };

    // follow the installation's warnings before anything arrives
    const tail = await alice.tail({ ...filter, severity: 13 });
    const followed = (async () => {
        const received: Entry[] = [];
        for await (const tailed of tail) {
            received.push(tailed);
            break;
        }

        return received;
    })();

    // export a span, an informational record inside it and a warning, as the notes workload
    const exporter = OtlpExporter.http(
        "https://monitor.test",
        () => "Bearer notes",
        (error) => failures.push(error),
    );
    const fetch = globalThis.fetch;
    globalThis.fetch = Object.assign(
        (input: RequestInfo | URL, request?: RequestInit) =>
            server.fetch(new Request(input, request)),
        { preconnect: fetch.preconnect },
    );
    onTestFinished(() => {
        globalThis.fetch = fetch;
    });
    const telemetry = await startTelemetry(
        exporter.options(notes, { attributes: { "service.instance.id": "instance-1" } }),
    );
    try {
        const { log, span, meter } = telemetry.scope(notes);
        await span("note.render", { blocks: 12 }, () => log.info("note.saved", { length: 5 }));
        log.warn("note.conflict", { attempts: 2 });
        meter.createCounter("note.saves").add(2);
        meter.createHistogram("note.render.duration", { unit: "ms" }).record(5);
        await telemetry.flush();
    } finally {
        await telemetry.shutdown();
    }

    // search the open records, newest first, and read the trace
    const window = { ...filter, from: 0, before: Date.now() * 1000 + 1, limit: 10 };
    const open = await alice.search(window);
    const savedRecord = open.entries.find((found) => found.name === "note.saved");
    if (savedRecord?.trace === undefined) {
        throw new TypeError("the open records have no traced note.saved");
    }
    const { trace } = savedRecord;
    const traced = await alice.trace({ ...filter, trace });

    // chart the counter and the histogram over the window
    const metric = { ...filter, from: 0, before: window.before, step: 60_000_000, group: [] };
    const saves = await alice.series({ ...metric, name: "note.saves" });
    const renders = await alice.series({ ...metric, name: "note.render.duration" });

    // seal the segment and search the stored records the same way
    await monitor.seal(Date.now(), true);
    const sealed = await alice.search(window);
    const segments = await catalog.select({ rows: monitorSegment.rows }).from(monitorSegment);

    // refuse a reader without the space's grant
    const denied = await refusal(client(server, "bob").search(window));

    // find both records, the trace's span and record, the followed warning, the same page sealed
    const anyTime: unknown = expect.any(Number);
    const anySpan: unknown = expect.stringMatching(/^[0-9a-f]{16}$/u);
    const base = {
        installation: ids.notes,
        instance: "instance-1",
        source: { name: "@example/notes", version: "2026.9.0" },
        time: anyTime,
    };
    const saved = {
        ...base,
        kind: "log",
        name: "note.saved",
        trace,
        span: anySpan,
        status: "unset",
        severity: 9,
        attributes: { length: 5 },
    };
    const conflict = {
        ...base,
        kind: "log",
        name: "note.conflict",
        status: "unset",
        severity: 13,
        attributes: { attempts: 2 },
    };
    expect({
        open: open.entries,
        traced: traced.entries.map((found) => [found.kind, found.name]),
        followed: await followed,
        sealed: sealed.entries,
        saves,
        renders,
        catalog: segments,
        denied,
        failures,
    }).toEqual({
        open: [conflict, saved],
        traced: [
            ["span", "note.render"],
            ["log", "note.saved"],
        ],
        followed: [conflict],
        sealed: [conflict, saved],
        saves: { series: [{ attributes: {}, steps: [{ time: anyTime, value: 2 }] }] },
        renders: {
            series: [
                {
                    attributes: {},
                    steps: [{ time: anyTime, count: 1, sum: 5, p50: 5, p90: 5, p99: 5 }],
                },
            ],
        },
        catalog: [{ rows: 5 }],
        denied: ["FORBIDDEN", "permission denied: read-logs"],
        failures: [],
    });
});

test("compact an hour's minute segments into one, prune them past retention, and sweep their files", async () => {
    const { monitor, catalog } = await serveMonitor();
    const now = Date.now();
    const hour = Math.floor((now - 3 * 24 * 60 * 60 * 1000) / 3_600_000) * 3_600_000;

    // seal two minute segments of one hour three days ago
    const record = (name: string, minute: number): Entry => ({
        kind: "log",
        name,
        time: (hour + minute * 60_000) * 1000,
        installation: ids.notes,
        source: { name: "@example/notes", version: "2026.9.0" },
        status: "unset",
        severity: 9,
        attributes: {},
    });
    monitor.ingest(ids.space, [record("note.saved", 1)]);
    await monitor.seal(now, true);
    monitor.ingest(ids.space, [record("note.opened", 2)]);
    await monitor.seal(now, true);

    // merge them into one hourly segment and find both entries in it
    await monitor.compact(ids.space, ids.notes, now);
    const compacted = await catalog
        .select({ level: monitorSegment.level, rows: monitorSegment.rows })
        .from(monitorSegment);
    const search = {
        scope: ids.space,
        installation: ids.notes,
        from: 0,
        before: now * 1000,
        limit: 10,
    };
    const found = (await monitor.search(ids.space, search)).entries.map((each) => each.name);

    // keep a day, drop the segment, and sweep every file once the grace passed
    await monitor.prune(ids.space, ids.notes, 1, now);
    const pruned = await catalog.select().from(monitorSegment);
    await monitor.sweep(ids.space, ids.notes, now + 60 * 60 * 1000);
    const files = await monitor.bucket.list({ prefix: `${ids.space}/` });

    expect({ compacted, found, pruned, files: files.files }).toEqual({
        compacted: [{ level: 1, rows: 2 }],
        found: ["note.opened", "note.saved"],
        pruned: [],
        files: [],
    });
});

test("prune an installation's segments past the retention its space's settings place", async () => {
    const { monitor, database, catalog } = await serveMonitor();
    const now = Date.now();
    const controller = new SegmentController(monitor, database);

    // seal one entry of three days ago
    monitor.ingest(ids.space, [
        {
            kind: "log",
            name: "note.saved",
            time: (now - 3 * 24 * 60 * 60 * 1000) * 1000,
            installation: ids.notes,
            source: { name: "@example/notes", version: "2026.9.0" },
            status: "unset",
            severity: 9,
            attributes: {},
        },
    ]);
    await monitor.seal(now, true);
    const [key] = await controller.list();

    // keep it under the default retention of thirty days
    await controller.reconcile(present(key, "the notes installation's key"));
    const kept = await catalog.select({ rows: monitorSegment.rows }).from(monitorSegment);

    // drop it once the space keeps a day
    await database.insert(setting.table).values({
        id: schema.identifier("setting").parse("setting-01996ab0-0000-7000-8000-000000000013"),
        scope: ids.space,
        packageId: telemetryRetention.package.id,
        name: telemetryRetention.name,
        mode: "set",
        value: 1,
        release: telemetryRetention.package.version,
        createdAt: now,
        updatedAt: now,
    });
    await new SegmentController(monitor, database).reconcile(
        present(key, "the notes installation's key"),
    );
    const pruned = await catalog.select().from(monitorSegment);

    expect({ kept, pruned }).toEqual({ kept: [{ rows: 1 }], pruned: [] });
});

test("mask sensitive values for readers who may not unmask them, and audit an unmasked read", async () => {
    const { server, monitor, database } = await serveMonitor();
    const now = Date.now();

    // let carol read the space's logs without unmasking them
    await copyRole(
        database,
        space.reference(ids.account, ids.space),
        {
            name: "reader",
            description: "Reads logs.",
            permissions: [entry.permission("read-logs")],
        },
        principal.user.reference("universe", "carol"),
    );

    // record an invitation naming an address
    monitor.ingest(ids.space, [
        {
            kind: "log",
            name: "user.invited",
            time: now * 1000,
            installation: ids.notes,
            source: { name: "@example/notes", version: "2026.9.0" },
            status: "unset",
            severity: 9,
            attributes: { "sensitive.email": "dana@example.com", role: "editor" },
        },
    ]);

    // search as carol and as alice who owns the space
    const search = {
        scope: ids.space,
        installation: ids.notes,
        from: 0,
        before: now * 1000 + 1,
        limit: 10,
    };
    const masked = await client(server, "carol").search(search);
    const unmasked = await client(server, "alice").search(search);

    // mask the address for carol, show it to alice, and record only alice's read
    expect({
        masked: masked.entries.map((found) => found.attributes),
        unmasked: unmasked.entries.map((found) => found.attributes),
        audits: (await new Journal(database, testCallKey).read()).map(({ method, execution }) => ({
            scope: execution.context.scope,
            method,
            category: execution.category,
            targets: execution.targets,
            details: execution.details,
        })),
    }).toEqual({
        masked: [{ "sensitive.email": "****", role: "editor" }],
        unmasked: [{ "sensitive.email": "dana@example.com", role: "editor" }],
        audits: [
            {
                scope: ids.space,
                method: "monitor.unmask",
                category: "access",
                targets: { emitter: { type: "installation", id: ids.notes } },
                details: { operation: "search" },
            },
        ],
    });
});

test("follow and search the entries another instance ingests, through the channel between their monitors", async () => {
    // run two monitors over one database, each reaching the other over its own end of a channel
    const storage = await TestDatabase.create("sqlite", monitorDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const directory = await mkdtemp(join(tmpdir(), "monitor-"));
    const bucket = await LocalBucket.open(directory, "space-test");
    onTestFinished(async () => {
        await bucket[Symbol.asyncDispose]();
        await rm(directory, { recursive: true, force: true });
    });
    const topic = `monitor-${crypto.randomUUID()}`;
    const failures: unknown[] = [];
    const open = (): Monitor =>
        new Monitor(
            storage.database,
            bucket,
            (error) => failures.push(error),
            broadcastChannel(topic),
        );
    const [first, second] = [open(), open()];
    const running = new AbortController();
    const runs = [first.run(running.signal), second.run(running.signal)];
    onTestFinished(async () => {
        running.abort();
        await Promise.all(runs);
    });

    // tail on the second instance once the instances know each other
    const now = Date.now();
    const record = (name: string): Entry => ({
        kind: "log",
        name,
        time: now * 1000,
        installation: ids.notes,
        source: { name: "@example/notes", version: "2026.9.0" },
        status: "unset",
        severity: 9,
        attributes: {},
    });
    await expect.poll(async () => (await second.search(ids.space, window())).entries).toEqual([]);
    const tailing = new AbortController();
    const tail = second.tail({ scope: ids.space, installation: ids.notes }, tailing.signal);
    await new Promise((resolve) => {
        setTimeout(resolve, 50);
    });

    // ingest on the first instance, and see it on the second before any seal
    first.ingest(ids.space, [record("note.saved")]);
    const followed = (await tail.next()).value;
    tailing.abort();
    expect({
        followed,
        searched: (await second.search(ids.space, window())).entries,
        failures,
    }).toEqual({ followed: record("note.saved"), searched: [record("note.saved")], failures: [] });

    /** Search the notes installation's records of the last minute. */
    function window() {
        return {
            scope: ids.space,
            installation: ids.notes,
            from: (now - 60_000) * 1000,
            before: (now + 60_000) * 1000,
            limit: 10,
        };
    }
});

test("search sealed log records by text and attributes, keeping bodies and records without a severity", async () => {
    // keep one monitor over a database and a bucket
    const storage = await TestDatabase.create("sqlite", monitorDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const directory = await mkdtemp(join(tmpdir(), "monitor-"));
    const bucket = await LocalBucket.open(directory, "space-test");
    onTestFinished(async () => {
        await bucket[Symbol.asyncDispose]();
        await rm(directory, { recursive: true, force: true });
    });
    const failures: unknown[] = [];
    const monitor = new Monitor(storage.database, bucket, (error) => failures.push(error));

    // receive two records over OTLP, one without a severity, and seal them
    const now = Date.now();
    const record = (event: string, body: string, region: string, severityNumber?: number) => ({
        timeUnixNano: String(now * 1_000_000),
        eventName: event,
        body: { stringValue: body },
        ...(severityNumber === undefined ? {} : { severityNumber }),
        attributes: [
            { key: "region", value: { stringValue: region } },
            { key: "empty", value: {} },
        ],
    });
    monitor.receive(ids.space, ids.notes, "logs", {
        resourceLogs: [
            {
                resource: { attributes: [] },
                scopeLogs: [
                    {
                        scope: { name: "@example/notes", version: "2026.9.0" },
                        logRecords: [
                            record("note.saved", "Saved the Plan note", "eu"),
                            record("note.failed", "Disk quota exceeded", "us", 17),
                        ],
                    },
                ],
            },
        ],
    });
    await monitor.seal(now, true);

    // find the record without a severity and filter by text, by attribute and by severity
    const search = (filter: object) =>
        monitor
            .search(ids.space, {
                scope: ids.space,
                installation: ids.notes,
                from: (now - 60_000) * 1000,
                before: (now + 60_000) * 1000,
                limit: 10,
                ...filter,
            })
            .then((page) =>
                page.entries.map((found) => [found.name, found.body, found.attributes]),
            );
    expect({
        every: await search({}),
        text: await search({ text: "plan" }),
        attribute: await search({ attributes: { region: "us" } }),
        severe: await search({ severity: 13 }),
        failures,
    }).toEqual({
        every: [
            ["note.failed", "Disk quota exceeded", { region: "us" }],
            ["note.saved", "Saved the Plan note", { region: "eu" }],
        ],
        text: [["note.saved", "Saved the Plan note", { region: "eu" }]],
        attribute: [["note.failed", "Disk quota exceeded", { region: "us" }]],
        severe: [["note.failed", "Disk quota exceeded", { region: "us" }]],
        failures: [],
    });

    // refuse a metric point that carries no value
    expect(() =>
        monitor.receive(ids.space, ids.notes, "metrics", {
            resourceMetrics: [
                {
                    resource: { attributes: [] },
                    scopeMetrics: [
                        {
                            scope: { name: "@example/notes", version: "2026.9.0" },
                            metrics: [
                                {
                                    name: "notes.saved",
                                    gauge: {
                                        dataPoints: [
                                            {
                                                timeUnixNano: String(now * 1_000_000),
                                                attributes: [],
                                            },
                                        ],
                                    },
                                },
                            ],
                        },
                    ],
                },
            ],
        }),
    ).toThrow(
        new ServiceError("BAD_REQUEST", {
            message: "a point of metric notes.saved carries no value",
        }),
    );
});
