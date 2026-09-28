import { v7 } from "uuid";
import { schema } from "@destack/schema";
import type { AuditAction } from "../action/action.ts";
import { AuditEvent, AuditResult } from "../event/event.ts";
import { canonicalize } from "@destack/schema/json";
import { AuditContext } from "../event/context.ts";
import { AuditError } from "../error/index.ts";
import { ServiceError } from "@destack/service";
import type { Caller } from "@destack/service/authentication";
import {
    domainFailure,
    type ProcedureAudit,
    type ProcedureCall,
    type ServiceContext,
} from "@destack/service/server";
import { sameSubject, type Subject } from "@destack/access";
import { context, trace, isSpanContextValid } from "@destack/telemetry";
import type { AuditActor } from "../event/actor.ts";
import { invokeService } from "./action.ts";

/** The host-selected origin of events. */
export type AuditOrigin = Omit<
    AuditContext,
    "actor" | "subject" | "delegation" | "deploymentId" | "traceId"
>;

/** The durable store of recorded events. */
export interface AuditWriter<Transaction = never> {
    /** Append an event, inside a transaction when given. */
    append(event: AuditEvent, transaction?: Transaction): Promise<void>;
}

/** Record actions under a fixed context. */
export class AuditRecorder<Transaction = never> {
    /** The context of every event. */
    readonly #context: AuditContext;
    /** The writer events go to. */
    readonly #writer: AuditWriter<Transaction>;

    /** Record under a verified caller, or none, with the active trace. */
    static from<Transaction>(
        caller: Caller | null,
        writer: AuditWriter<Transaction>,
        origin: AuditOrigin,
    ): AuditRecorder<Transaction> {
        // split the caller into subject, delegation and acting principal
        const authentication = caller?.authentication;
        const delegates = (authentication?.delegates ?? []).map((delegate) => delegate.subject);
        const acting = delegates.at(-1) ?? authentication?.subject;
        const delegation =
            authentication && delegates.length > 0
                ? [authentication.subject, ...delegates.slice(0, -1)]
                : [];

        // select the acting principal's deployment
        const deploymentId = acting
            ? authentication?.deployments?.find((entry) => sameSubject(entry.subject, acting))?.id
            : undefined;
        const span = trace.getSpanContext(context.active());

        return new AuditRecorder(
            {
                ...origin,
                actor: acting ? actorOf(acting) : { type: "anonymous" },
                subject: authentication?.subject,
                delegation: delegation.map(actorOf),
                deploymentId,
                traceId: span && isSpanContextValid(span) ? span.traceId : undefined,
            },
            writer,
        );
    }

    /** Record a service's calls in the verified caller's scope, or the service's own. */
    static service<Transaction>(
        writer: AuditWriter<Transaction>,
        origin: Omit<AuditOrigin, "scope" | "requestId">,
    ): (scope: string, context?: ServiceContext) => AuditRecorder<Transaction> {
        return (scope, context) =>
            context === undefined
                ? AuditRecorder.system(writer, { ...origin, scope })
                : AuditRecorder.from(context.caller, writer, {
                      ...origin,
                      scope,
                      requestId: context.requestId,
                  });
    }

    /** Record as the service itself, with the active trace. */
    static system<Transaction>(
        writer: AuditWriter<Transaction>,
        origin: AuditOrigin,
    ): AuditRecorder<Transaction> {
        const span = trace.getSpanContext(context.active());

        return new AuditRecorder(
            {
                ...origin,
                actor: { type: "system", name: origin.service },
                delegation: [],
                traceId: span && isSpanContextValid(span) ? span.traceId : undefined,
            },
            writer,
        );
    }

    /** Record each procedure call as an attempt and its result. */
    static procedure<State extends object>(
        recorder: (call: ProcedureCall<State>) => ProcedureRecorder | Promise<ProcedureRecorder>,
    ): (event: ProcedureAudit<State>) => Promise<void> {
        // keep each call's attempt until its result
        const attempts = new WeakMap<
            ProcedureCall<State>,
            { recorder: ProcedureRecorder; event: AuditEvent }
        >();

        return async (event) => {
            // persist the attempt when the procedure starts
            if (event.outcome === "started") {
                const writer = await recorder(event.call);
                const attempt = writer.begin(invokeService, {
                    targets: { procedure: { type: "procedure", id: event.call.path.join(".") } },
                    details: { authentication: event.call.access.authentication },
                });
                await writer.append(attempt);
                attempts.set(event.call, { recorder: writer, event: attempt });
            }
            // complete the attempt with the failure code only
            else {
                const attempt = attempts.get(event.call);
                if (!attempt) {
                    throw new AuditError(
                        "INVALID_EVENT",
                        "audit completion has no recorded procedure attempt",
                    );
                }

                // map the outcome and failure code to the result
                const errorCode =
                    event.error instanceof ServiceError
                        ? event.error.code
                        : event.outcome === "cancelled"
                          ? "CANCELLED"
                          : "INTERNAL_SERVER_ERROR";
                const result: AuditResult =
                    event.outcome === "success"
                        ? { outcome: "success" }
                        : { outcome: event.outcome, errorCode };

                // persist the result and release the attempt
                await attempt.recorder.append(attempt.recorder.complete(attempt.event, result));
                attempts.delete(event.call);
            }
        };
    }

    /** Create the recorder. */
    constructor(context: AuditContext, writer: AuditWriter<Transaction>) {
        this.#context = structuredClone(AuditContext.parse(context));
        this.#writer = writer;
    }

