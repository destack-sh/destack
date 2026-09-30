import { through, union } from "@destack/access";
import { check, index, sql, uniqueIndex, type Select } from "@destack/db";
import { defineObject, field, method } from "@destack/object";
import { DeclarationName, PackageId } from "@destack/package";
import { CONCURRENCIES } from "@destack/service/schedule";
import { LentAuthority } from "@destack/service/authentication";
import { RunError } from "@destack/service/trigger";
import { schema } from "@destack/schema";
import { Call } from "@destack/sync";
import { installation } from "./installation.ts";
import { schedule } from "./schedule.ts";
import { space } from "./space.ts";

/** Where a run stands: waiting for an attempt, attempting, or finished one of three ways. */
export const RUN_STATES = ["pending", "running", "succeeded", "failed", "skipped"] as const;

/** The states of a run that claims its cause, waiting for or making an attempt. */
export const ACTIVE_RUN_STATES = ["pending", "running"] as const;

/** What a run was recorded for: a sent call, a schedule's occurrence, a webhook's delivery or a watch's change. */
export const RUN_CAUSES = ["send", "schedule", "webhook", "watch"] as const;

/** How runs of one cause may overlap: the schedule's rules, and in order one at a time for watches. */
export const RUN_CONCURRENCIES = [...CONCURRENCIES, "queue"] as const;

// TODO #Incomplete: delete finished runs under the space's retention, one policy with telemetry, logs and the change log
/** One object method call an installation runs later, and its attempts. */
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
        /** The caller's authority lent to the installation, absent for its own calls. */
        onBehalfOf: field.json(LentAuthority).optional(),

        // the cause, recorded once each
        /** What the run was recorded for. */
        cause: field.enum(RUN_CAUSES),
        /** The schedule whose occurrence runs, absent once the schedule is deleted. */
        schedule: field.reference(schedule, { delete: "null" }).optional(),
        /** The occurrence of the schedule. */
        scheduledAt: field.time().optional(),
        /** The package declaring the webhook or the watch. */
        packageId: field.string(PackageId).optional(),
        /** The webhook's or the watch's name within its package. */
        trigger: field.string(DeclarationName).optional(),
        /** The webhook delivery: the route path it arrived at and the sender's delivery identifier. */
        deliveryId: field.string(schema.string().min(1)).optional(),
        /** The log epoch of the watch's change. */
        epoch: field.string(schema.string().min(1)).optional(),
        /** The log sequence of the watch's change. */
        sequence: field.integer().optional(),
        /** The row the watch's snapshot delivers at the change's position. */
        key: field.string(schema.string().min(1)).optional(),

        // the execution
        /** Where the run stands. */
        state: field.enum(RUN_STATES),
        /** How runs of the same cause may overlap. */
        concurrency: field.enum(RUN_CONCURRENCIES),
        /** The attempts started so far. */
        attempts: field.integer(),
        /** The first attempt's start, in UTC epoch milliseconds. */
        startedAt: field.time().optional(),
        /** The finish, in UTC epoch milliseconds. */
        finishedAt: field.time().optional(),
        /** Why the run failed or was skipped. */
        error: field.json(RunError).optional(),
        /** The trace of the run's attempts. */
        traceId: field.string(schema.string().min(1)).optional(),
    },
    constraints: (run) => [
        // record each cause once
        uniqueIndex("run_schedule_cause")
            .on(run.schedule, run.scheduledAt)
            .where(sql`${run.cause} = 'schedule'`),
        uniqueIndex("run_webhook_cause")
            .on(run.installation, run.packageId, run.trigger, run.deliveryId)
            .where(sql`${run.cause} = 'webhook'`),
        uniqueIndex("run_watch_cause")
            .on(
                run.installation,
                run.packageId,
                run.trigger,
                run.epoch,
                run.sequence,
                sql`coalesce(${run.key}, '')`,
            )
            .where(sql`${run.cause} = 'watch'`),

        // claim a schedule refusing overlapping runs for one run at a time, across hosts
        uniqueIndex("run_claim")
            .on(run.schedule)
            .where(
                sql`${run.state} IN ('pending', 'running') AND ${run.concurrency} IN ('forbid', 'replace')`,
            ),
        index("run_due").on(run.state, run.at),

        // keep exactly the cause's columns
        check(
            "run_cause",
            sql`(${run.cause} = 'send' AND ${run.schedule} IS NULL AND ${run.scheduledAt} IS NULL AND ${run.packageId} IS NULL AND ${run.trigger} IS NULL AND ${run.deliveryId} IS NULL AND ${run.epoch} IS NULL AND ${run.sequence} IS NULL AND ${run.key} IS NULL) OR (${run.cause} = 'schedule' AND ${run.scheduledAt} IS NOT NULL AND ${run.packageId} IS NULL AND ${run.trigger} IS NULL AND ${run.deliveryId} IS NULL AND ${run.epoch} IS NULL AND ${run.sequence} IS NULL AND ${run.key} IS NULL) OR (${run.cause} = 'webhook' AND ${run.packageId} IS NOT NULL AND ${run.trigger} IS NOT NULL AND ${run.deliveryId} IS NOT NULL AND ${run.schedule} IS NULL AND ${run.scheduledAt} IS NULL AND ${run.epoch} IS NULL AND ${run.sequence} IS NULL AND ${run.key} IS NULL) OR (${run.cause} = 'watch' AND ${run.packageId} IS NOT NULL AND ${run.trigger} IS NOT NULL AND ${run.epoch} IS NOT NULL AND ${run.sequence} IS NOT NULL AND ${run.schedule} IS NULL AND ${run.scheduledAt} IS NULL AND ${run.deliveryId} IS NULL)`,
        ),
        check("run_attempts", sql`${run.attempts} >= 0`),
        check(
            "run_finish",
            sql`(${run.state} IN ('succeeded', 'failed', 'skipped')) = (${run.finishedAt} IS NOT NULL)`,
        ),
    ],
    permissions: {
        read: through("installation", "read"),
        send: through("installation", "run"),
        cancel: union(through("installation", "run"), through("installation", "update")),
    },
    reserved: ["send"],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        send: method.create("send", {
            fields: [
                "installation",
                "call",
                "cause",
                "packageId",
                "trigger",
                "deliveryId",
                "epoch",
                "sequence",
                "key",
            ],
            input: schema.object({
                /** When the call runs in UTC epoch milliseconds, at recording when absent. */
                at: schema.number().int().nonnegative().optional(),
                /** The lending of the sending caller's authority to the installation. */
                delegation: schema.string().min(1).optional(),
            }),
        }),
        cancel: method.update("cancel", { fields: [] }),
        create: method.create(null, { isSystem: true }),
        update: method.update(null, { isSystem: true }),
    },
});

/** One recorded run of an installation's call. */
export type Run = Select<typeof run.table>;
