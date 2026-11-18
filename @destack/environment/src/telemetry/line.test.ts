import { SeverityNumber } from "@opentelemetry/api";
import { expect, test } from "@destack/test";
import { Clock } from "../clock/clock.ts";
import { Telemetry } from "./telemetry.ts";

test("write each record to the runtime's log as one line with its time, severity and attributes, an exception with its type, message and stack", async () => {
    // log an event and an exception to a runtime's log
    const lines: [string, SeverityNumber][] = [];
    const telemetry = Telemetry.local(Clock.system, (line, severity) =>
        lines.push([line, severity]),
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
