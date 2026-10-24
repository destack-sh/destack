import { diag, DiagLogLevel } from "@opentelemetry/api";

/** A running telemetry's report of an SDK warning: its message and the details the SDK passes. */
type DiagnosticReport = (message: string, ...details: unknown[]) => void;

// NOTE #Architecture: OpenTelemetry's SDK warns through one process-wide diagnostic logger, so this module owns that one registration for every running telemetry
/** The reports of the running telemetries, which the one diagnostic logger reaches. */
const REPORTS = new Set<DiagnosticReport>();

/** Drop an SDK message below a warning. */
function ignore(): void {}

/** Pass a warning to every running telemetry's report. */
function report(message: string, ...details: unknown[]): void {
    for (const each of REPORTS) {
        each(message, ...details);
    }
}

/** The one owner of OpenTelemetry's diagnostic logger, reaching every running telemetry. */
export const Diagnostics = {
    /** Pass the SDK's warnings to a report until the returned removal. */
    register(reporting: DiagnosticReport): () => void {
        // register the logger once, for the first report
        if (REPORTS.size === 0) {
            diag.setLogger(
                { error: report, warn: report, info: ignore, debug: ignore, verbose: ignore },
                DiagLogLevel.WARN,
            );
        }
        REPORTS.add(reporting);

        // remove the report, and the logger with the last one
        return () => {
            REPORTS.delete(reporting);
            if (REPORTS.size === 0) {
                diag.disable();
            }
        };
    },
};
