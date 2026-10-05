import { AuditCaller } from "./actor.ts";
import { AuditCall } from "./call.ts";
import { AuditContext } from "./context.ts";
import { AuditExecution } from "./execution.ts";
import { v7 } from "uuid";
import { schema, canonicalize, JsonValue, type JsonObject } from "@destack/schema";
import { Failure, Outcome, Subject } from "@destack/sync";
import { denialOf, isServiceError } from "@destack/service";
import type { Authentication } from "@destack/service/authentication";
import {
    domainFailure,
    type ProcedureAudit,
    type ProcedureCall,
    type ServiceContext,
} from "@destack/service/server";
import { context, trace, isSpanContextValid } from "@destack/telemetry";
import type { AuditAction } from "../declare/action.ts";
import { AuditError } from "../error/index.ts";
import { invokeService } from "./action.ts";

/** The host-selected origin of calls. */
export type AuditOrigin = Omit<AuditContext, "caller" | "deploymentId" | "traceId">;

/** The durable store of recorded calls. */
export interface AuditWriter<Transaction = never> {
    /** Record a call, or its outcome once it ends, inside a transaction when given. */
    record(
        call: AuditCall,
        options: {
            readonly isAudited: boolean;
            readonly caller?: string;
            readonly position?: number;
        },
        transaction?: Transaction,
    ): Promise<void>;
}

/** The request a recorded call belongs to, and what a retry replays. */
export interface CallRequest {
    /** The request identifier retries repeat. */
    readonly requestId: string;
    /** The caller a retry repeats the request as. */
    readonly caller: string;
    /** The call's place in its request. */
    readonly position: number;
    /** The digest of the request's input. */
    readonly digest: string;
    /** The call's input, sensitive values redacted. */
    readonly input?: JsonObject;
    /** The release of the method's package the caller made the call against. */
    readonly release?: string;
    /** The transaction identity of the call's logged changes. */
    readonly transaction?: string;
}

/** The ended call values a recorder takes. */
type Ended<Targets extends schema.Schema, Details extends schema.Schema<JsonValue>> = {
    /** The named affected objects. */
    readonly targets: schema.Input<Targets>;
    /** The details the action schema accepts. */
    readonly details: schema.Input<Details>;
    /** How the call ended. */
    readonly outcome: Outcome;
};

/** The outcome of a cancelled call. */
const CANCELLED: Outcome = {
    kind: "cancelled",
    error: { code: "CANCELLED", status: 499, message: "cancelled" },
};

/** Record calls under a fixed context. */
export class AuditRecorder<Transaction = never> {
    /** The context of every call. */
    readonly #context: AuditContext;
    /** The request every call belongs to, absent outside a request. */
    readonly #requestId: string | undefined;
    /** The writer calls go to. */
    readonly #writer: AuditWriter<Transaction>;

    /** Record under a verified caller, or none, with the active trace. */
    static from<Transaction>(
        caller: Authentication | null,
        writer: AuditWriter<Transaction>,
        origin: AuditOrigin,
        requestId?: string,
    ): AuditRecorder<Transaction> {
        // record the represented subject and its delegates, and read who acted
        const claims = caller?.claims;
        const recorded: AuditCaller =
            claims === undefined
                ? { type: "anonymous" }
                : {
                      type: "subject",
                      subject: claims.subject,
                      ...(claims.delegates === undefined ? {} : { delegates: claims.delegates }),
                  };
        const actor = AuditCaller.actor(recorded);
        const acting = actor.type === "subject" ? actor.subject : undefined;

        // select the acting principal's deployment, and the session or token the caller presented
        const deploymentId = acting
            ? claims?.deployments?.find((entry) => Subject.same(entry.subject, acting))?.id
            : undefined;
        const presented = caller?.credential.id;
        const session = schema.identifier("session").safeParse(presented);
        const token = schema.identifier("token").safeParse(presented);
        const span = trace.getSpanContext(context.active());

        // read the request's session, token and trace
        const sessionId = session.success ? session.data : origin.sessionId;
        const deviceId = caller?.credential.device ?? origin.deviceId;
        const tokenId = token.success ? token.data : origin.tokenId;
        const traceId = span && isSpanContextValid(span) ? span.traceId : undefined;

        return new AuditRecorder(
            {
                ...origin,
                caller: recorded,
                ...(deploymentId === undefined ? {} : { deploymentId }),
                ...(deviceId === undefined ? {} : { deviceId }),
                ...(sessionId === undefined ? {} : { sessionId }),
                ...(tokenId === undefined ? {} : { tokenId }),
                ...(traceId === undefined ? {} : { traceId }),
            },
            writer,
            requestId,
        );
    }

