import type { EventFilter } from "@destack/event";
import { Digest, present, schema } from "@destack/schema";
import stackPackage from "@destack/stack/package";
import { ids } from "@destack/stack/test";
import { RequestId } from "@destack/sync";
import {
    ACTION_EVENT,
    ANALYTICS_ATTRIBUTES,
    SESSION_ATTRIBUTE,
    SESSION_EVENTS,
    SESSION_STATUS_ATTRIBUTE,
    VISIT_EVENT,
} from "@destack/telemetry/otlp";
import { expect, refusal, test } from "@destack/test";
import {
    action,
    issue,
    issuePerson,
    log,
    session,
    span,
    type Until,
    visit,
} from "../object/index.ts";
import { visitorSalt } from "../object/index.ts";
import observabilityPackage from "../package.ts";
import { TestObservability } from "../test/index.ts";
import { EMITTER_ATTRIBUTES } from "./otlp.ts";

/** The hexadecimal digits of a visitor's hash. */
const VISITOR_DIGITS = 32;

/** The milliseconds in a minute. */
const MINUTE = 60_000;

/** The browser a page's export came from, as its machine saw it. */
const BROWSER = {
    [EMITTER_ATTRIBUTES.address]: "203.0.113.7",
    [EMITTER_ATTRIBUTES.userAgent]: "Mozilla/5.0 (Macintosh)",
};

test("append an export through the otlp adapter as the machine forwarding it, and conceal the append from a member not serving the space", async () => {
    await using kit = await TestObservability.open();
    const now = kit.clock.now();

    // forward two records as the machine, and one as the member, who serves nothing
    const forwarded = await kit.post(
        "logs",
        kit.emitter.logs([
            { time: now, name: "note.saved", severity: 9 },
            { time: now + 1, name: "note.conflict", severity: 13 },
        ]),
    );
    const refused = await refusal(
        kit.post(
            "logs",
            kit.emitter.logs([{ time: now, name: "note.forged", severity: 9 }]),
            kit.member,
        ),
    );

    // keep the machine's records alone, stamped with the forwarder's installation and build
    const kept = await kit.served.store.events.query(log.eventKind(), { scopes: [ids.space] });
    expect({
        forwarded,
        refused,
        kept: kept.events.map((event) => ({
            name: event.keys.name,
            installationId: event.keys.installationId,
            build: event.keys.build,
        })),
    }).toEqual({
        forwarded: 200,
        refused: ["NOT_FOUND", "NOT_FOUND", "items.0: log not found"],
        kept: [
            { name: "note.saved", installationId: kit.notes, build: kit.manifest },
            { name: "note.conflict", installationId: kit.notes, build: kit.manifest },
        ],
    });
});

test("group an export's failures in the transaction appending it, opening no issue for a refused append", async () => {
    await using kit = await TestObservability.open();

    // post a failure as the member, who serves nothing, so its append is refused
    const refused = await refusal(
        kit.post(
            "logs",
            kit.emitter.exceptions([kit.renameFailure(kit.clock.now(), "dana")]),
            kit.member,
        ),
    );

    // refuse its exception's append, hidden from the member, keeping neither the record nor an issue
    await kit.count();
    const issues = await kit.served.database.select().from(issue.table);
    const kept = await kit.served.store.events.query(log.eventKind(), { scopes: [ids.space] });
    expect({ refused, issues, kept: kept.events }).toEqual({
        refused: ["NOT_FOUND", "NOT_FOUND", "items.0: exception not found"],
        issues: [],
        kept: [],
    });
});

test("count an exception exported twice at once a single time, as the issue records each occurrence once", async () => {
    await using kit = await TestObservability.open();
    const exported = kit.emitter.exceptions([kit.renameFailure(kit.clock.now(), "dana")]);

    // post the same export twice concurrently, as an exporter's overlapping retry does
    const posted = await Promise.all([kit.post("logs", exported), kit.post("logs", exported)]);
    await kit.count();

    // keep one issue counting the exception once
    const client = kit.member.client(observabilityPackage);
    const issues = (await client.issue.list({ ...issue.in(ids.space) })).items;
    expect({ posted, counts: issues.map((row) => [row.count, row.people]) }).toEqual({
        posted: [200, 200],
        counts: [[1, 1]],
    });
});

