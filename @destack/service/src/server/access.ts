import type { Context } from "@orpc/server";
import { AsyncIteratorClass } from "@orpc/shared";
import type { ProcedureAccess } from "../procedure/procedure.ts";
import { reportError } from "./error.ts";
import type { HandlerOptions } from "./handler.ts";
import { ServiceError } from "../error/index.ts";

/** A procedure invocation evaluated by the host's authorization policy. */
export interface ProcedureCall<State extends Context> {
    /** Declared authentication, permission, and audit requirements. */
    access: ProcedureAccess;
    /** Procedure keys within the service. */
    path: readonly string[];
    /** Request input, which the policy must validate before interpreting. */
    input: unknown;
    /** Host-provided request context. */
    context: State;
    /** Request cancellation. */
    signal?: AbortSignal;
}

/** An invocation's audit event, including stream completion or cancellation. */
export interface ProcedureAudit<State extends Context> {
    /** The invocation evaluated by the host. */
    call: ProcedureCall<State>;
    /** The stage reached by the invocation. */
    outcome: "started" | "succeeded" | "failed" | "denied" | "cancelled";
    /** The failure supplied to the host, which must redact its persisted record. */
    error?: unknown;
}

/** Authorize and audit an invocation before returning its value or stream. */
export async function invokeProcedure<State extends Context>(
    call: ProcedureCall<State>,
    next: () => Promise<unknown>,
    options: Pick<HandlerOptions<State>, "authorize" | "audit">,
): Promise<unknown> {
    // persist the attempt before allowing application code to execute
    const audit = call.access.audit ? options.audit! : undefined;
    if (audit) {
        await recordAudit({ call, outcome: "started" }, audit);
    }

    // distinguish authorization denial from an application failure
    try {
        if (options.authorize) {
            await options.authorize(call);
        }
    } catch (error) {
        if (audit) {
            const denied =
                error instanceof ServiceError && (error.status === 401 || error.status === 403);
            await recordAudit({ call, outcome: denied ? "denied" : "failed", error }, audit);
        }
        throw reportError(error);
    }

    // retain the actual application outcome independently of audit delivery failures
    let result: unknown;
    try {
        result = await next();
    } catch (error) {
        if (audit) {
            const denied =
                error instanceof ServiceError && (error.status === 401 || error.status === 403);
            await recordAudit({ call, outcome: denied ? "denied" : "failed", error }, audit);
        }
        throw reportError(error);
    }

    // retain stream completion until its iterator closes
    if (result !== null && typeof result === "object" && Symbol.asyncIterator in result) {
        return streamProcedure(result as AsyncIterable<unknown>, call, options);
    }
    if (audit) {
        await recordAudit({ call, outcome: "succeeded" }, audit);
    }

    return result;
}

/** Report stream failures and record declared audit outcomes. */
function streamProcedure<State extends Context>(
    stream: AsyncIterable<unknown>,
    call: ProcedureCall<State>,
    options: Pick<HandlerOptions<State>, "authorize" | "audit">,
): AsyncIteratorClass<unknown> {
    // retain completion separately from consumer cancellation
    const audit = call.access.audit ? options.audit! : undefined;
    let outcome: ProcedureAudit<State>["outcome"] = "cancelled";
    let failure: unknown;
    const iterator = stream[Symbol.asyncIterator]();

    return new AsyncIteratorClass(
        async () => {
            try {
                // recheck long-lived subscriptions before work and immediately before disclosure
                if (options.authorize) {
                    await options.authorize(call);
                }
                const result = await iterator.next();
                if (options.authorize && !result.done) {
                    await options.authorize(call);
                }
                if (result.done) {
                    outcome = "succeeded";
                }

                return result;
            } catch (error) {
                outcome =
                    error instanceof ServiceError && (error.status === 401 || error.status === 403)
                        ? "denied"
                        : "failed";
                failure = error;
                throw reportError(error);
            }
        },
        async (reason) => {
            // release and audit streams even when cancelled before their first value
            try {
                if (reason !== "next" || outcome !== "succeeded") {
                    await iterator.return?.();
                }
            } catch (error) {
                outcome = "failed";
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

/** Record an audit event and preserve any preceding application failure. */
async function recordAudit<State extends Context>(
    event: ProcedureAudit<State>,
    audit: (event: ProcedureAudit<State>) => Promise<void>,
): Promise<void> {
    try {
        await audit(event);
    } catch (error) {
        // retain both failures when audit storage rejects an application failure record
        const failure =
            event.error !== undefined
                ? new AggregateError([event.error, error], "procedure failure audit failed")
                : error;

        throw reportError(failure);
    }
}
