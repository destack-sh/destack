import { AccessContext, Caller } from "@destack/access";
import type { Subject } from "@destack/sync";
import { present } from "@destack/schema";
import {
    ACTION_EVENT,
    ANALYTICS_ATTRIBUTES,
    SESSION_ATTRIBUTE,
    SESSION_EVENTS,
    SESSION_STATUS_ATTRIBUTE,
    VISIT_EVENT,
} from "@destack/telemetry/otlp";
import { expect, refusal, test } from "@destack/test";
import { action, log, session, span, visit } from "../src/object/index.ts";
import { issue, issuePerson, visitorSalt } from "../src/object/index.ts";
import { EMITTER_ATTRIBUTES } from "../src/otlp/index.ts";
import {
    exceptionsExport,
    logsExport,
    NOTES_MANIFEST,
    NOTES_RESOURCE,
    recordNotesRevision,
    renameFailure,
    tracesExport,
} from "./fixture/notes.ts";
import {
    actors,
    ids,
    mutate,
    type ObservedSpace,
    openSpace,
    reconcile,
    START,
} from "./fixture/space.ts";

/** A visitor's hash: 32 hexadecimal digits. */
const HASH = /^[0-9a-f]{32}$/u;

/** The milliseconds in a day. */
const DAY = 86_400_000;

/** The browser a page's export came from, as its machine saw it. */
const BROWSER = {
    [EMITTER_ATTRIBUTES.address]: "203.0.113.7",
    [EMITTER_ATTRIBUTES.userAgent]: "Mozilla/5.0 (Macintosh)",
};

/** Post an OTLP/JSON export of a signal to the space's observability as a caller. */
async function post(
    space: ObservedSpace,
    signal: "logs" | "traces",
    body: unknown,
    caller: Subject = actors.machine,
): Promise<number | undefined> {
    const request = new Request(`https://observability.test/v1/${signal}`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(body),
    });

    return (await space.fetch(request, caller))?.status;
}

test("append an export through the otlp adapter as the machine forwarding it, and conceal the append from a reader not serving the space", async () => {
    const space = await openSpace();

    // forward two records as the machine, and one as alice, who serves nothing
    const forwarded = await post(
        space,
        "logs",
        logsExport([
            { time: START, name: "note.saved", severity: 9 },
            { time: START + 1, name: "note.conflict", severity: 13 },
        ]),
    );
    const refused = await refusal(
        post(
            space,
            "logs",
            logsExport([{ time: START, name: "note.forged", severity: 9 }]),
            actors.alice,
        ),
    );

    // keep the machine's records alone, stamped with the forwarder's installation and build
    const kept = await space.events.query(log.eventKind(), { scopes: [ids.space] });
    expect({
        forwarded,
        refused,
        kept: kept.events.map((event) => ({
            name: event.keys.name,
            installationId: event.keys.installationId,
            build: event.keys.build,
        })),
    }).toEqual({
        forwarded: 204,
        refused: ["NOT_FOUND", "items.0: log not found"],
        kept: [
            { name: "note.saved", installationId: ids.notes, build: NOTES_MANIFEST },
            { name: "note.conflict", installationId: ids.notes, build: NOTES_MANIFEST },
        ],
    });
});

