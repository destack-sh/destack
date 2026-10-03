import type { Context } from "@orpc/server";
import { AsyncIteratorClass } from "@orpc/shared";
import type { ProcedureAccess } from "../procedure/procedure.ts";
import { reportError } from "./error.ts";
import type { HandlerOptions } from "./handler.ts";
import { denialOf, ServiceError } from "../error/index.ts";

/** A procedure call. */
export interface ProcedureCall<State extends Context> {
    /** The access requirements. */
    access: ProcedureAccess;
    /** The procedure's key path. */
    path: readonly string[];
    /** The unvalidated input. */
    input: unknown;
    /** The request context. */
    context: State;
    /** The cancellation signal. */
    signal?: AbortSignal;
}

/** An audit event of a call. */
export interface ProcedureAudit<State extends Context> {
    /** The call. */
    call: ProcedureCall<State>;
    /** How the call ended. */
    outcome: "success" | "failure" | "denied" | "cancelled";
    /** The failure. */
    error?: unknown;
}

/** Authorize a call and audit it after it ends. */
export async function invokeProcedure<State extends Context>(
    call: ProcedureCall<State>,
    next: () => Promise<unknown>,
    options: Pick<HandlerOptions<State>, "authorize" | "audit">,
): Promise<unknown> {
    // tell a denial from a failure, recording every call's end for the recorder to keep or skip
    const audit = options.audit;
    try {
        if (options.authorize) {
            await options.authorize(call);
        }
    } catch (error) {
        const failure = reportError(error);
        if (audit) {
            await recordAudit({ call, outcome: outcomeOf(failure), error: failure }, audit);
        }
        throw failure;
    }

    // record the handler's outcome
    let result: unknown;
    try {
        result = await next();
    } catch (error) {
        const failure = reportError(error);
        if (audit) {
            await recordAudit({ call, outcome: outcomeOf(failure), error: failure }, audit);
        }
        throw failure;
    }

    // record a stream when it closes
    if (isAsyncIterable(result)) {
        return streamProcedure(result, call, options);
    }
    if (audit) {
        await recordAudit({ call, outcome: "success" }, audit);
    }

    return result;
}

/** Authorize and audit a stream. */
function streamProcedure<State extends Context>(
    stream: AsyncIterable<unknown>,
    call: ProcedureCall<State>,
    options: Pick<HandlerOptions<State>, "authorize" | "audit">,
): AsyncIteratorClass<unknown> {
    // record completion apart from cancellation
    const audit = options.audit;
    let outcome: ProcedureAudit<State>["outcome"] = "cancelled";
    let failure: unknown;
    const iterator = stream[Symbol.asyncIterator]();

    return new AsyncIteratorClass(
        async () => {
            try {
                // recheck access before each value
                if (options.authorize) {
                    await options.authorize(call);
                }
                const result = await iterator.next();
                if (options.authorize && result.done !== true) {
                    await options.authorize(call);
                }
                if (result.done === true) {
                    outcome = "success";
                }

                return result;
            } catch (error) {
                const reported = reportError(error);
                failure = reported;
                outcome = outcomeOf(reported);
                throw reported;
            }
        },
        async (reason) => {
            // record the stream's end
            try {
                if (reason !== "next" || outcome !== "success") {
                    await iterator.return?.();
                }
            } catch (error) {
                outcome = "failure";
                failure = error;
                throw reportError(error);
            } finally {
                if (audit) {
                    await recordAudit({ call, outcome, error: failure }, audit);
                }
            }
        },
    );
}

/** Record an audit event and keep any handler failure. */
async function recordAudit<State extends Context>(
    event: ProcedureAudit<State>,
    audit: (event: ProcedureAudit<State>) => Promise<void>,
): Promise<void> {
    try {
        await audit(event);
    } catch (error) {
        // keep both failures
        const failure =
            event.error !== undefined
                ? new AggregateError([event.error, error], "procedure failure audit failed")
                : error;

        throw reportError(failure);
    }
}

/** Classify a failure as a denial, concealed or not, or a failure. */
function outcomeOf(failure: ServiceError<string, unknown>): "denied" | "failure" {
    return denialOf(failure) === undefined ? "failure" : "denied";
}

/** Report whether a procedure's result is a stream. */
function isAsyncIterable(value: unknown): value is AsyncIterable<unknown> {
    return value !== null && typeof value === "object" && Symbol.asyncIterator in value;
}