    /** Open a recorder of a service's calls in a scope, for a request's caller or the service itself. */
    static service<Transaction>(
        writer: AuditWriter<Transaction>,
        origin: Omit<AuditOrigin, "scope">,
    ): (scope: string, context?: ServiceContext) => AuditRecorder<Transaction> {
        return (scope, request) =>
            request === undefined
                ? AuditRecorder.system(writer, { ...origin, scope })
                : AuditRecorder.from(
                      request.authenticationError === undefined ? request.authentication : null,
                      writer,
                      { ...origin, scope },
                      request.requestId,
                  );
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
                caller: { type: "system", name: origin.service },
                ...(span && isSpanContextValid(span) ? { traceId: span.traceId } : {}),
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

            // map how the procedure ended to an outcome, recording a concealed denial's own code
            const outcome: Outcome =
                event.outcome === "success"
                    ? { kind: "success" }
                    : event.outcome === "cancelled"
                      ? CANCELLED
                      : AuditRecorder.outcome(event.error);

            // record the call
            const writer = await recorder(event.call);
            await writer.record(
                undefined,
                invokeService,
                {
                    targets: { procedure: { type: "procedure", id: event.call.path.join(".") } },
                    details: { authentication: access.authentication },
                    outcome,
                },
                category,
            );
        };
    }

    /** Read the outcome a failed call ends with: a denial for rejected access, a failure otherwise. */
    static outcome(error: unknown): Exclude<Outcome, { kind: "success" }> {
        // map domain failures to service failures
        const known = domainFailure(error) ?? error;
        const denial = isServiceError(known) ? denialOf(known) : undefined;

        // report rejected access as a denial
        if (denial !== undefined) {
            return { kind: "denied", error: Failure.of(denial) };
        }
        // report a service or audit failure
        else if (isServiceError(known) || known instanceof AuditError) {
            return { kind: "failure", error: Failure.of(known) };
        }

        return {
            kind: "failure",
            error: { code: "INTERNAL_SERVER_ERROR", status: 500, message: "internal error" },
        };
    }

    /** Create the recorder. */
    constructor(origin: AuditContext, writer: AuditWriter<Transaction>, requestId?: string) {
        this.#context = structuredClone(AuditContext.parse(origin));
        this.#requestId = requestId;
        this.#writer = writer;
    }

    /** Record an ended call, a committed write unless named otherwise. */
    async record<Targets extends schema.Schema, Details extends schema.Schema<JsonValue>>(
        transaction: Transaction | undefined,
        action: AuditAction<Targets, Details>,
        values: Ended<Targets, Details>,
        category: AuditExecution["category"] = "activity",
        request?: CallRequest,
    ): Promise<AuditCall> {
        return this.#ended(transaction, action, values, category, true, request);
    }

    /** Record an executed call only retries read, such as an ephemeral write no history receives. */
    async keep<Targets extends schema.Schema, Details extends schema.Schema<JsonValue>>(
        transaction: Transaction | undefined,
        action: AuditAction<Targets, Details>,
        values: Ended<Targets, Details>,
        request: CallRequest,
    ): Promise<AuditCall> {
        return this.#ended(transaction, action, values, "activity", false, request);
    }

    /** Prepare a running call, recorded before its external effect runs. */
    begin<Targets extends schema.Schema, Details extends schema.Schema<JsonValue>>(
        action: AuditAction<Targets, Details>,
        values: Omit<Ended<Targets, Details>, "outcome">,
    ): AuditCall {
        return this.#call(action, values, "activity", { startedAt: Date.now() });
    }

    /** Prepare the end of a running call, merging any result details. */
    finish(running: AuditCall, outcome: Outcome, details?: JsonObject): AuditCall {
        // require a running call of this context
        const call = AuditCall.parse(running);
        const execution = call.execution;
        if (
            execution.outcome !== undefined ||
            canonicalize(execution.context) !== canonicalize(this.#context)
        ) {
            throw new AuditError(
                "INVALID_EVENT",
                "finishing a call requires a running call from this context",
            );
        }

        return AuditCall.parse({
            ...call,
            execution: {
                ...execution,
                details:
                    details === undefined
                        ? execution.details
                        : mergeDetails(execution.details, details),
                outcome,
                finishedAt: Date.now(),
            },
        });
    }

    /** Record a prepared call of this context, running or ended. */
    async append(call: AuditCall): Promise<void> {
        // require this recorder's context
        if (canonicalize(call.execution.context) !== canonicalize(this.#context)) {
            throw new AuditError("INVALID_EVENT", "call belongs to another context");
        }

        await this.#writer.record(call, { isAudited: true });
    }

    /** Record a running call, run an external effect, and record how it ended. */
    async attempt<Targets extends schema.Schema, Details extends schema.Schema<JsonValue>, Value>(
        action: AuditAction<Targets, Details>,
        values: Omit<Ended<Targets, Details>, "outcome">,
        execute: () => Promise<Value>,
    ): Promise<Value> {
        // record the call before executing
        const running = this.begin(action, values);
        await this.append(running);

        // record how the execution ended
        let value: Value;
        try {
            value = await execute();
        } catch (error) {
            await this.#persist(error, () =>
                this.append(this.finish(running, AuditRecorder.outcome(error))),
            );
            throw error;
        }
        await this.append(this.finish(running, { kind: "success" }));

        return value;
    }

    /** Run a read and record it as one access call, with the details its result holds. */
    async read<Targets extends schema.Schema, Details extends schema.Schema<JsonValue>, Value>(
        action: AuditAction<Targets, Details>,
        values: Omit<Ended<Targets, Details>, "outcome">,
        execute: () => Promise<Value>,
        detail?: (value: Value) => schema.Input<Details>,
    ): Promise<Value> {
        // record a failed read, leaving denials to the procedure layer
        let value: Value;
        try {
            value = await execute();
        } catch (error) {
            const outcome = AuditRecorder.outcome(error);
            if (outcome.kind !== "denied") {
                await this.#persist(error, () =>
                    this.record(undefined, action, { ...values, outcome }, "access"),
                );
            }
            throw error;
        }

        // record the read before disclosing its result
        const details = detail === undefined ? values.details : detail(value);
        await this.record(
            undefined,
            action,
            { targets: values.targets, details, outcome: { kind: "success" } },
            "access",
        );

        return value;
    }

    /** Pass on a stream's values and record it as one access call when it ends. */
    async *stream<Targets extends schema.Schema, Details extends schema.Schema<JsonValue>, Value>(
        action: AuditAction<Targets, Details>,
        values: Omit<Ended<Targets, Details>, "outcome">,
        source: () => AsyncIterable<Value>,
    ): AsyncGenerator<Value> {
        // count the stream cancelled until it ends
        let outcome: Outcome = CANCELLED;
        let cause: unknown;

        // pass on the values and record how the stream ended
        try {
            yield* source();
            outcome = { kind: "success" };
        } catch (error) {
            cause = error;
            outcome = AuditRecorder.outcome(error);
            throw error;
        } finally {
            // leave a denial to the procedure that refused it
            if (outcome.kind !== "denied") {
                await this.#persist(cause, () =>
                    this.record(undefined, action, { ...values, outcome }, "access"),
                );
            }
        }
    }

    /** Build and write an ended call. */
    async #ended<Targets extends schema.Schema, Details extends schema.Schema<JsonValue>>(
        transaction: Transaction | undefined,
        action: AuditAction<Targets, Details>,
        values: Ended<Targets, Details>,
        category: AuditExecution["category"],
        isAudited: boolean,
        request?: CallRequest,
    ): Promise<AuditCall> {
        // build the call, ended now
        const now = Date.now();
        const call = this.#call(
            action,
            values,
            category,
            {
                ...(request === undefined ? {} : { digest: request.digest }),
                ...(request?.transaction === undefined ? {} : { transaction: request.transaction }),
                outcome: values.outcome,
                startedAt: now,
                finishedAt: now,
            },
            request,
        );

        // write it, keyed for retries within a request
        await this.#writer.record(
            call,
            {
                isAudited,
                ...(request === undefined
                    ? {}
                    : { caller: request.caller, position: request.position }),
            },
            transaction,
        );

        return call;
    }

    /** Record an outcome, and keep the call's failure beside a failed recording. */
    async #persist(cause: unknown, write: () => Promise<unknown>): Promise<void> {
        try {
            await write();
        } catch (error) {
            if (cause !== undefined) {
                throw new AggregateError([cause, error], "call and its recording failed", {
                    cause: error,
                });
            }
            throw error;
        }
    }

    /** Build a call of this context, redacting sensitive details. */
    #call<Targets extends schema.Schema, Details extends schema.Schema<JsonValue>>(
        action: AuditAction<Targets, Details>,
        values: Omit<Ended<Targets, Details>, "outcome">,
        category: AuditExecution["category"],
        execution: Omit<
            AuditExecution,
            "id" | "requestId" | "category" | "context" | "targets" | "details"
        >,
        request?: CallRequest,
    ): AuditCall {
        return AuditCall.parse({
            method: action.name,
            input: request?.input ?? {},
            release: request?.release ?? action.package.version,
            execution: {
                id: `call-${v7()}`,
                ...((request?.requestId ?? this.#requestId) === undefined
                    ? {}
                    : { requestId: request?.requestId ?? this.#requestId }),
                category,
                context: this.#context,
                targets: action.targets.parse(values.targets),
                details: schema.redact(action.details, action.details.parse(values.details)),
                ...execution,
            },
        });
    }
}

/** The recorder methods a procedure's audit uses. */
type ProcedureRecorder = Pick<AuditRecorder<unknown>, "record">;

/** Merge result details into a call's recorded object details. */
function mergeDetails(recorded: JsonValue, details: JsonObject): JsonObject {
    if (!JsonValue.isObject(recorded)) {
        throw new AuditError("INVALID_EVENT", "result details merge only into object details");
    }

    return { ...recorded, ...details };
}
