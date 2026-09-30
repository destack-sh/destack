import { v7 } from "uuid";
import { schema } from "@destack/schema";
import type { AuditAction } from "../action/action.ts";
import { AuditEvent, AuditResult } from "../event/event.ts";
import { canonicalize } from "@destack/schema/json";
import { AuditContext } from "../event/context.ts";
import { AuditError } from "../error/index.ts";
import { denialOf, ServiceError } from "@destack/service";
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

    /** Record a procedure call after it ends: writes, audited reads and denials. */
    static procedure<State extends object>(
        recorder: (call: ProcedureCall<State>) => ProcedureRecorder | Promise<ProcedureRecorder>,
        options: { readonly isAccessAudited?: boolean } = {},
    ): (event: ProcedureAudit<State>) => Promise<void> {
        return async (event) => {
            // choose the category, skipping unaudited reads
            const { access } = event.call;
            const category =
                event.outcome === "denied"
                    ? "denial"
                    : access.audit === "activity" ||
                        (access.audit === "access" && options.isAccessAudited === true)
                      ? access.audit
                      : undefined;
            if (category === undefined) {
                return;
            }

            // map the outcome and failure code to the result, recording a concealed denial's own code
            const errorCode =
                event.error instanceof ServiceError
                    ? (denialOf(event.error) ?? event.error).code
                    : event.outcome === "cancelled"
                      ? "CANCELLED"
                      : "INTERNAL_SERVER_ERROR";
            const result: AuditResult =
                event.outcome === "success"
                    ? { outcome: "success" }
                    : { outcome: event.outcome, errorCode };

            // record the call as one event
            const writer = await recorder(event.call);
            await writer.record(
                undefined,
                invokeService,
                {
                    targets: { procedure: { type: "procedure", id: event.call.path.join(".") } },
                    details: { authentication: access.authentication },
                    ...result,
                },
                category,
            );
        };
    }

    /** Read the result a failed action ends with: a denial for rejected access, a failure otherwise. */
    static result(error: unknown): AuditResult {
        return resultOf(error);
    }

    /** Create the recorder. */
    constructor(context: AuditContext, writer: AuditWriter<Transaction>) {
        this.#context = structuredClone(AuditContext.parse(context));
        this.#writer = writer;
    }

    /** Record a completed action, a committed write unless named otherwise. */
    async record<Targets extends schema.Schema, Details extends schema.Schema>(
        transaction: Transaction | undefined,
        action: AuditAction<Targets, Details>,
        values: { targets: schema.Input<Targets>; details: schema.Input<Details> } & AuditResult,
        category: AuditEvent["category"] = "activity",
    ): Promise<AuditEvent> {
        // append the result event
        const { targets, details, ...result } = values;
        const event = this.#event(
            action,
            { targets, details },
            { stage: "result", ...AuditResult.parse(result) },
            category,
        );
        await this.#writer.append(event, transaction);

        return event;
    }

    /** Prepare an attempt. */
    begin<Targets extends schema.Schema, Details extends schema.Schema>(
        action: AuditAction<Targets, Details>,
        values: { targets: schema.Input<Targets>; details: schema.Input<Details> },
    ): AuditEvent {
        return this.#event(action, values, { stage: "attempt" }, "activity");
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

    /** Persist an attempt, run an external effect, and persist its result. */
    async attempt<Targets extends schema.Schema, Details extends schema.Schema, Value>(
        action: AuditAction<Targets, Details>,
        values: { targets: schema.Input<Targets>; details: schema.Input<Details> },
        execute: () => Promise<Value>,
    ): Promise<Value> {
        // persist the attempt before executing
        const attempt = this.begin(action, values);
        await this.append(attempt);

        // persist the result of the execution
        let value: Value;
        try {
            value = await execute();
        } catch (error) {
            await this.#keep(error, () => this.append(this.complete(attempt, resultOf(error))));
            throw error;
        }
        await this.append(this.complete(attempt, { outcome: "success" }));

        return value;
    }

    /** Run a read and record it as one access event, with the details its result names. */
    async read<Targets extends schema.Schema, Details extends schema.Schema, Value>(
        action: AuditAction<Targets, Details>,
        values: { targets: schema.Input<Targets>; details: schema.Input<Details> },
        execute: () => Promise<Value>,
        detail?: (value: Value) => schema.Input<Details>,
    ): Promise<Value> {
        // record a failed read, leaving denials to the procedure layer
        let value: Value;
        try {
            value = await execute();
        } catch (error) {
            const result = resultOf(error);
            if (result.outcome !== "denied") {
                await this.#keep(error, () =>
                    this.record(undefined, action, { ...values, ...result }, "access"),
                );
            }
            throw error;
        }

        // record the read before disclosing its result
        const details = detail === undefined ? values.details : detail(value);
        await this.record(
            undefined,
            action,
            { targets: values.targets, details, outcome: "success" },
            "access",
        );

        return value;
    }

    /** Pass on a stream's values and record it as one access event when it ends. */
    async *stream<Targets extends schema.Schema, Details extends schema.Schema, Value>(
        action: AuditAction<Targets, Details>,
        values: { targets: schema.Input<Targets>; details: schema.Input<Details> },
        source: () => AsyncIterable<Value>,
    ): AsyncGenerator<Value> {
        // count the stream cancelled until it ends
        let result: AuditResult = { outcome: "cancelled", errorCode: "CANCELLED" };
        let cause: unknown;

        // pass on the values, then record how the stream ended
        try {
            yield* source();
            result = { outcome: "success" };
        } catch (error) {
            cause = error;
            result = resultOf(error);
            throw error;
        } finally {
            // leave a denial to the procedure that refused it
            if (result.outcome !== "denied") {
                await this.#keep(cause, () =>
                    this.record(undefined, action, { ...values, ...result }, "access"),
                );
            }
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

    /** Persist a result and keep the action's failure beside a failed write. */
    async #keep(cause: unknown, persist: () => Promise<unknown>): Promise<void> {
        try {
            await persist();
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
        category: AuditEvent["category"],
    ): AuditEvent {
        return AuditEvent.parse({
            id: `audit-event-${v7()}`,
            action: { name: action.name, package: action.package },
            category,
            occurredAt: Date.now(),
            context: this.#context,
            targets: action.targets.parse(values.targets),
            details: schema.redact(action.details, action.details.parse(values.details)),
            result,
        });
    }
}

/** The recorder methods a procedure's audit uses. */
type ProcedureRecorder = Pick<AuditRecorder<unknown>, "record">;

/** Map an error to a result. */
function resultOf(error: unknown): AuditResult {
    // map domain failures to service failures
    const known = domainFailure(error) ?? error;
    const denial = known instanceof ServiceError ? denialOf(known) : undefined;

    // report rejected access, concealed or not, as a denial with its own code
    if (denial !== undefined) {
        return { outcome: "denied", errorCode: denial.code };
    } else if (known instanceof ServiceError || known instanceof AuditError) {
        return { outcome: "failure", errorCode: known.code };
    }

    return { outcome: "failure", errorCode: "INTERNAL_SERVER_ERROR" };
}

/** Name a verified subject as the actor it records. */
function actorOf(subject: Subject): AuditActor {
    return { type: "subject", subject };
}
