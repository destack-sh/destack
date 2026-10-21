import type { AuditOutcome } from "../audit/execution.ts";
import type { Context } from "@orpc/server";
import { AsyncIteratorClass } from "@orpc/shared";
import type { ProcedureAccess } from "../procedure/index.ts";
import type { HandlerOptions } from "./handler.ts";
import { findDenial, CallError } from "../error/index.ts";
import { reportError } from "./report.ts";

/** The context of one call of a procedure, as the server runs it. */
export interface ProcedureContext<State extends Context> {
    /** The access requirements. */
    access: ProcedureAccess;
    /** The procedure's key path. */
    path: readonly string[];
    /** The unvalidated input. */
    input: unknown;
    /** The request's context. */
    request: State;
    /** The cancellation signal. */
    signal?: AbortSignal;
}

/** An audit event of a call. */
export interface ProcedureAudit<State extends Context> {
    /** The call's context. */
    context: ProcedureContext<State>;
    /** How the call ended. */
    outcome: AuditOutcome;
    /** The failure. */
    error?: unknown;
}

/** Procedure calls as the server runs them. */
export const ProcedureContext = {
    /** Authorize a call and audit it after it ends. */
    async invoke<State extends Context>(
        context: ProcedureContext<State>,
        next: () => Promise<unknown>,
        options: Pick<HandlerOptions<State>, "authorize" | "audit">,
    ): Promise<unknown> {
        // tell a denial from a failure
        const audit = options.audit;
        try {
            if (options.authorize) {
                await options.authorize(context);
            }
        } catch (error) {
            const failure = reportError(error);
            if (audit) {
                await recordAudit(
                    { context: context, outcome: auditOutcome(failure), error: failure },
                    audit,
                );
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
                await recordAudit(
                    { context: context, outcome: auditOutcome(failure), error: failure },
                    audit,
                );
            }
            throw failure;
        }

        // record a stream when it closes
        if (isAsyncIterable(result)) {
            return streamProcedure(result, context, options);
        }
        if (audit) {
            await recordAudit({ context: context, outcome: "success" }, audit);
        }

        return result;
    },
};

/** Authorize and audit a stream. */
function streamProcedure<State extends Context>(
    stream: AsyncIterable<unknown>,
    context: ProcedureContext<State>,
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
                    await options.authorize(context);
                }
                const result = await iterator.next();
                if (options.authorize && result.done !== true) {
                    await options.authorize(context);
                }
                if (result.done === true) {
                    outcome = "success";
                }

                return result;
            } catch (error) {
                const reported = reportError(error);
                failure = reported;
                outcome = auditOutcome(reported);
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
                    await recordAudit({ context: context, outcome, error: failure }, audit);
                }
            }
        },
    );
}

/** Record an audit event and rethrow any handler failure. */
async function recordAudit<State extends Context>(
    event: ProcedureAudit<State>,
    audit: (event: ProcedureAudit<State>) => Promise<void>,
): Promise<void> {
    try {
        await audit(event);
    } catch (error) {
        // report both failures
        const failure =
            event.error !== undefined
                ? new AggregateError([event.error, error], "procedure failure audit failed")
                : error;

        throw reportError(failure);
    }
}

/** Classify a failure as a denial, concealed or not, or a failure. */
function auditOutcome(failure: CallError<string, unknown>): "denied" | "failure" {
    return findDenial(failure) === undefined ? "failure" : "denied";
}

/** Report whether a procedure's result is a stream. */
function isAsyncIterable(value: unknown): value is AsyncIterable<unknown> {
    return value !== null && typeof value === "object" && Symbol.asyncIterator in value;
}
