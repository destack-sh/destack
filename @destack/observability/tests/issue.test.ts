import { expect, test } from "@destack/test";
import { present } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { installationRevision } from "@destack/space/object";
import { log, span } from "../src/index.ts";
import { client, eventClient, ids, serveObservability } from "./fixture/observability.ts";
import {
    exportLogs,
    NOTES_EMITTER,
    openNotesBuild,
    recordNotesRevision,
    renameFailure,
    tracesExport,
} from "./fixture/notes.ts";

test("group failures logged and annotated on spans into an issue at the declaration they ran, counting a span's exception once, list its events by issue, and regress the resolved issue", async () => {
    // serve observability with the notes build
    const notesBuild = await openNotesBuild();
    const { server, eventServer, receive, database, events, failures } = await serveObservability({
        openBuild: async () => notesBuild,
    });
    const revision = await recordNotesRevision(database);
    await expect
        .poll(async () => (await database.select().from(installationRevision.table)).length)
        .toBe(1);
    const alice = client(server, "alice");

    // fail twice for dana and once for erin, and refuse a missing note, which is a declared outcome
    const now = Date.now();
    const trace = `${now.toString(16).padStart(12, "0")}${"a".repeat(20)}`;
    const spanId = "b".repeat(16);
    await exportLogs({ receive }, [
        { ...renameFailure(now, "dana"), trace, span: spanId },
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

    // annotate the span the first failure was logged in with the same exception
    const failure = renameFailure(now, "dana");
    await receive(
        NOTES_EMITTER,
        "traces",
        tracesExport([
            {
                trace,
                span: spanId,
                name: "note.rename",
                start: now - 1,
                end: now + 1,
                status: 2,
                attributes: { "enduser.id": "dana" },
                annotations: [{ name: "exception", time: now, attributes: failure.attributes }],
            },
        ]),
    );

    // group the rename failures into one issue at the method's declaration, counting the people and the revision
    const issues = async () => (await alice.issue.list({ spaceId: ids.space })).items;
    expect(
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
    ).toEqual([
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
    const opened = present((await issues())[0], "the issue");

    // list the issue's events, and find the span stamped with it
    const where = `issue = "${opened.id}"`;
    const logged = await eventClient(eventServer, "alice").query({
        kind: "log",
        scope: ids.space,
        where,
    });
    const spans = await events.query(span, { scope: ids.space });

    // regress the issue once it fails again after alice resolves it
    await alice.issue.resolve({ spaceId: ids.space, id: opened.id, requestId: RequestId.create() });
    await exportLogs({ receive }, [renameFailure(Date.now(), "dana")]);
    expect({
        logged: logged.events.map((event) => log.parseKeys(event.keys).issue),
        spans: spans.events.map((event) => span.parseKeys(event.keys).issue),
        regressed: (await issues()).map(({ status, count }) => ({ status, count })),
        failures,
    }).toEqual({
        logged: [opened.id, opened.id, opened.id],
        spans: [opened.id],
        regressed: [{ status: "regressed", count: 4 }],
        failures: [],
    });
});

test("count an exception escaping three nested spans and logged in the innermost once, stamping each recording event with its issue", async () => {
    const { server, receive, events, failures } = await serveObservability();
    const alice = client(server, "alice");

    // log the failure in the handler's span
    const now = Date.now();
    const trace = `${now.toString(16).padStart(12, "0")}${"c".repeat(20)}`;
    const failure = renameFailure(now, "dana");
    await exportLogs({ receive }, [{ ...failure, trace, span: "3".repeat(16) }]);

    // annotate the procedure's, request's and handler's spans with the escaping failure
    const annotations = [{ name: "exception", time: now, attributes: failure.attributes }];
    await receive(
        NOTES_EMITTER,
        "traces",
        tracesExport(
            [
                { trace, span: "1".repeat(16), name: "procedure", start: now - 3, end: now + 3 },
                {
                    trace,
                    span: "2".repeat(16),
                    parent: "1".repeat(16),
                    name: "request",
                    start: now - 2,
                    end: now + 2,
                },
                {
                    trace,
                    span: "3".repeat(16),
                    parent: "2".repeat(16),
                    name: "handler",
                    start: now - 1,
                    end: now + 1,
                },
            ].map((each) => ({ ...each, status: 2 as const, annotations })),
        ),
    );

    // read the issue and the issue of every recording event
    const issues = (await alice.issue.list({ spaceId: ids.space })).items;
    const opened = present(issues[0], "the issue");
    const logs = await events.query(log, { scope: ids.space });
    const spans = await events.query(span, { scope: ids.space });

    // count one occurrence for one person, and stamp the log record and all three spans with it
    expect({
        issues: issues.map(({ count, people }) => ({ count, people })),
        logs: logs.events.map((event) => log.parseKeys(event.keys).issue),
        spans: spans.events.map((event) => {
            const keys = span.parseKeys(event.keys);

            return { name: keys.name, issue: keys.issue };
        }),
        failures,
    }).toEqual({
        issues: [{ count: 1, people: 1 }],
        logs: [opened.id],
        spans: [
            { name: "procedure", issue: opened.id },
            { name: "request", issue: opened.id },
            { name: "handler", issue: opened.id },
        ],
        failures: [],
    });
});

test("group failures by the fingerprint they request, across error types and stacks", async () => {
    const { server, receive, failures } = await serveObservability();
    const alice = client(server, "alice");

    // fail twice under one requested fingerprint with different errors, and once under another
    const now = Date.now();
    const failure = renameFailure(now, "dana");
    const requested = (time: number, type: string, fingerprint: readonly string[]) => ({
        ...failure,
        time,
        attributes: {
            ...failure.attributes,
            "error.type": type,
            "exception.type": type,
            "exception.fingerprint": fingerprint,
        },
    });
    await exportLogs({ receive }, [
        requested(now, "RangeError", ["export"]),
        requested(now + 1, "TypeError", ["export"]),
        requested(now + 2, "RangeError", ["import"]),
    ]);

    // open one issue per fingerprint, counting each failure into its own
    const issues = (await alice.issue.list({ spaceId: ids.space })).items;
    expect({
        issues: issues
            .map(({ errorType, count }) => ({ errorType, count }))
            .toSorted((left, right) => right.count - left.count),
        failures,
    }).toEqual({
        issues: [
            { errorType: "RangeError", count: 2 },
            { errorType: "RangeError", count: 1 },
        ],
        failures: [],
    });
});