test("keep an issue ignored for good through further failures, reopen it until its next failure, and refuse a revision of another installation", async () => {
    await using kit = await TestObservability.open();
    const now = kit.clock.now();
    const client = kit.member.client(observabilityPackage);
    const read = async () =>
        present((await client.issue.list({ ...issue.in(ids.space) })).items[0], "the issue");

    // fail, ignore the issue for good, and fail again
    await kit.post("logs", kit.emitter.exceptions([kit.renameFailure(now, "dana")]));
    await kit.count();
    const opened = await read();
    await client.issue.ignore({
        ...issue.target(opened),
        requestId: RequestId.create(kit.environment),
        until: { kind: "never" },
    });
    await kit.post("logs", kit.emitter.exceptions([kit.renameFailure(now + 1, "dana")]));
    await kit.count();
    const ignored = await read();

    // reopen it, which reopens it until its next failure, and refuse a revision the installation never ran
    const reopened = await client.issue.reopen({
        ...issue.target(ignored),
        requestId: RequestId.create(kit.environment),
    });
    const foreign = schema
        .identifier("installation-revision")
        .parse("installation-revision-01996ab0-0000-7000-8000-0000000000f9");
    const resolve = (until: Until) =>
        refusal(
            client.issue.resolve({
                ...issue.target(reopened),
                requestId: RequestId.create(kit.environment),
                until,
            }),
        );
    const refused = [
        await resolve({ kind: "revision", after: foreign }),
        await resolve({ kind: "count", count: 2 }),
    ];

    expect({
        ignored: [ignored.status, ignored.count, ignored.until],
        reopened: [reopened.status, reopened.until],
        refused,
    }).toEqual({
        ignored: ["ignored", 2, { kind: "never" }],
        reopened: ["unresolved", { kind: "event" }],
        refused: [
            [
                "BAD_REQUEST",
                "REVISION_UNKNOWN",
                `revision ${foreign} is not a revision of installation ${kit.notes}`,
            ],
            ["BAD_REQUEST", "UNTIL_PASSED", "the issue counted 2 events, at or past 2"],
        ],
    });
});

test("group failures logged and annotated on spans into an issue at their declaration, counting each carrying record once, and regress it once resolved", async () => {
    await using kit = await TestObservability.open();
    const now = kit.clock.now();
    const trace = `${now.toString(16).padStart(12, "0")}${"a".repeat(20)}`;
    const spanId = "b".repeat(16);

    // fail twice for dana and once for erin, and refuse a missing note, which is a declared outcome
    await kit.post(
        "logs",
        kit.emitter.exceptions([
            { ...kit.renameFailure(now, "dana"), trace, span: spanId },
            kit.renameFailure(now + 1, "dana"),
            kit.renameFailure(now + 2, "erin"),
            {
                time: now + 3,
                attributes: {
                    "error.type": "NOT_FOUND",
                    "exception.type": "CallError",
                    "exception.message": "no note",
                    "exception.escaped": true,
                },
            },
        ]),
    );

    // annotate the span the first failure was logged in with the same exception
    await kit.post(
        "traces",
        kit.emitter.traces([
            {
                trace,
                span: spanId,
                name: "note.rename",
                start: now - 1,
                end: now + 1,
                status: 2,
                attributes: { "enduser.id": "dana" },
                annotations: [
                    {
                        name: "exception",
                        time: now,
                        attributes: kit.renameFailure(now, "dana").attributes,
                    },
                ],
            },
        ]),
    );

    // read the issues as the member, and the issue each event carries
    await kit.count();
    const client = kit.member.client(observabilityPackage);
    const issues = async () => (await client.issue.list({ ...issue.in(ids.space) })).items;
    const opened = present((await issues())[0], "the issue");
    const { events } = kit.served.store;
    const stamped = async () => ({
        logs: (await events.query(log.eventKind(), { scopes: [ids.space] })).events.map(
            (event) => event.keys.issue,
        ),
        spans: (await events.query(span.eventKind(), { scopes: [ids.space] })).events.map(
            (event) => event.keys.issue,
        ),
    });
    const before = await stamped();

    // regress the issue once it fails again after the member resolves it
    await client.issue.resolve({
        ...issue.target(opened),
        requestId: RequestId.create(kit.environment),
        until: { kind: "event" },
    });
    await kit.post("logs", kit.emitter.exceptions([kit.renameFailure(now + 10, "dana")]));
    await kit.count();
    const notes = await kit
        .person(kit.space.owner)
        .client(stackPackage)
        .rollout.get({ spaceId: ids.space, id: kit.notes });
    expect({
        opened: {
            title: opened.title,
            errorType: opened.errorType,
            culprit: opened.culprit,
            declaration: opened.declaration,
            level: opened.level,
            count: opened.count,
            people: opened.people,
            firstRevision: opened.firstRevisionId,
        },
        people: (await kit.served.database.select().from(issuePerson.table))
            .map(({ person }) => person)
            .toSorted(),
        before,
        regressed: (await issues()).map(({ status, count }) => ({ status, count })),
    }).toEqual({
        opened: {
            title: "RangeError: title is too long",
            errorType: "RangeError",
            culprit: "package-01996ab0-0000-7000-8000-000000000004/src/note.ts#Note.rename",
            declaration:
                "package-01996ab0-0000-7000-8000-000000000004/src/note.ts#Note:object:note.rename:method",
            level: "error",
            count: 4,
            people: 2,
            firstRevision: notes.appliedRevisionId,
        },
        people: ["dana", "erin"],
        before: { logs: [opened.id, opened.id, opened.id, null], spans: [opened.id] },
        regressed: [{ status: "regressed", count: 5 }],
    });
});