test("group failures logged and annotated on spans into an issue at their declaration, counting a span's exception once, and regress it once resolved", async () => {
    // serve observability with the notes build at a revision
    const space = await openSpace();
    const revision = await recordNotesRevision(space.store.database, START);
    const trace = `${START.toString(16).padStart(12, "0")}${"a".repeat(20)}`;
    const spanId = "b".repeat(16);

    // fail twice for dana and once for erin, and refuse a missing note, which is a declared outcome
    await post(
        space,
        "logs",
        exceptionsExport([
            { ...renameFailure(START, "dana"), trace, span: spanId },
            renameFailure(START + 1, "dana"),
            renameFailure(START + 2, "erin"),
            {
                time: START + 3,
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
    await post(
        space,
        "traces",
        tracesExport([
            {
                trace,
                span: spanId,
                name: "note.rename",
                start: START - 1,
                end: START + 1,
                status: 2,
                attributes: { "enduser.id": "dana" },
                annotations: [
                    {
                        name: "exception",
                        time: START,
                        attributes: renameFailure(START, "dana").attributes,
                    },
                ],
            },
        ]),
    );

    // read the issues as alice, and the issue each event carries
    const issues = async () =>
        (
            await space.store.query(
                AccessContext.of(Caller.of(actors.alice), Date.now()),
                issue.call("list", { ...issue.in(ids.space) }),
            )
        ).items;
    const opened = present((await issues())[0], "the issue");
    const stamped = async () => ({
        logs: (await space.events.query(log.eventKind(), { scopes: [ids.space] })).events.map(
            (event) => event.keys.issue,
        ),
        spans: (await space.events.query(span.eventKind(), { scopes: [ids.space] })).events.map(
            (event) => event.keys.issue,
        ),
    });
    const before = await stamped();

    // regress the issue once it fails again after alice resolves it
    await mutate(space.store, actors.alice, [issue.call("resolve", issue.target(opened))]);
    await post(space, "logs", exceptionsExport([renameFailure(START + 10, "dana")]));
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
        people: (await space.store.database.select().from(issuePerson.table)).map(
            ({ person }) => person,
        ),
        before,
        regressed: (await issues()).map(({ status, count }) => ({ status, count })),
        failures: space.failures,
    }).toEqual({
        opened: {
            title: "RangeError: title is too long",
            errorType: "RangeError",
            culprit: `${ids.package}/src/note.ts#Note.rename`,
            declaration: `${ids.package}/src/note.ts#Note:object:note.rename:method`,
            level: "error",
            count: 3,
            people: 2,
            firstRevision: revision,
        },
        people: ["dana", "erin"],
        before: { logs: [opened.id, opened.id, opened.id, null], spans: [opened.id] },
        regressed: [{ status: "regressed", count: 4 }],
        failures: [],
    });
});

test("keep a page's visits and actions apart from its logs, its visitor a hash of the day's salt, rotating the salt each day", async () => {
    const space = await openSpace();
    const page = { ...NOTES_RESOURCE, ...BROWSER };
    const visited = (time: number) =>
        logsExport(
            [
                {
                    time,
                    name: VISIT_EVENT,
                    attributes: {
                        [ANALYTICS_ATTRIBUTES.route]: "/notes/:id",
                        "session.id": "session-1",
                    },
                },
            ],
            page,
        );

    // visit a note, share it, and visit again the next day
    await post(space, "logs", visited(START));
    await post(
        space,
        "logs",
        logsExport(
            [
                {
                    time: START + 1,
                    name: ACTION_EVENT,
                    attributes: { [ANALYTICS_ATTRIBUTES.action]: "note.shared", audience: "space" },
                },
            ],
            page,
        ),
    );
    space.clock.advance(DAY);
    await post(space, "logs", visited(START + DAY));
    await reconcile(space, "expiry");

    // read the visits, the action, the logs and the salts kept
    const visits = (await space.events.query(visit.eventKind(), { scopes: [ids.space] })).events;
    const actions = (await space.events.query(action.eventKind(), { scopes: [ids.space] })).events;
    const logs = (await space.events.query(log.eventKind(), { scopes: [ids.space] })).events;
    const salts = await space.store.database.select().from(visitorSalt.table);
    const [first, next] = visits.map((event) => event.keys.visitor);
    // hash one visitor alike within a day and apart across days, the day-old salt expired
    expect({
        routes: visits.map((event) => event.keys.route),
        isHashed: HASH.test(String(first)) && HASH.test(String(next)),
        isStable: actions[0]?.keys.visitor === first,
        isRotated: first !== next,
        logs: logs.length,
        salts: salts.map((salt) => salt.day),
    }).toEqual({
        routes: ["/notes/:id", "/notes/:id"],
        isHashed: true,
        isStable: true,
        isRotated: true,
        logs: 0,
        salts: [new Date(START + DAY).toISOString().slice(0, 10)],
    });
});

test("record each session's end with its build and status, keying its records by session, so release health counts crashed sessions per build", async () => {
    const space = await openSpace();

    // end one crashed session after a failure, and one session without failures
    await post(
        space,
        "logs",
        logsExport([
            {
                time: START,
                name: "note.failed",
                severity: 17,
                attributes: { [SESSION_ATTRIBUTE]: "session-1" },
            },
            {
                time: START + 1,
                name: SESSION_EVENTS.end,
                severity: 9,
                attributes: {
                    [SESSION_ATTRIBUTE]: "session-1",
                    [SESSION_STATUS_ATTRIBUTE]: "crashed",
                },
            },
            {
                time: START + 2,
                name: SESSION_EVENTS.end,
                severity: 9,
                attributes: { [SESSION_ATTRIBUTE]: "session-2", [SESSION_STATUS_ATTRIBUTE]: "ok" },
            },
        ]),
    );

    // read the failure's session, and count the sessions per build and status
    const logs = (await space.events.query(log.eventKind(), { scopes: [ids.space] })).events;
    const health = await space.events.series(
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
            { build: NOTES_MANIFEST, status: "crashed", count: 1 },
            { build: NOTES_MANIFEST, status: "ok", count: 1 },
        ],
    });
});
