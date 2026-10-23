import { AuditCaller } from "./actor.ts";
import type { CallEvent, JournalEntry } from "./execution.ts";
import { CallOrigin } from "./origin.ts";
import { AuditExecution, type AuditTarget } from "./execution.ts";
import {
    canonicalize,
    Identifier,
    type JsonObject,
    JsonValue,
    present,
    schema,
} from "@destack/schema";
import { Outcome } from "@destack/sync";
import { Failure } from "@destack/error";
import { findDenial, CallError } from "../error/call.ts";
import { AccessContext, Credential } from "@destack/access";
import { type ProcedureAudit, type ProcedureContext } from "../server/procedure.ts";
import { context, trace, isSpanContextValid } from "@destack/telemetry";
import { type AuditAction, defineAuditAction } from "./action.ts";
import type { Package } from "@destack/package";
import { ObjectError } from "../error/index.ts";
import type { DatabaseConnection } from "@destack/db";
import type { Journal } from "./journal.ts";

/** Invoke a procedure. */
export const procedureInvoke = defineAuditAction({
    name: "procedure.invoke",
    target: schema.object({ type: schema.literal("procedure"), id: schema.string().min(1) }),
    details: schema.object({ authentication: schema.enum(["public", "identity", "machine"]) }),
});

/** The machine-selected origin of calls. */
export type AuditOrigin = Omit<CallOrigin, "caller" | "deploymentId" | "traceId">;

/** The options of a recorder: what it records about the calls beside their caller. */
export interface AuditRecorderOptions {
    /** The package whose method or procedure the calls run. */
    readonly package: Package;
    /** The request the calls answer. */
    readonly requestId?: string;
    /** The code making the calls, such as a controller's name. */
    readonly component?: string;
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
type Ended<Target extends schema.Schema<AuditTarget>, Details extends schema.Schema<JsonValue>> = {
    /** The object the call acts on. */
    readonly target: schema.Input<Target>;
    /** The details the action schema accepts. */
    readonly details: schema.Input<Details>;
    /** How the call ended. */
    readonly outcome: Outcome;
};

/** The outcome of a cancelled call. */
const CANCELLED: Outcome = {
    kind: "cancelled",
    error: Failure.http("CLIENT_CLOSED_REQUEST", "cancelled"),
};

/** Record calls under a fixed context. */
export class AuditRecorder {
    /** The context of every call. */
    readonly #context: CallOrigin;
    /** The request every call belongs to, absent outside a request. */
    readonly #requestId: string | undefined;
    /** The journal calls go to. */
    readonly #journal: Journal;

    /** Record a caller's calls. */
    static from(
        caller: AccessContext,
        journal: Journal,
        origin: AuditOrigin,
        requestId?: string,
    ): AuditRecorder {
        // record the subject and its actors, and read who acted
        const { subject, actors, credential, grants } = caller;
        const recorded: AuditCaller = {
            subject,
            ...(actors.length === 0 ? {} : { actors: [...actors] }),
        };
        const acting = AuditCaller.actor(recorded).subject;

        // select the deployment the acting workload ran, and the session or token the caller presented
        const deploymentId =
            credential === undefined ? undefined : Credential.deployment(credential, acting);
        const session = schema.identifier("session").safeParse(credential?.id);
        const presented = schema.identifier("token").safeParse(credential?.id);
        const span = trace.getSpanContext(context.active());

        // read the caller's session, token, trace and the grant its sender acts under
        const grant = grants.at(-1);
        const sessionId = session.success ? session.data : origin.sessionId;
        const clientId = credential?.client ?? origin.clientId;
        const tokenId = presented.success ? presented.data : origin.tokenId;
        const traceId = span && isSpanContextValid(span) ? span.traceId : undefined;

        return new AuditRecorder(
            {
                ...origin,
                caller: recorded,
                ...(deploymentId === undefined ? {} : { deploymentId }),
                ...(clientId === undefined ? {} : { clientId }),
                ...(sessionId === undefined ? {} : { sessionId }),
                ...(tokenId === undefined ? {} : { tokenId }),
                ...(traceId === undefined ? {} : { traceId }),
                ...(grant === undefined ? {} : { grantId: grant.id }),
            },
            journal,
            requestId,
        );
    }

    /** Open recorders of a store's calls, one per scope and caller. */
    static scoped(
        journal: Journal,
        origin: Omit<AuditOrigin, "scope" | "package">,
    ): (scope: string, caller: AccessContext, options: AuditRecorderOptions) => AuditRecorder {
        return (scope, caller, options) =>
            AuditRecorder.from(
                caller,
                journal,
                {
                    ...origin,
                    package: options.package,
                    scope,
                    ...(options.component === undefined ? {} : { component: options.component }),
                },
                options.requestId,
            );
    }

