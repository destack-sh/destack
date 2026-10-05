import { expect, onTestFinished, refusal, test } from "@destack/test";
import { testCallKey } from "@destack/service/test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { accessRole, Authorization, Authorizer, principal } from "@destack/access";
import { Journal } from "@destack/audit/server";
import { AccessFixture } from "@destack/access/test";
import { LocalBucket } from "@destack/bucket/local";
import { TestDatabase } from "@destack/db/test";
import {
    and,
    broadcastChannel,
    type DatabaseConnection,
    defineDatabase,
    eq,
    Snapshot,
} from "@destack/db";
import { activity, announcement, subscription } from "@destack/notification";
import type { ObjectServer } from "@destack/object/server";
import { comment, reaction } from "@destack/social";
import { spaceCopies, spaceTables } from "@destack/space/stack";
import { alert, alertRule, type AlertRuleDefinition, issue } from "../src/object/index.ts";
import { Scope } from "@destack/sync";
import { graph, PackageId } from "@destack/package";
import { PackageFile } from "@destack/package/file";
import { BuildReader, PackageManifest } from "@destack/package/manifest";
import { ResourceContext } from "@destack/resource/context";
import { Digest, present, schema } from "@destack/schema";
import { Authentication } from "@destack/service/authentication";
import { createClient } from "@destack/service/client";
import { RequestId } from "@destack/service/request";
import { Health } from "@destack/service/health";
import { Server } from "@destack/service/server";
import { type Controller, ControlLoop } from "@destack/service/control";
import { ServiceError } from "@destack/service/error";
import { installation, installationRevision, space, SpaceCell } from "@destack/space/object";
import type { OpenBuild } from "@destack/space/server";
import { serveObjects } from "@destack/space/server";
import { openBuild, SpaceFixture } from "@destack/space/test";
import { startTelemetry } from "@destack/telemetry/bun";
import { OtlpExporter } from "@destack/telemetry/otlp";
import { setting } from "@destack/setting/object";
import { entry, type Entry, Monitor, telemetryRetention } from "../src/index.ts";
import { SegmentController } from "../src/monitor/controller.ts";
import { implementMonitor } from "../src/server/index.ts";
import { monitorService } from "../src/service/index.ts";
import { monitorDatabase, monitorSegment } from "../src/stack/index.ts";

/** A cell's space database copying the issues, alert rules and alerts the monitor keeps, with the comments, subscriptions and notifications on them. */
const cellDatabase = defineDatabase({
    name: "space",
    tables: spaceTables,
    copies: [
        ...spaceCopies,
        ...[issue, alertRule, alert, comment, reaction, subscription, activity, announcement].map(
            (object) => object.table,
        ),
    ],
});

/** Run some controllers over a database until the test ends, failing it on a reported failure. */
function run(database: DatabaseConnection, controllers: readonly Controller[]): void {
    const stopping = new AbortController();
    const running = new ControlLoop(database, controllers, {
        report: (_controller, _key, error) => {
            throw error;
        },
    }).run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });
}

/** The identities the scenario names. */
const ids = {
    account: schema.identifier("account").parse("account-01996ab0-0000-7000-8000-000000000001"),
    space: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
    package: PackageId.parse("package-01996ab0-0000-7000-8000-000000000004"),
    notes: schema
        .identifier("installation")
        .parse("installation-01996ab0-0000-7000-8000-000000000005"),
    region: schema.identifier("region").parse("region-01996ab0-0000-7000-8000-000000000003"),
};

/** The notes package, as its workload names itself. */
const notes = { id: ids.package, name: "@example/notes", version: "2026.9.0" };

/** How a test serves the monitor: the builds it opens. */
interface MonitorTestOptions {
    /** Open a package's build in the space, the space fixture's by default. */
    readonly openBuild?: OpenBuild;
}