    /** Record a completed action. */
    async record<Targets extends schema.Schema, Details extends schema.Schema>(
        transaction: Transaction,
        action: AuditAction<Targets, Details>,
        values: { targets: schema.Input<Targets>; details: schema.Input<Details> } & AuditResult,
    ): Promise<AuditEvent> {
        // append the result event
        const { targets, details, ...result } = values;
        const event = this.#event(
            action,
            { targets, details },
            { stage: "result", ...AuditResult.parse(result) },
        );
        await this.#writer.append(event, transaction);

        return event;
    }

    /** Prepare an attempt. */
    begin<Targets extends schema.Schema, Details extends schema.Schema>(
        action: AuditAction<Targets, Details>,
        values: { targets: schema.Input<Targets>; details: schema.Input<Details> },
    ): AuditEvent {
        return this.#event(action, values, { stage: "attempt" });
    }

    /** Prepare the result of an attempt, merging any result details. */
    complete(
        attempt: AuditEvent,
        result: AuditResult,
        details?: Readonly<Record<string, unknown>>,
    ): AuditEvent {
        // require an attempt of this context
        attempt = AuditEvent.parse(attempt);
        if (
            attempt.result.stage !== "attempt" ||
            canonicalize(attempt.context) !== canonicalize(this.#context)
        ) {
            throw new AuditError(
                "INVALID_EVENT",
                "audit completion requires an attempt from this context",
            );
        }

        return AuditEvent.parse({
            ...attempt,
            id: `audit-event-${v7()}`,
            attemptId: attempt.id,
            occurredAt: Date.now(),
            ...(details === undefined
                ? {}
                : { details: { ...(attempt.details as Record<string, unknown>), ...details } }),
            result: { stage: "result", ...AuditResult.parse(result) },
        });
    }

    /** Persist an attempt, run the action, and persist its result. */
    async attempt<Targets extends schema.Schema, Details extends schema.Schema, Value>(
        action: AuditAction<Targets, Details>,
        values: { targets: schema.Input<Targets>; details: schema.Input<Details> },
        execute: () => Promise<Value>,
        detail?: (value: Value) => schema.Input<Details>,
    ): Promise<Value> {
        // persist the attempt before executing
        const attempt = this.begin(action, values);
        await this.append(attempt);

        // persist the result of the execution
        let value: Value;
        try {
            value = await execute();
        } catch (error) {
            await this.#conclude(attempt, resultOf(error), error);
            throw error;
        }
        const details =
            detail === undefined
                ? undefined
                : (schema.redact(action.details, action.details.parse(detail(value))) as Readonly<
                      Record<string, unknown>
                  >);
        await this.#conclude(attempt, { outcome: "success" }, undefined, details);

        return value;
    }

    /** Persist an attempt, pass on a stream's values, and persist its result when it ends. */
    async *stream<Targets extends schema.Schema, Details extends schema.Schema, Value>(
        action: AuditAction<Targets, Details>,
        values: { targets: schema.Input<Targets>; details: schema.Input<Details> },
        source: () => AsyncIterable<Value>,
    ): AsyncGenerator<Value> {
        // persist the attempt, cancelled until the stream ends
        const attempt = this.begin(action, values);
        await this.append(attempt);
        let result: AuditResult = { outcome: "cancelled", errorCode: "CANCELLED" };
        let cause: unknown;

        // pass on the values, then persist how the stream ended
        try {
            yield* source();
            result = { outcome: "success" };
        } catch (error) {
            cause = error;
            result = resultOf(error);
            throw error;
        } finally {
            await this.#conclude(attempt, result, cause);
        }
    }

    /** Append a prepared event of this context. */
    async append(event: AuditEvent): Promise<void> {
        // require this recorder's context
        if (canonicalize(event.context) !== canonicalize(this.#context)) {
            throw new AuditError("INVALID_EVENT", "audit event belongs to another context");
        }

        // append through the writer
        await this.#writer.append(event);
    }

    /** Persist an attempt's result, keeping the action's failure. */
    async #conclude(
        attempt: AuditEvent,
        result: AuditResult,
        cause?: unknown,
        details?: Readonly<Record<string, unknown>>,
    ): Promise<void> {
        // append the result, keeping both failures
        try {
            await this.append(this.complete(attempt, result, details));
        } catch (error) {
            if (cause !== undefined) {
                throw new AggregateError(
                    [cause, error],
                    "audit action and result recording failed",
                );
            }
            throw error;
        }
    }

    /** Build an event, redacting sensitive details. */
    #event<Targets extends schema.Schema, Details extends schema.Schema>(
        action: AuditAction<Targets, Details>,
        values: { targets: schema.Input<Targets>; details: schema.Input<Details> },
        result: AuditEvent["result"],
    ): AuditEvent {
        return AuditEvent.parse({
            id: `audit-event-${v7()}`,
            action: { name: action.name, package: action.package, version: action.version },
            occurredAt: Date.now(),
            context: this.#context,
            targets: action.targets.parse(values.targets),
            details: schema.redact(action.details, action.details.parse(values.details)),
            result,
        });
    }
}

/** The recorder methods a procedure's audit uses. */
type ProcedureRecorder = Pick<AuditRecorder<unknown>, "begin" | "complete" | "append">;

/** Map an error to a result. */
function resultOf(error: unknown): AuditResult {
    // map domain failures to service failures
    const known = domainFailure(error) ?? error;
    const errorCode =
        known instanceof ServiceError || known instanceof AuditError
            ? known.code
            : "INTERNAL_SERVER_ERROR";

    // report rejected access as a denial
    const outcome =
        errorCode === "FORBIDDEN" || errorCode === "UNAUTHORIZED" ? "denied" : "failure";

    return { outcome, errorCode };
}

/** Name a verified subject as the actor it records. */
function actorOf(subject: Subject): AuditActor {
    return { type: "subject", subject };
}