    /** Record a procedure call after it ends: writes, audited reads and denials. */
    static procedure<State extends object>(
        recorder: (call: ProcedureContext<State>) => ProcedureRecorder | Promise<ProcedureRecorder>,
        options: { readonly isAccessAudited?: boolean } = {},
    ): (event: ProcedureAudit<State>) => Promise<void> {
        return async (event) => {
            // choose the category, skipping unaudited reads
            const { access } = event.context;
            const category = AuditRecorder.#categoryOf(event, options.isAccessAudited === true);
            if (category === undefined) {
                return;
            }

            // map how the procedure ended to an outcome, recording a concealed denial's code
            const outcome = AuditRecorder.#outcomeOf(event);

            // record the call
            const writer = await recorder(event.context);
            await writer.record(
                undefined,
                procedureInvoke,
                {
                    target: { type: "procedure", id: event.context.path.join(".") },
                    details: { authentication: access.authentication },
                    outcome,
                },
                category,
            );
        };
    }

    /** Read the outcome a failed call ends with: a denial for rejected access, else a failure. */
    static outcome(error: unknown): Exclude<Outcome, { kind: "success" }> {
        // read the call error a failure is or has
        const known = CallError.of(error);
        const denial = known === undefined ? undefined : findDenial(known);

        // report rejected access as a denial
        if (denial !== undefined) {
            return { kind: "denied", error: Failure.of(denial) };
        }
        // report a call error
        else if (known !== undefined) {
            return { kind: "failure", error: Failure.of(known) };
        }
        // report any other failure as internal, its message out of the history
        else {
            return {
                kind: "failure",
                error: Failure.http("INTERNAL_SERVER_ERROR", "internal error"),
            };
        }
    }

    /** Create the recorder. */
    constructor(origin: CallOrigin, journal: Journal, requestId?: string) {
        this.#context = CallOrigin.parse(origin);
        this.#requestId = requestId;
        this.#journal = journal;
    }

    /** Record an ended call, a committed write unless the category says otherwise. */
    async record<
        Target extends schema.Schema<AuditTarget>,
        Details extends schema.Schema<JsonValue>,
    >(
        transaction: DatabaseConnection | undefined,
        action: AuditAction<Target, Details>,
        values: Ended<Target, Details>,
        category: AuditExecution["category"] = "activity",
        request?: CallRequest,
    ): Promise<CallEvent> {
        return this.#ended(transaction, action, values, category, true, request);
    }

    /** Record an executed call that only retries read, such as an ephemeral write. */
    async recordRead<
        Target extends schema.Schema<AuditTarget>,
        Details extends schema.Schema<JsonValue>,
    >(
        transaction: DatabaseConnection | undefined,
        action: AuditAction<Target, Details>,
        values: Ended<Target, Details>,
        request: CallRequest,
    ): Promise<CallEvent> {
        return this.#ended(transaction, action, values, "activity", false, request);
    }

    /** Prepare a running call, recorded before its external effect runs. */
    begin<Target extends schema.Schema<AuditTarget>, Details extends schema.Schema<JsonValue>>(
        action: AuditAction<Target, Details>,
        values: Omit<Ended<Target, Details>, "outcome">,
    ): JournalEntry {
        return this.#call(action, values, "activity", { startedAt: this.#journal.clock.now() });
    }