/** Serve a space's monitor, where alice owns the space and notes is installed, authenticating by bearer name. */
async function serveMonitor(options: MonitorTestOptions = {}) {
    // keep the space, owned by alice, with the notes installation, copying the issues, rules and alerts the monitor keeps
    const fixture = await SpaceFixture.open({ database: cellDatabase });
    const database = fixture.database;
    await createSpace(database);

    // keep the segment catalog
    const catalog = await TestDatabase.create("sqlite", monitorDatabase, {
        isMigrated: true,
    });
    onTestFinished(() => catalog.close());

    // store segments in a local bucket
    const directory = await mkdtemp(join(tmpdir(), "monitor-"));
    const bucket = await LocalBucket.open(directory, "space-test");
    onTestFinished(async () => {
        await bucket[Symbol.asyncDispose]();
        await rm(directory, { recursive: true, force: true });
    });
    const failures: unknown[] = [];
    const monitor = await Monitor.open(catalog.database, bucket, (error) => failures.push(error));
    onTestFinished(() => monitor.close());

    // serve it following the space's rows, and the space copying its objects back from it
    let spaces: ObjectServer | undefined;
    const implementation = implementMonitor({
        monitor,
        callKey: testCallKey,
        cell: SpaceCell.id({ regionId: ids.region }),
        scopes: [space],
        uplink: {
            stream: (followed, signal) =>
                present(spaces, "the space service").uplink.stream(followed, signal),
            receive: (mutation) => present(spaces, "the space service").uplink.receive(mutation),
        },
        openBuild: options.openBuild ?? openBuild,
        report: (error) => failures.push(error),
    });
    spaces = serveObjects(await fixture.options({ sources: [implementation.objects] }));
    const server = Server.start({
        ...implementation,
        audience: monitorService.package.id,
        drainTimeout: 1000,
        health: new Health("monitor"),
        resources: new ResourceContext(),
        authorizeHost: async () => {},
        authenticate: async (request) => authenticate(request),
    });
    onTestFinished(() => server.close());

    // copy the monitor's objects into the space, stopping before the server drains
    run(
        database,
        spaces.controllers().filter((controller) => ["sends", "replica"].includes(controller.name)),
    );

    // wait for the monitor's copies of the space's scope chain and its installation
    await expect
        .poll(async () => (await catalog.database.select().from(installation.table)).length)
        .toBe(1);
    await expect
        .poll(async () =>
            (await Scope.chain(Snapshot.live(catalog.database), ids.space)).map(
                (link) => link.object.id,
            ),
        )
        .toEqual([ids.space, ids.account, "universe"]);

    return { server, monitor, spaces, database, catalog: catalog.database, failures };
}