test("count an exporter's retry of an untraced export once", async () => {
    await using kit = await TestObservability.open();
    const now = kit.clock.now();

    // post the same two failures twice, as an exporter retrying a lost response does
    const exported = kit.emitter.exceptions([
        kit.renameFailure(now, "dana"),
        kit.renameFailure(now + 1, "erin"),
    ]);
    await kit.post("logs", exported);
    await kit.post("logs", exported);
    await kit.count();

    // keep each record once, and one issue counting each failure once
    const issues = await kit.served.database.select().from(issue.table);
    const logs = await kit.served.store.events.query(log.eventKind(), { scopes: [ids.space] });
    expect({
        logs: logs.events.length,
        issues: issues.map(({ count, people }) => ({ count, people })),
    }).toEqual({ logs: 2, issues: [{ count: 2, people: 2 }] });
});

test("answer an export with OTLP's empty response, and refuse an unknown path, another method and another media type", async () => {
    await using kit = await TestObservability.open();
    const endpoint = kit.machine.endpoint(observabilityPackage);
    const exported = JSON.stringify(
        kit.emitter.logs([{ time: kit.clock.now(), name: "note.saved" }]),
    );
    const answer = async (path: string, request: RequestInit) => {
        const response = await endpoint.fetch(
            new Request(`https://observability.test${path}`, request),
        );

        return [response.status, response.headers.get("allow"), await response.text()];
    };

    // post as JSON, to an unknown path, with GET, and as protobuf
    const json = { "content-type": "application/json" };
    expect([
        await answer("/v1/logs", { method: "POST", headers: json, body: exported }),
        await answer("/v1/profiles", { method: "POST", headers: json, body: exported }),
        await answer("/v1/logs", { method: "GET" }),
        await answer("/v1/logs", {
            method: "POST",
            headers: { "content-type": "application/x-protobuf" },
            body: new Uint8Array([10, 0]),
        }),
    ]).toEqual([
        [200, null, "{}"],
        [404, null, ""],
        [405, "POST", ""],
        [415, null, ""],
    ]);
});