    /** Prepare the end of a running call, merging any result details. */
    finish(running: JournalEntry, outcome: Outcome, details?: JsonObject): CallEvent {
        // require a running call of this context
        const execution = running.execution;
        if (
            execution.outcome !== undefined ||
            canonicalize(execution.context) !== canonicalize(this.#context)
        ) {
            throw new ObjectError("INVALID_CALL", {
                message: "finishing a call requires a running call from this context",
            });
        }

        return {
            ...running,
            execution: {
                ...execution,
                details:
                    details === undefined
                        ? execution.details
                        : AuditRecorder.#mergeDetails(execution.details, details),
                outcome,
                finishedAt: this.#journal.clock.now(),
            },
        };
    }

    /** Record a prepared call of this context, running or ended. */
    async append(call: JournalEntry): Promise<void> {
        // require this recorder's context
        if (canonicalize(call.execution.context) !== canonicalize(this.#context)) {
            throw new ObjectError("INVALID_CALL", { message: "call belongs to another context" });
        }

        await this.#journal.record(call, { isAudited: true });
    }

    /** Record a running call, run an external effect, and record how it ended. */
    async attempt<
        Target extends schema.Schema<AuditTarget>,
        Details extends schema.Schema<JsonValue>,
        Value,
    >(
        action: AuditAction<Target, Details>,
        values: Omit<Ended<Target, Details>, "outcome">,
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

    /** Run a read and record it as one access call, with the details its result has. */
    async read<
        Target extends schema.Schema<AuditTarget>,
        Details extends schema.Schema<JsonValue>,
        Value,
    >(
        action: AuditAction<Target, Details>,
        values: Omit<Ended<Target, Details>, "outcome">,
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
            { target: values.target, details, outcome: { kind: "success" } },
            "access",
        );

        return value;
    }

    /** Pass on a stream's values and record it as one access call when it ends. */
    async *stream<
        Target extends schema.Schema<AuditTarget>,
        Details extends schema.Schema<JsonValue>,
        Value,
    >(
        action: AuditAction<Target, Details>,
        values: Omit<Ended<Target, Details>, "outcome">,
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
    async #ended<
        Target extends schema.Schema<AuditTarget>,
        Details extends schema.Schema<JsonValue>,
    >(
        transaction: DatabaseConnection | undefined,
        action: AuditAction<Target, Details>,
        values: Ended<Target, Details>,
        category: AuditExecution["category"],
        isAudited: boolean,
        request?: CallRequest,
    ): Promise<CallEvent> {
        // build the call, ended now
        const now = this.#journal.clock.now();
        const entry = this.#call(
            action,
            values,
            category,
            {
                ...(request === undefined ? {} : { digest: request.digest }),
                ...(request?.transaction === undefined ? {} : { transaction: request.transaction }),
                startedAt: now,
            },
            request,
        );
        const call: CallEvent = {
            ...entry,
            execution: { ...entry.execution, outcome: values.outcome, finishedAt: now },
        };

        // write it, keyed for retries within a request
        await this.#journal.record(
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

    /** Record an outcome, and the call's failure beside a failed recording. */
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
    #call<Target extends schema.Schema<AuditTarget>, Details extends schema.Schema<JsonValue>>(
        action: AuditAction<Target, Details>,
        values: Omit<Ended<Target, Details>, "outcome">,
        category: AuditExecution["category"],
        execution: Omit<
            AuditExecution,
            "id" | "requestId" | "category" | "context" | "target" | "details"
        >,
        request?: CallRequest,
    ): JournalEntry {
        const requestId = request?.requestId ?? this.#requestId;

        return {
            method: action.name,
            input: request?.input ?? {},
            release: request?.release ?? action.package.version,
            execution: {
                id: Identifier.create("call"),
                ...(requestId === undefined ? {} : { requestId }),
                category,
                context: this.#context,
                target: action.target.parse(values.target),
                details: present(
                    schema.redact(action.details, action.details.parse(values.details)),
                    "the call's details",
                ),
                ...execution,
            },
        };
    }

    /** Choose the category a procedure call is recorded under, absent for a read nobody audits. */
    static #categoryOf<State extends object>(
        event: ProcedureAudit<State>,
        isAccessAudited: boolean,
    ): AuditExecution["category"] | undefined {
        const { audit } = event.context.access;
        // a refused call
        if (event.outcome === "denied") {
            return "denial";
        }
        // a write, or a read when reads are audited
        else if (audit === "activity" || (audit === "access" && isAccessAudited)) {
            return audit;
        }
        // a read nobody audits
        else {
            return undefined;
        }
    }

    /** Map how a procedure call ended to its outcome. */
    static #outcomeOf<State extends object>(event: ProcedureAudit<State>): Outcome {
        // a success
        if (event.outcome === "success") {
            return { kind: "success" };
        }
        // a cancellation
        else if (event.outcome === "cancelled") {
            return CANCELLED;
        }
        // a failure or a denial, with its code
        else {
            return AuditRecorder.outcome(event.error);
        }
    }

    /** Merge result details into a call's recorded object details. */
    static #mergeDetails(recorded: JsonValue, details: JsonObject): JsonObject {
        if (!JsonValue.isObject(recorded)) {
            throw new ObjectError("INVALID_CALL", {
                message: "result details merge only into object details",
            });
        }

        return { ...recorded, ...details };
    }
}

/** The recorder methods a procedure's audit uses. */
type ProcedureRecorder = Pick<AuditRecorder, "record">;
