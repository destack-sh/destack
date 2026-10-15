import { Space } from "@destack/account/object";
import { eq } from "@destack/db";
import {
    type MethodContextOf,
    type NextOf,
    type ObjectReconciliation,
    ObjectWatch,
    type ResultOf,
} from "@destack/object";
import { CallError } from "@destack/runtime/error";
import { Schedule } from "@destack/runtime/schedule";
import { RunEvent, type ScheduleOn } from "@destack/runtime/trigger";
import { aligned, Identifier, present } from "@destack/schema";
import {
    Installation,
    installation,
    run,
    Run,
    type RunSkip,
    type Trigger,
    trigger,
    runOutcome,
} from "../../object/index.ts";

/** The most missed occurrences a schedule records one by one. */
const MAX_MISSED_OCCURRENCES = 100;

/** A trigger firing on a schedule, with the call its occurrences run. */
type ScheduleTrigger = Trigger & { readonly on: { readonly schedule: ScheduleOn } };

/** Implement triggers: fire schedules and refuse changes to declared ones. */
export function implementTrigger() {
    return trigger.implement({
        methods: {
            create: requireSchedule,
            update: (call, next) => requireCreated(call, () => requireSchedule(call, next)),
            delete: (call, next) => requireCreated(call, () => withdraw(call, next)),
            retire: withdraw,
            fire: (call) => fire(call),
        },
        controller: {
            pending: { isPaused: false },
            watches: [
                ObjectWatch.of(installation.table, async (row, database) =>
                    database
                        .select({ id: trigger.table.id })
                        .from(trigger.table)
                        .where(eq(trigger.table.installationId, row.id)),
                ),
                Space.watch((spaceId, database) =>
                    database
                        .select({ id: trigger.table.id })
                        .from(trigger.table)
                        .where(eq(trigger.table.scope, spaceId)),
                ),
            ],
            reconcile: (reconciliation) => {
                // fire a schedule's occurrences and leave the triggers the package's code fires
                const current = aligned(reconciliation.rows, 0);
                const schedule = readSchedule(current);

                return schedule === undefined
                    ? Promise.resolve(undefined)
                    : fireDue(schedule, reconciliation);
            },
        },
    });
}

/** Cancel a removed trigger's live runs, finishing each skipped and aborting its attempt. */
async function withdraw(
    context: MethodContextOf<typeof trigger, "delete">,
    next: NextOf<typeof trigger, "delete">,
): Promise<ResultOf<typeof trigger, "delete">> {
    // finish each waiting or attempting run skipped, as the space recording runs
    const live = await context.read(run).findMany({ where: { triggerId: context.target.id } });
    const cancelled = { kind: "skipped", skip: { reason: "cancelled" } } as const;
    if (live.length > 0) {
        await context.mutate(
            context.self,
            live.map((each) => run.call("finish", { ...run.target(each), outcome: cancelled })),
        );
    }

    return next();
}

/** Refuse changing or deleting a trigger its build declares. */
async function requireCreated<Result>(
    context: MethodContextOf<typeof trigger, "update"> | MethodContextOf<typeof trigger, "delete">,
    next: () => Promise<Result>,
): Promise<Result> {
    // refuse a trigger its build declares
    const { target } = context;
    if (target.origin === "declared") {
        throw new CallError("CONFLICT", {
            message: `trigger ${target.name} is declared by its build, which changes it`,
        });
    }

    return next();
}

/** Require a created trigger to fire on a schedule with its call. */
async function requireSchedule<Result>(
    context: MethodContextOf<typeof trigger, "create"> | MethodContextOf<typeof trigger, "update">,
    next: () => Promise<Result>,
): Promise<Result> {
    // require a schedule as the only trigger a row runs without its package's code
    const on = context.input.on ?? context.target?.on;
    if (on === undefined || !("schedule" in on)) {
        throw new CallError("BAD_REQUEST", {
            message: "a created trigger fires on a schedule",
        });
    }

    // check its timing
    Schedule.require(on.schedule.timing);

    return next();
}

/** Read a trigger as the schedule it fires on, absent for any other trigger. */
function readSchedule(current: Trigger): ScheduleTrigger | undefined {
    const on = current.on;

    return "schedule" in on ? { ...current, on } : undefined;
}

