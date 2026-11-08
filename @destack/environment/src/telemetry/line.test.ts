import { SeverityNumber } from "@opentelemetry/api";
import { expect, test } from "@destack/test";
import { Clock } from "../clock/clock.ts";
import { Telemetry } from "./telemetry.ts";

test("write each record to the runtime's log as one line at its severity, an exception with its type, message and stack", async () => {
    // log an event and an exception to a runtime's log
    const lines: [string, SeverityNumber][] = [];
    const telemetry = Telemetry.local(Clock.system, (line, severity) =>
        lines.push([line, severity]),
    );
    const logger = telemetry.loggers.getLogger("@example/notes");
    logger.emit({
        eventName: "note.saved",
        severityNumber: SeverityNumber.INFO,
        severityText: "INFO",
    });
    logger.emit({
        eventName: "exception",
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
        ["INFO note.saved\n", SeverityNumber.INFO],
        [
            "ERROR exception TypeError: the note is gone\nTypeError: the note is gone\n    at save\n",
            SeverityNumber.ERROR,
        ],
    ]);
});
