import { none, through, union } from "@destack/access";
import { type Select } from "@destack/db";
import { EventTime } from "@destack/event";
import { defineObject, type EventOf, field } from "@destack/object";
import { CONCURRENCIES, RunEvent } from "@destack/runtime/trigger";
import { Instant, schema } from "@destack/schema";
import { Call, Failure } from "@destack/sync";
import { ExchangeAsk } from "@destack/runtime/authentication";
import { installation } from "./installation.ts";
import { trigger } from "./trigger.ts";
import { space } from "@destack/account/object";

/** The most run events a segment keeps before it flushes, beside the hour its oldest may wait. */
const MAX_SEGMENT_RUNS = 100_000;

/** Where a live run stands: waiting for an attempt or attempting. */
export const RUN_STATES = ["pending", "running"] as const;

/** How a run finished: succeeded, failed, or skipped without running. */
export const RUN_OUTCOMES = ["succeeded", "failed", "skipped"] as const;

/** How runs of a trigger may overlap: a schedule's rules, and one at a time for changes. */
export const RUN_CONCURRENCIES = [...CONCURRENCIES, "queue"] as const;

/** Why a run ended without running. */
export const RUN_SKIP_REASONS = [
    "missed",
    "late",
    "superseded",
    "overlap",
    "replaced",
    "cancelled",
] as const;

/** Why a run ended without running, and the earlier missed occurrences it stands for. */
export const RunSkip = schema.object({
    /** Why the run was skipped. */
    reason: schema.enum(RUN_SKIP_REASONS),
    /** The earlier occurrences missed before this one, "many" past the recorded limit. */
    earlier: schema
        .union([schema.number().int().positive(), schema.literal("many")])
        .exactOptional(),
});
/** Why a run ended without running. */
export type RunSkip = schema.Infer<typeof RunSkip>;

/** How a run finished: succeeded, failed with the failure ending it, or skipped with why. */
export const RunOutcome = schema.discriminatedUnion("kind", [
    schema.object({
        /** A run whose call succeeded. */
        kind: schema.literal("succeeded"),
    }),
    schema.object({
        /** A run whose call failed. */
        kind: schema.literal("failed"),
        /** The failure ending the run. */
        failure: Failure,
    }),
    schema.object({
        /** A run that ended without running. */
        kind: schema.literal("skipped"),
        /** Why the run was skipped. */
        skip: RunSkip,
    }),
]);
/** How a run finished. */
export type RunOutcome = schema.Infer<typeof RunOutcome>;

/** One object method call an installation runs later, with its attempts. */
export const run = defineObject({
    name: "run",
    plural: "runs",
    scope: space,
    fields: {
        /** The installation making the call. */
        installation: field.reference(installation, { delete: "cascade" }),

        // the call and when it runs
        /** The object method call. */
        call: field.json(Call),
        /** When the call runs, in UTC epoch milliseconds. */
        at: field.time(),
        /** The subject the call runs for under its grant, absent for the installation's own. */
        exchange: field.json(ExchangeAsk).optional(),

        // what recorded it, once
        /** The trigger that fired, absent for a sent call or once the trigger is deleted. */
        trigger: field.reference(trigger, { delete: "null" }).optional(),
        /** The event that fired the trigger, absent for a sent call. */
        event: field.json(RunEvent).optional(),
        /** The request recording the run once: the sender's, or the trigger event's. */
        requestId: field.string(schema.string().min(1)),

        // the execution
        /** Where the run stands. */
        state: field.enum(RUN_STATES),
        /** How runs of the same trigger may overlap. */
        concurrency: field.enum(RUN_CONCURRENCIES),
        /** The attempts started so far. */
        attempts: field.integer(),
        /** The first attempt's start, in UTC epoch milliseconds. */
        startedAt: field.time().optional(),
        /** The failure of the last attempt, absent before one failed. */
        failure: field.json(Failure).optional(),
        /** The trace of the run's attempts. */
        traceId: field.string(schema.string().min(1)).optional(),
    },
    permissions: {
        read: through("installation", "read"),
        send: through("installation", "run"),
        cancel: union(through("installation", "run"), through("installation", "update")),
        record: none(),
    },
    reserved: ["send"],
    constraints: {
        // record each request once per trigger and each sent call once while its run is live
        request: {
            kind: "index",
            on: ["installationId", "triggerId", "requestId"],
            unique: true,
            where: { OR: [{ triggerId: { isNotNull: true } }, { event: { isNull: true } }] },
            nullsNotDistinct: true,
        },

        // claim a trigger refusing overlapping runs for one run at a time across machines
        claim: {
            kind: "index",
            on: ["triggerId"],
            unique: true,
            where: { concurrency: { in: ["forbid", "replace"] } },
        },
        due: { kind: "index", on: ["state", "at"] },
        queue: { kind: "index", on: ["triggerId", "requestId"] },

        // keep the event of every run a trigger fired
        event: {
            kind: "check",
            condition: {
                OR: [{ triggerId: { isNull: true } }, { event: { isNotNull: true } }],
            },
        },
        attempts: { kind: "check", condition: { attempts: { gte: 0 } } },
    },
}).methods((method) => ({
    send: method.create("send", {
        fields: ["installationId", "call"],
        input: schema.object({
            /** The name the installation's package declares the fired trigger under. */
            triggerName: schema.string().min(1).exactOptional(),
            /** The event that fired the trigger. */
            event: RunEvent.exactOptional(),
            /** When the call runs in UTC epoch milliseconds, at recording when absent. */
            at: Instant.exactOptional(),
            /** The subject the call runs for under its grant to the installation. */
            exchange: ExchangeAsk.exactOptional(),
        }),
    }),
    cancel: method.delete("cancel"),
    finish: method.mutation({
        permission: "record",
        isInternal: true,
        input: schema.object({
            /** How the run finished. */
            outcome: RunOutcome,
        }),
        output: schema.object({}),
    }),
    create: method.create("record", { isInternal: true }),
    update: method.update("record", { isInternal: true }),
    delete: method.delete("record", { isInternal: true }),
}));

