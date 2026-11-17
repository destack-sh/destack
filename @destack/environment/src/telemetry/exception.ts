import type { Attributes, Span } from "@opentelemetry/api";
import { DomainError, Failure } from "@destack/error";

/** The event name of an exception on a log record or a span, as OpenTelemetry names it. */
export const EXCEPTION_EVENT = "exception";

/** The attribute with a failure's reason, such as REVISION_STALE, beside its code in error.type. */
export const REASON_ATTRIBUTE = "destack.error.reason";

/** The start of the section a stack trace lists each cause in. */
const CAUSE_PREFIX = "Caused by: ";

/** The causes a captured failure's stack trace lists at most, below its own stack. */
const MAX_CAUSES = 8;

/** Describe a failure by the OpenTelemetry exception attributes, its causes below its stack. */
export function exceptionAttributes(error: unknown, isEscaped: boolean): Attributes {
    // write each cause below the stack as a `Caused by:` section, as Java's traces do
    const exception = error instanceof Error ? error : new Error(String(error));
    const sections = [exception.stack ?? `${exception.name}: ${exception.message}`];
    let cause = exception.cause;
    while (cause !== undefined && sections.length <= MAX_CAUSES) {
        // write an error's stack, else a value's JSON, ending at a value
        const section =
            cause instanceof Error
                ? (cause.stack ?? `${cause.name}: ${cause.message}`)
                : JSON.stringify(cause);
        sections.push(`${CAUSE_PREFIX}${section}`);
        cause = cause instanceof Error ? cause.cause : undefined;
    }

    // take the type and the reason of the first domain error among the failure and its causes
    const failure = Failure.causes(exception).find((each) => DomainError.is(each));

    return {
        "error.type": errorType(failure ?? exception),
        ...(failure === undefined ? {} : { [REASON_ATTRIBUTE]: failure.toCallError().reason }),
        "exception.type": exception.name,
        "exception.message": exception.message,
        "exception.stacktrace": sections.join("\n"),
        "exception.escaped": isEscaped,
    };
}

/** List the first line of each cause in a stack trace `exceptionAttributes` wrote. */
export function exceptionCauses(stacktrace: string): string[] {
    return stacktrace
        .split("\n")
        .filter((line) => line.startsWith(CAUSE_PREFIX))
        .map((line) => line.slice(CAUSE_PREFIX.length));
}

/** Annotate a span with an escaping failure, as the exception event its captured log carries. */
export function recordSpanException(span: Span, error: unknown): void {
    span.addEvent(EXCEPTION_EVENT, exceptionAttributes(error, true));
}

/** Read a failure's error type: its call error code, else its own code, else its name. */
function errorType(exception: Error): string {
    // the call error code a domain error reports
    if (DomainError.is(exception)) {
        return exception.toCallError().code;
    }
    // the failure's own code, such as a system error's
    else if ("code" in exception && typeof exception.code === "string") {
        return exception.code;
    }
    // the failure's name
    else {
        return exception.name;
    }
}