test("keep a page's visits and actions apart from its logs, its visitor a hash of the day's salt, rotating the salt at midnight", async () => {
    // open the space minutes before a UTC midnight
    const midnight = Date.UTC(2026, 9, 17);
    await using kit = await TestObservability.open(midnight - 5 * MINUTE);
    const now = kit.clock.now();
    const page = kit.emitter.with(BROWSER);
    const visited = (time: number) =>
        page.logs([
            {
                time,
                name: VISIT_EVENT,
                attributes: {
                    [ANALYTICS_ATTRIBUTES.route]: "/notes/:id",
                    "session.id": "session-1",
                },
            },
        ]);

    // visit a note, share it, and visit again just past midnight
    await kit.post("logs", visited(now));
    await kit.post(
        "logs",
        page.logs([
            {
                time: now + 1,
                name: ACTION_EVENT,
                attributes: { [ANALYTICS_ATTRIBUTES.action]: "note.shared", audience: "space" },
            },
        ]),
    );
    await kit.simulation.run({ to: midnight + 1 });
    await kit.post("logs", visited(midnight + 1));

    // read the visits once both arrived, the action, the logs and the salts kept
    const { events } = kit.served.store;
    const scopes: EventFilter = { scopes: [ids.space] };
    const read = async () => (await events.query(visit.eventKind(), scopes)).events;
    await kit.simulation.run({ until: async () => (await read()).length === 2 });
    const visits = await read();
    const actions = (await events.query(action.eventKind(), scopes)).events;
    const logs = (await events.query(log.eventKind(), scopes)).events;
    const salts = await kit.served.database.select().from(visitorSalt.table);
    const hashed = async (day: string) => {
        const salt = present(
            salts.find((each) => each.day === day),
            `the salt of ${day}`,
        ).salt;
        const browser = [
            kit.notes,
            BROWSER[EMITTER_ATTRIBUTES.address],
            BROWSER[EMITTER_ATTRIBUTES.userAgent],
        ];
        const digest = await Digest.of([salt, ...browser].join("\n"), kit.environment.crypto);

        return digest.slice(0, VISITOR_DIGITS);
    };
    const today = new Date(now).toISOString().slice(0, 10);
    const tomorrow = new Date(midnight).toISOString().slice(0, 10);

    // hash one visitor alike within a day and apart across days, under each day's salt
    expect({
        routes: visits.map((event) => event.keys.route),
        visitors: [visits[0], actions[0], visits[1]].map((event) => event?.keys.visitor),
        logs: logs.length,
        salts: salts.map((salt) => salt.day),
    }).toEqual({
        routes: ["/notes/:id", "/notes/:id"],
        visitors: [await hashed(today), await hashed(today), await hashed(tomorrow)],
        logs: 0,
        salts: [today, tomorrow],
    });
});

test("record each session's end with its build and status, keying its records by session, so release health counts crashed sessions per build", async () => {
    await using kit = await TestObservability.open();
    const now = kit.clock.now();

    // end one crashed session after a failure, and one session without failures
    await kit.post(
        "logs",
        kit.emitter.logs([
            {
                time: now,
                name: "note.failed",
                severity: 17,
                attributes: { [SESSION_ATTRIBUTE]: "session-1" },
            },
            {
                time: now + 1,
                name: SESSION_EVENTS.end,
                severity: 9,
                attributes: {
                    [SESSION_ATTRIBUTE]: "session-1",
                    [SESSION_STATUS_ATTRIBUTE]: "crashed",
                },
            },
            {
                time: now + 2,
                name: SESSION_EVENTS.end,
                severity: 9,
                attributes: { [SESSION_ATTRIBUTE]: "session-2", [SESSION_STATUS_ATTRIBUTE]: "ok" },
            },
        ]),
    );

    // read the failure's session, and count the sessions per build and status
    const { events } = kit.served.store;
    const logs = (await events.query(log.eventKind(), { scopes: [ids.space] })).events;
    const health = await events.series(
        session.eventKind(),
        { scopes: [ids.space] },
        { fold: "count", group: ["build", "status"] },
    );

    // expect the failure in the crashed session, and one crashed and one clean session of the build
    expect({
        sessions: logs.map((event) => event.keys.session),
        health: health
            .map((series) => ({
                build: series.group["build"],
                status: series.group["status"],
                count: series.steps[0]?.value,
            }))
            .toSorted((left, right) => String(left.status).localeCompare(String(right.status))),
    }).toEqual({
        sessions: ["session-1"],
        health: [
            { build: kit.manifest, status: "crashed", count: 1 },
            { build: kit.manifest, status: "ok", count: 1 },
        ],
    });
});