/** One recorded run of an installation's call. */
export type Run = Select<typeof run.table>;

/** The verbs of runs beside their methods. */
export const Run = {
    /** Describe the event of a run finished at a time. */
    event(
        finished: Pick<
            Run,
            | "id"
            | "scope"
            | "installationId"
            | "triggerId"
            | "call"
            | "at"
            | "event"
            | "requestId"
            | "attempts"
            | "startedAt"
            | "traceId"
        >,
        outcome: RunOutcome,
        triggerName: string | null,
        now: number,
    ) {
        // key the event by its outcome's kind and keep the outcome beside the call and its attempts
        return {
            id: finished.id,
            time: EventTime.of(now),
            installationId: finished.installationId,
            triggerId: finished.triggerId,
            outcome: outcome.kind,
            method: Call.label(finished.call),
            data: {
                call: finished.call,
                triggerName,
                at: finished.at,
                startedAt: finished.startedAt,
                finishedAt: now,
                attempts: finished.attempts,
                outcome,
                traceId: finished.traceId,
                requestId: finished.requestId,
                event: finished.event,
            },
        };
    },
};

/** What an ended run's event keeps beside its keys. */
export const RunRecord = schema.object({
    /** The object method call. */
    call: Call,
    /** The name of the trigger that fired the run when it finished, null for a sent call. */
    triggerName: schema.string().nullable(),
    /** When the call was due, in UTC epoch milliseconds. */
    at: Instant,
    /** The first attempt's start in UTC epoch milliseconds, null for a run never attempted. */
    startedAt: Instant.nullable(),
    /** The finish, in UTC epoch milliseconds. */
    finishedAt: Instant,
    /** The attempts started. */
    attempts: schema.number().int().nonnegative(),
    /** How the run finished. */
    outcome: RunOutcome,
    /** The trace of the run's attempts, null for a run never traced. */
    traceId: schema.string().nullable(),
    /** The request that recorded the run. */
    requestId: schema.string().min(1),
    /** The event that fired the trigger, null for a sent call. */
    event: RunEvent.nullable(),
});

/** The outcomes of an installation's ended runs, with their calls, results and attempts. */
export const runOutcome = defineObject({
    name: "run-outcome",
    plural: "runOutcomes",
    scope: space,
    storage: {
        kind: "events",
        flush: { maxAge: { hours: 1 }, maxRows: MAX_SEGMENT_RUNS },
        retention: { days: 90 },
        delivery: "exactly-once",
    },
    fields: {
        /** The installation that made the call. */
        installation: field.reference(installation),
        /** The trigger that fired the run, absent for a sent call. */
        trigger: field.reference(trigger).optional(),
        /** How the run finished. */
        outcome: field.enum(RUN_OUTCOMES),
        /** The object type and method called, such as note.update. */
        method: field.string(),
        /** The run's call, result and attempts. */
        data: field.json(RunRecord),
    },
    permissions: { read: through("installation", "read"), record: none() },
}).methods((method) => ({
    record: method.createMany("record", {
        isInternal: true,
        fields: ["installationId", "triggerId", "outcome", "method", "data"],
    }),
}));

/** An ended run as its outcome event keeps it. */
export type FinishedRun = EventOf<typeof runOutcome>;