/** Fire a schedule's due occurrences on a served installation. */
async function fireDue(
    current: ScheduleTrigger,
    reconciliation: ObjectReconciliation,
): Promise<number | undefined> {
    // leave the schedule of an installation this machine does not serve now
    if (
        (await Installation.findServed(reconciliation.database, current.installationId)) ===
        undefined
    ) {
        return undefined;
    }

    // fire the occurrences due by now
    const { now } = reconciliation;
    await reconciliation.mutate([trigger.call("fire", { ...trigger.target(current), at: now })]);

    // look again at the next occurrence
    const next = Schedule.following(current.on.schedule.timing, now);

    return next === undefined ? undefined : next - now;
}

/** Record a schedule's occurrences due since its last one. */
async function fire(context: MethodContextOf<typeof trigger, "fire">) {
    // find the latest occurrences due since the last scheduled one or within the deadline
    const current = present(
        readSchedule(context.target),
        `the schedule of trigger ${context.target.id}`,
    );
    const { at: now } = context.input;
    const { timing, deadline } = current.on.schedule;
    const after = current.lastScheduledAt ?? now - deadline - 1;
    const { due, earlier } = Schedule.recent(timing, after, now, MAX_MISSED_OCCURRENCES);
    const last = due.at(-1);
    if (last === undefined) {
        return context.target;
    }

    // record the missed occurrences skipped as the transaction commits, summarising the earlier ones in the first
    const isDue = (scheduledAt: number) => now - scheduledAt <= deadline;
    const skips = due.flatMap((scheduledAt, index) =>
        scheduledAt === last && isDue(scheduledAt)
            ? []
            : [
                  skippedEvent(
                      current,
                      scheduledAt,
                      missed(!isDue(scheduledAt), index === 0 ? earlier : 0),
                      now,
                  ),
              ],
    );
    if (skips.length > 0) {
        await context.mutate(context.self, [
            runOutcome.call("record", { ...runOutcome.in(current.scope), items: skips }),
        ]);
    }

    // record the latest occurrence within its deadline for an attempt, and the last one scheduled
    if (isDue(last)) {
        await occur(context, current, last);
    }

    return context.update({ lastScheduledAt: last });
}

/** Record a schedule's occurrence in the firing call's transaction. */
async function occur(
    context: MethodContextOf<typeof trigger, "fire">,
    current: ScheduleTrigger,
    scheduledAt: number,
): Promise<void> {
    // find the run claiming a schedule refusing overlaps
    const concurrency = current.on.schedule.concurrency;
    const [claimant] =
        concurrency === "allow"
            ? []
            : await context
                  .read(run)
                  .findMany({ where: { triggerId: current.id, concurrency: { ne: "allow" } } });

    // skip an occurrence overlapping the run claiming the schedule
    if (claimant !== undefined && concurrency === "forbid") {
        const overlap = skippedEvent(current, scheduledAt, { reason: "overlap" }, context.now);
        await context.mutate(context.self, [
            runOutcome.call("record", { ...runOutcome.in(current.scope), items: [overlap] }),
        ]);

        return;
    }
    // finish the run a replacing occurrence displaces
    else if (claimant !== undefined) {
        const replaced = { kind: "skipped", skip: { reason: "replaced" } } as const;
        await context.mutate(context.self, [
            run.call("finish", { ...run.target(claimant), outcome: replaced }),
        ]);
    }

    // record the occurrence for an attempt
    await context.mutate(context.self, [
        run.call("create", {
            ...run.in(current.scope),
            ...occurrence(current, scheduledAt),
            state: "pending",
        }),
    ]);
}

/** Describe the run event of a schedule's occurrence skipped at a time. */
function skippedEvent(current: ScheduleTrigger, scheduledAt: number, skip: RunSkip, now: number) {
    const finished = {
        ...occurrence(current, scheduledAt),
        id: Identifier.create("run"),
        scope: current.scope,
        startedAt: null,
        traceId: null,
    };

    return Run.event(finished, { kind: "skipped", skip }, current.name, now);
}

/** Describe the fields of a new run of a schedule's occurrence. */
function occurrence(current: ScheduleTrigger, scheduledAt: number) {
    const event = { at: scheduledAt };

    return {
        installationId: current.installationId,
        call: current.on.schedule.call,
        at: scheduledAt,
        triggerId: current.id,
        event,
        requestId: RunEvent.requestId(event),
        concurrency: current.on.schedule.concurrency,
        attempts: 0,
    };
}

/** Describe a skipped occurrence. */
function missed(isLate: boolean, earlier: number | "many"): RunSkip {
    // stand for the occurrences omitted before the first recorded one
    if (earlier === "many" || earlier > 0) {
        return { reason: "missed", earlier };
    }
    // describe a late occurrence
    else if (isLate) {
        return { reason: "late" };
    }
    // describe an occurrence a later one supersedes
    else {
        return { reason: "superseded" };
    }
}
