import { v7 } from "uuid";
import { schema } from "@destack/schema";
import type { AuditAction } from "../action/action.ts";
import { AuditEvent, AuditResult } from "../event/event.ts";
import { canonical } from "../event/encode.ts";
import { AuditContext } from "../event/context.ts";
import { AuditError } from "../error/index.ts";

/** Durable append provided by the host or an application transaction. */
export interface AuditWriter<Transaction = never> {
    /** Acknowledge only after durable persistence, or inclusion in the caller's transaction. */
    append(event: AuditEvent, transaction?: Transaction): Promise<void>;
}

/** Record typed actions under host-supplied authority. */
export class AuditRecorder<Transaction = never> {
    /** Immutable authority captured for this invocation. */
    readonly #context: AuditContext;
    /** Durable append used for external effects. */
    readonly #writer: AuditWriter<Transaction>;

    /** Bind trusted context to durable storage. */
    constructor(context: AuditContext, writer: AuditWriter<Transaction>) {
        this.#context = structuredClone(AuditContext.parse(context));
        this.#writer = writer;
    }

    /** Record a completed action through the caller's transaction-bound writer. */
    async record<Targets extends schema.Schema, Details extends schema.Schema>(
        transaction: Transaction,
        action: AuditAction<Targets, Details>,
        values: { targets: schema.Input<Targets>; details: schema.Input<Details> } & AuditResult,
    ): Promise<AuditEvent> {
        const { targets, details, ...result } = values;
        const event = this.#event(
            action,
            { targets, details },
            { stage: "result", ...AuditResult.parse(result) },
        );
        await this.#writer.append(event, transaction);

        return event;
    }

    /** Prepare an attempt for persistence before an external effect. */
    begin<Targets extends schema.Schema, Details extends schema.Schema>(
        action: AuditAction<Targets, Details>,
        values: { targets: schema.Input<Targets>; details: schema.Input<Details> },
    ): AuditEvent {
        return this.#event(action, values, { stage: "attempt" });
    }

    /** Prepare the observed result once, then append the same event on every persistence retry. */
    complete(attempt: AuditEvent, result: AuditResult): AuditEvent {
        // require an actual attempt from this recorder's authority
        attempt = AuditEvent.parse(attempt);
        if (
            attempt.result.stage !== "attempt" ||
            canonical(attempt.context) !== canonical(this.#context)
        ) {
            throw new AuditError(
                "INVALID_EVENT",
                "audit completion requires an attempt from this context",
            );
        }
        const event = AuditEvent.parse({
            ...attempt,
            id: `audit-event-${v7()}`,
            attemptId: attempt.id,
            occurredAt: Date.now(),
            result: { stage: "result", ...AuditResult.parse(result) },
        });
        return event;
    }

    /** Persist a prepared event under this recorder's verified authority. */
    async append(event: AuditEvent): Promise<void> {
        if (canonical(event.context) !== canonical(this.#context)) {
            throw new AuditError("INVALID_EVENT", "audit event belongs to another context");
        }
        await this.#writer.append(event);
    }

    /** Validate declared fields and assign immutable event identity. */
    #event<Targets extends schema.Schema, Details extends schema.Schema>(
        action: AuditAction<Targets, Details>,
        values: { targets: schema.Input<Targets>; details: schema.Input<Details> },
        result: AuditEvent["result"],
    ): AuditEvent {
        const id = `audit-event-${v7()}`;

        return AuditEvent.parse({
            id,
            action: { name: action.name, package: action.package, version: action.version },
            occurredAt: Date.now(),
            context: this.#context,
            targets: action.targets.parse(values.targets),
            details: action.details.parse(values.details),
            result,
        });
    }
}
