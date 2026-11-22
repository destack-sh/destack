import { AsyncLocalStorageContextManager } from "@opentelemetry/context-async-hooks";
import { SeverityNumber } from "@opentelemetry/api";
import { expect, test } from "@destack/test";
import { Clock } from "../clock/clock.ts";
import { LineExporter } from "./line.ts";
import { SpanRunner } from "./runner.ts";
import { SpanPath } from "./span.ts";

test("write each record to the runtime's log as one line with its time, severity and attributes, an exception with its type, message and stack", async () => {
    // log an event and an exception to a runtime's log
    const lines: [string, SeverityNumber][] = [];
    const telemetry = new LineExporter((line, severity) => lines.push([line, severity])).telemetry(
        Clock.system,
        new AsyncLocalStorageContextManager().enable(),
    );
    const logger = telemetry.loggers.getLogger("@example/notes");
    const time = Date.UTC(2026, 10, 18, 12, 4, 31);
    logger.emit({
        eventName: "note.saved",
        timestamp: time,
        severityNumber: SeverityNumber.INFO,
        severityText: "INFO",
        attributes: { "note.id": "note-1", "note.title": "Plan the week", "note.words": 3 },
    });
    logger.emit({
        eventName: "exception",
        timestamp: time,
        severityNumber: SeverityNumber.ERROR,
        severityText: "ERROR",
        attributes: {
            "exception.type": "TypeError",
            "exception.message": "the note is gone",
            "exception.stacktrace": "TypeError: the note is gone\n    at save",
        },
    });

    // write the event, then the failure with its stack, each with its severity
    expect(lines).toEqual([
        [
            '12:04:31 INFO  note.saved note.id=note-1 note.title="Plan the week" note.words=3\n',
            SeverityNumber.INFO,
        ],
        [
            "12:04:31 ERROR exception TypeError: the note is gone\nTypeError: the note is gone\n    at save\n",
            SeverityNumber.ERROR,
        ],
    ]);
});

test("write each info span as one line once it ends with its trace, a failed one as an error, and leave debug spans out", async () => {
    // run an info span, a failing info span and a debug span on a runtime's log
    const lines: string[] = [];
    const telemetry = new LineExporter((line) => lines.push(line)).telemetry(
        Clock.system,
        new AsyncLocalStorageContextManager().enable(),
    );
    const span = SpanRunner.of(
        telemetry.tracers.getTracer("@example/notes"),
        telemetry.meters.getMeter("@example/notes"),
        telemetry,
    );
    await span(
        "notes.start",
        { "notes.count": 2 },
        async () => {
            SpanPath.add(telemetry.context.active(), { "notes.reads": 1 });
            await span("notes.open", {}, () => undefined, { level: "info" });
        },
        { level: "info" },
    );
    await span("notes.load", {}, () => Promise.reject(new Error("the disk is gone")), {
        level: "info",
    }).catch(() => undefined);
    await span("notes.request", {}, () => undefined);

    // write the info spans with their durations, traces and parents, the failed one with its error type
    const ids: string[] = [];
    const named = (id: string) => {
        // name each identifier by the order it first appears in
        if (!ids.includes(id)) {
            ids.push(id);
        }

        return `#${ids.indexOf(id)}`;
    };
    expect(
        lines.map((line) =>
            line
                .replace(/^\d\d:\d\d:\d\d /u, "")
                .replace(/duration_ms=\d+/u, "duration_ms=…")
                .replaceAll(/(?<==)[\da-f]{16,32}\b/gu, named),
        ),
    ).toEqual([
        "INFO  notes.open duration_ms=… trace_id=#0 span_id=#1 parent_span_id=#2\n",
        "INFO  notes.start notes.count=2 duration_ms=… trace_id=#0 span_id=#2\n",
        'ERROR notes.load error.type=INTERNAL_SERVER_ERROR error.message="the disk is gone" duration_ms=… trace_id=#3 span_id=#4\n',
    ]);
});

test("write each record as one JSON object when the options ask for json", () => {
    // log an event through a JSON line exporter
    const lines: string[] = [];
    const telemetry = new LineExporter(
        (line) => lines.push(line),
        LineExporter.options({ DESTACK_LOG_FORMAT: "json" }),
    ).telemetry(Clock.system, new AsyncLocalStorageContextManager().enable());
    telemetry.loggers.getLogger("@example/notes").emit({
        eventName: "note.saved",
        timestamp: Date.UTC(2026, 10, 18, 12, 4, 31),
        severityNumber: SeverityNumber.INFO,
        severityText: "INFO",
        attributes: { "note.id": "note-1", "note.words": 3 },
    });

    // write the time, level, name and attributes as one object per line
    expect(lines.map((line): unknown => JSON.parse(line))).toEqual([
        {
            time: "2026-11-18T12:04:31.000Z",
            level: "INFO",
            name: "note.saved",
            "note.id": "note-1",
            "note.words": 3,
        },
    ]);
});