/** Make alice an owner of the fixture's space and install notes in it. */
async function createSpace(database: DatabaseConnection): Promise<void> {
    // bind the space's owner role to alice as the fixture's owner
    const now = Date.now();
    const [role] = await database
        .select({ id: accessRole.id })
        .from(accessRole)
        .where(and(eq(accessRole.scope, ids.space), eq(accessRole.name, "owner")));
    await new Authorization(new Authorizer([space.policy], [space.mapping]), database, () => ({
        subjects: [principal.user.reference("universe", "owner")],
        now,
        attributes: {},
    })).grant({
        object: space.reference(ids.account, ids.space),
        role: present(role, "the space's owner role").id,
        subject: principal.user.reference("universe", "alice"),
    });

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
        exporter.options(notes, {
            attributes: { "service.instance.id": "instance-1" },
        }),
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
    const window = {
        ...filter,
        from: 0,
        before: Date.now() * 1000 + 1,
        limit: 10,
    };
    const open = await alice.search(window);
    const savedRecord = open.entries.find((found) => found.name === "note.saved");
    if (savedRecord?.trace === undefined) {
        throw new TypeError("the open records have no traced note.saved");
    }
    const { trace } = savedRecord;
    const traced = await alice.trace({ ...filter, trace });

    // chart the counter and the histogram over the window
    const metric = {
        ...filter,
        from: 0,
        before: window.before,
        step: 60_000_000,
        group: [],
    };
    const saves = await alice.series({ ...metric, name: "note.saves" });
    const renders = await alice.series({
        ...metric,
        name: "note.render.duration",
    });

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
        saves: {
            series: [{ attributes: {}, steps: [{ time: anyTime, value: 2 }] }],
        },
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
    const { server, monitor, database, catalog } = await serveMonitor();
    const now = Date.now();

    // let carol read the space's logs without unmasking them
    await new AccessFixture(database).copyRole(
        space.reference(ids.account, ids.space),
        {
            name: "reader",
            description: "Reads logs.",
            permissions: [entry.permission("read-logs")],
        },
        principal.user.reference("universe", "carol"),
    );
    await expect.poll(async () => (await catalog.select().from(accessRole)).length).toBe(2);

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
        audits: (await new Journal(catalog, testCallKey).read()).map(({ method, execution }) => ({
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
    const storage = await TestDatabase.create("sqlite", monitorDatabase, {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const directory = await mkdtemp(join(tmpdir(), "monitor-"));
    const bucket = await LocalBucket.open(directory, "space-test");
    onTestFinished(async () => {
        await bucket[Symbol.asyncDispose]();
        await rm(directory, { recursive: true, force: true });
    });
    const topic = `monitor-${crypto.randomUUID()}`;
    const failures: unknown[] = [];
    const open = (): Promise<Monitor> =>
        Monitor.open(
            storage.database,
            bucket,
            (error) => failures.push(error),
            broadcastChannel(topic),
        );
    const [first, second] = [await open(), await open()];
    onTestFinished(async () => {
        await Promise.all([first.close(), second.close()]);
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
    }).toEqual({
        followed: record("note.saved"),
        searched: [record("note.saved")],
        failures: [],
    });

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
    const storage = await TestDatabase.create("sqlite", monitorDatabase, {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const directory = await mkdtemp(join(tmpdir(), "monitor-"));
    const bucket = await LocalBucket.open(directory, "space-test");
    onTestFinished(async () => {
        await bucket[Symbol.asyncDispose]();
        await rm(directory, { recursive: true, force: true });
    });
    const failures: unknown[] = [];
    const monitor = await Monitor.open(storage.database, bucket, (error) => failures.push(error));
    onTestFinished(() => monitor.close());

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

/** The note module's source: a class whose rename method fails, and the module's path. */
const NOTE_SOURCE = [
    "export class Note {",
    "    rename(title: string) {",
    "        return check(title);",
    "    }",
    "}",
].join("\n");

/** The digest of the notes build's manifest, as its workload's resource names it. */
const NOTES_MANIFEST = "b".repeat(64);

/** Encode text as UTF-8. */
function encode(value: string): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(value);
}

/** Open the notes build: a workload whose first generated line maps into `Note.rename`, an object method declaration, and the alert rules it declares. */
async function openNotesBuild(
    rules: Readonly<Record<string, AlertRuleDefinition>> = {},
): Promise<BuildReader> {
    // keep the source, the workload's source map and the note module's graph file
    const files = new Map<string, Uint8Array<ArrayBuffer>>();
    files.set("src/note.ts", encode(NOTE_SOURCE));
    files.set(
        "output/bun/workload.js.map",
        encode(
            JSON.stringify({
                version: 3,
                sources: ["../../src/note.ts"],
                names: ["rename"],
                // generated 1:0 → note.ts 3:8, inside rename
                mappings: "AAEQA",
            }),
        ),
    );
    const at = (name: string) =>
        graph.Moniker.of({ packageId: ids.package, module: "src/note.ts", name });
    const lineAt = (line: number) =>
        NOTE_SOURCE.split("\n")
            .slice(0, line - 1)
            .reduce((offset, text) => offset + text.length + 1, 0);
    const module = await graph.Module.file({
        path: "src/note.ts",
        digest: await Digest.of(encode(NOTE_SOURCE)),
        imports: [],
        exports: [],
        symbols: [
            {
                moniker: at("Note"),
                kind: "class",
                source: { file: "src/note.ts", start: 0, end: NOTE_SOURCE.length },
                signature: "class Note",
                isExported: true,
            },
            {
                moniker: at("Note.rename"),
                kind: "method",
                source: { file: "src/note.ts", start: lineAt(2), end: lineAt(5) - 1 },
                signature: "rename(title: string)",
                isExported: true,
            },
            ...Object.keys(rules).map((name) => ({
                moniker: at(name),
                kind: "variable" as const,
                source: { file: "src/note.ts", start: 0, end: 0 },
                signature: `const ${name}`,
                isExported: true,
            })),
        ],
        declarations: [
            {
                moniker: `${at("Note.rename")}:method`,
                symbol: at("Note"),
                kind: "method",
                package: ids.package,
                name: "rename",
                description: {},
            },
            ...Object.entries(rules).map(([name, rule]) => ({
                moniker: `${at(name)}:alert-rule`,
                symbol: at(name),
                kind: "alert-rule",
                package: alertRule.package.id,
                name,
                description: rule,
            })),
        ],
        edges: [],
    });
    files.set(`graph/${module.digest}.json`, module.bytes);

    // list them in the manifest
    const listed = async (path: string, value: unknown) => {
        const bytes = encode(JSON.stringify(value));
        files.set(path, bytes);

        return PackageFile.describe(path, "application/json", bytes);
    };
    const manifest = PackageManifest.parse({
        formatVersion: 1,
        package: notes,
        language: "typescript",
        lists: {
            dependencies: await listed("manifest/dependencies.json", {}),
            files: await listed("manifest/files.json", []),
            sourceMaps: await listed("manifest/sourceMaps.json", [
                {
                    generated: "output/bun/workload.js",
                    map: "output/bun/workload.js.map",
                },
            ]),
            graph: await listed("manifest/graph.json", {
                modules: { "src/note.ts": module.digest },
            }),
        },
        outputs: {},
    });

    return new BuildReader(manifest, async (path) =>
        present(files.get(path), `the notes build's file ${path}`),
    );
}

/** Record the notes revision built from the notes manifest at a time, returning its identifier. */
async function recordNotesRevision(
    database: DatabaseConnection,
    now = Date.now(),
    manifest: string = NOTES_MANIFEST,
) {
    const id = schema.identifier("installation-revision").parse(installationRevision.generateId());
    await database.insert(installationRevision.table).values({
        id,
        scope: ids.space,
        installationId: ids.notes,
        build: {
            kind: "release",
            version: notes.version,
            manifest,
        },
        views: {},
        settings: [],
        digest: String(now).padStart(64, "c"),
        createdAt: now,
        updatedAt: now,
    });

    return id;
}

/** Export log records as the notes workload running the notes build. */
async function exportLogs(
    server: Server,
    records: readonly {
        readonly time: number;
        readonly attributes: Readonly<Record<string, string | boolean>>;
    }[],
): Promise<void> {
    const response = await server.fetch(
        new Request("https://monitor.test/v1/logs", {
            method: "POST",
            headers: {
                authorization: "Bearer notes",
                "content-type": "application/json",
            },
            body: JSON.stringify({
                resourceLogs: [
                    {
                        resource: {
                            attributes: [
                                { key: "service.name", value: { stringValue: notes.name } },
                                {
                                    key: "destack.build.manifest",
                                    value: { stringValue: NOTES_MANIFEST },
                                },
                            ],
                        },
                        scopeLogs: [
                            {
                                scope: { name: notes.name, version: notes.version },
                                logRecords: records.map((record) => ({
                                    timeUnixNano: String(record.time * 1_000_000),
                                    severityNumber: 17,
                                    eventName: "exception",
                                    attributes: Object.entries(record.attributes).map(
                                        ([key, value]) => ({
                                            key,
                                            value:
                                                typeof value === "string"
                                                    ? { stringValue: value }
                                                    : { boolValue: value },
                                        }),
                                    ),
                                })),
                            },
                        ],
                    },
                ],
            }),
        }),
    );
    expect([response.status, await response.json()]).toEqual([200, {}]);
}

/** A failure of `Note.rename` as the notes workload captures it, for a person. */
function renameFailure(time: number, person: string) {
    return {
        time,
        attributes: {
            "error.type": "RangeError",
            "exception.type": "RangeError",
            "exception.message": "title is too long\nat most 500 characters",
            "exception.stacktrace":
                "RangeError: title is too long\n    at rename (file:///srv/output/bun/workload.js:1:1)",
            "exception.escaped": true,
            "enduser.id": person,
        },
    };
}

test("group failures into an issue at the declaration they ran, fire the rule watching new issues, and regress the resolved issue", async () => {
    // serve the monitor with the notes build
    const notesBuild = await openNotesBuild();
    const { server, database, catalog, failures } = await serveMonitor({
        openBuild: async () => notesBuild,
    });
    const revision = await recordNotesRevision(database);
    await expect
        .poll(async () => (await catalog.select().from(installationRevision.table)).length)
        .toBe(1);
    const alice = client(server, "alice");

    // watch new issues, notifying the rule's subscribers
    const rule = await alice.alertRule.create({
        spaceId: ids.space,
        requestId: RequestId.create(),
        name: "New issues",
        condition: { kind: "issue", on: "open" },
        actions: [{ kind: "notify" }],
    });

    // fail twice for dana and once for erin, and refuse a missing note, which is a declared outcome
    const now = Date.now();
    await exportLogs(server, [
        renameFailure(now, "dana"),
        renameFailure(now + 1, "dana"),
        renameFailure(now + 2, "erin"),
        {
            time: now + 3,
            attributes: {
                "error.type": "NOT_FOUND",
                "exception.type": "ServiceError",
                "exception.message": "no note",
                "exception.escaped": true,
            },
        },
    ]);

    // group the rename failures into one issue at the method's declaration, counting the people and the revision
    const issues = async () => (await alice.issue.list({ spaceId: ids.space })).items;
    await expect
        .poll(async () =>
            (await issues()).map((found) => ({
                title: found.title,
                errorType: found.errorType,
                culprit: found.culprit,
                declaration: found.declaration,
                level: found.level,
                status: found.status,
                count: found.count,
                people: found.people,
                firstRevision: found.firstRevisionId,
                lastRevision: found.lastRevisionId,
            })),
        )
        .toEqual([
            {
                title: "RangeError: title is too long",
                errorType: "RangeError",
                culprit: `${ids.package}/src/note.ts#Note.rename`,
                declaration: `${ids.package}/src/note.ts#Note.rename:method`,
                level: "error",
                status: "unresolved",
                count: 3,
                people: 2,
                firstRevision: revision,
                lastRevision: revision,
            },
        ]);
    const [opened] = await issues();

    // copy the issue back into the space, where views read it
    await expect
        .poll(
            async () =>
                (await database.select({ id: issue.table.id }).from(issue.table)).map(
                    ({ id }) => id,
                ),
            { timeout: 10_000 },
        )
        .toEqual([opened?.id]);

    // fire the rule once for the new issue, settled at once
    const alerts = async () => (await alice.alert.list({ spaceId: ids.space })).items;
    await expect.poll(async () => (await alerts()).length).toBe(1);
    expect(
        (await alerts()).map(({ ruleId, issueId, title, status }) => ({
            ruleId,
            issueId,
            title,
            status,
        })),
    ).toEqual([
        {
            ruleId: rule.id,
            issueId: opened?.id,
            title: "RangeError: title is too long",
            status: "resolved",
        },
    ]);

    // regress the issue once it fails again after alice resolves it
    await alice.issue.resolve({
        spaceId: ids.space,
        id: present(opened, "the issue").id,
        requestId: RequestId.create(),
    });
    await exportLogs(server, [renameFailure(Date.now(), "dana")]);
    await expect
        .poll(async () => (await issues()).map(({ status, count }) => ({ status, count })))
        .toEqual([{ status: "regressed", count: 4 }]);
    expect(failures).toEqual([]);
});

test("roll an installation back through installation.rollBack when a rule watching new issues fires", async () => {
    // serve the monitor with notes applied at its second revision
    const notesBuild = await openNotesBuild();
    const { server, database, catalog, failures } = await serveMonitor({
        openBuild: async () => notesBuild,
    });
    const now = Date.now();
    const previous = await recordNotesRevision(database, now - 60_000);
    const current = await recordNotesRevision(database, now);
    await database
        .update(installation.table)
        .set({ revisionId: current, appliedRevisionId: current })
        .where(eq(installation.table.id, ids.notes));
    await expect
        .poll(async () =>
            (await catalog.select().from(installation.table)).map(
                ({ appliedRevisionId }) => appliedRevisionId,
            ),
        )
        .toEqual([current]);
    const alice = client(server, "alice");

    // roll back on every new issue
    await alice.alertRule.create({
        spaceId: ids.space,
        requestId: RequestId.create(),
        name: "Roll back new issues",
        condition: { kind: "issue", on: "open" },
        actions: [{ kind: "call", method: "rollBack" }],
    });
    await exportLogs(server, [renameFailure(now, "dana")]);

    // follow the previous revision in the space
    await expect
        .poll(
            async () =>
                (
                    await database
                        .select({ revisionId: installation.table.revisionId })
                        .from(installation.table)
                        .where(eq(installation.table.id, ids.notes))
                ).map(({ revisionId }) => revisionId),
            { timeout: 10_000 },
        )
        .toEqual([previous]);
    expect(failures).toEqual([]);
});

test("keep the alert rules an installation's applied revision declares, and retire one a later revision drops", async () => {
    // serve the monitor with a notes revision declaring a rule, and a later one declaring none
    const rollback: AlertRuleDefinition = {
        name: "Roll back new fatal issues",
        condition: { kind: "issue", on: "open", filter: "level = fatal" },
        actions: [{ kind: "notify" }, { kind: "call", method: "rollBack" }],
    };
    const builds = new Map([
        ["c".repeat(64), await openNotesBuild({ rollback })],
        ["d".repeat(64), await openNotesBuild()],
    ]);
    const { database, catalog } = await serveMonitor({
        openBuild: async (_packageId, build) =>
            present(builds.get(build.manifest ?? ""), "the notes build of the revision"),
    });
    const revision = async (manifest: string, createdAt: number) => {
        const id = await recordNotesRevision(database, createdAt, manifest);
        await database
            .update(installation.table)
            .set({ revisionId: id, appliedRevisionId: id })
            .where(eq(installation.table.id, ids.notes));
    };

    // keep the declared rule, managed by the notes installation
    const kept = async () =>
        (await catalog.select().from(alertRule.table)).map((row) => ({
            name: row.name,
            manager: row.managerInstallationId,
            declaration: row.managerName,
            isRetiring: row.deletionRequestedAt !== null,
        }));
    await revision("c".repeat(64), Date.now() - 60_000);
    await expect
        .poll(kept, { timeout: 4000 })
        .toEqual([
            {
                name: "Roll back new fatal issues",
                manager: ids.notes,
                declaration: "rollback",
                isRetiring: false,
            },
        ]);

    // retire it once a later revision declares none
    await revision("d".repeat(64), Date.now());
    await expect
        .poll(async () => (await kept()).every((rule) => rule.isRetiring))
        .toBe(true);
});
