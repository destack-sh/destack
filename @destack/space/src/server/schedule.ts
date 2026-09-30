import { and, desc, eq, inArray, ne } from "@destack/db";
import { Condition } from "@destack/db/query";
import type { Call, ObjectReconciliation } from "@destack/object";
import type { Identifier } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { describeTiming } from "@destack/service/inspect";
import type { ScheduleTiming } from "@destack/service/schedule";
import type { RunError } from "@destack/service/trigger";
import { CronExpressionParser } from "cron-parser";
import {
    ACTIVE_RUN_STATES,
    Installation,
    installation,
    run,
    type Schedule,
    schedule,
} from "../object/index.ts";

/** The most missed occurrences a schedule records one by one, as a Kubernetes CronJob counts. */
const MAX_MISSED_OCCURRENCES = 100;

/** Serve schedules: fire each one's due occurrences as runs on this cell, and keep its build's schedules to the build. */
export function serveSchedules(now: () => number) {
    return {
        schedule: schedule
            .handle({
                create: timed,
                update: (call, next) => undeclared(call, () => timed(call, next)),
                delete: undeclared,
            })
            .control({
                pending: Condition.eq("isPaused", false),
                watches: [
                    {
                        table: installation.table,
                        keys: async (row, database) =>
                            database
                                .select({ id: schedule.table.id })
                                .from(schedule.table)
                                .where(
                                    eq(
                                        schedule.table.installation,
                                        row.id as Identifier<"installation">,
                                    ),
                                ),
                    },
                ],
                reconcile: (reconciliation) =>
                    fire(reconciliation.rows[0]!, { ...reconciliation, now: now() }),
            }),
    };
}

/** Refuse changing or deleting a schedule its build declares. */
async function undeclared(
    call: Call<typeof schedule.table>,
    next: (call?: Call<typeof schedule.table>) => Promise<unknown>,
): Promise<unknown> {
    const target = call.target!;
    if (target.packageId !== null) {
        throw new ServiceError("CONFLICT", {
            message: `schedule ${target.name} is declared by its build, which changes it`,
        });
    }

    return next();
}

/** Refuse a schedule's timing with an unknown calendar or time zone, or a range ending before it starts. */
async function timed(
    call: Call<typeof schedule.table>,
    next: (call?: Call<typeof schedule.table>) => Promise<unknown>,
): Promise<unknown> {
    const timing = call.input.timing as ScheduleTiming | undefined;
    if (timing !== undefined) {
        try {
            describeTiming(timing);
        } catch (error) {
            throw new ServiceError("BAD_REQUEST", {
                message: `invalid schedule timing: ${(error as Error).message}`,
            });
        }
    }

    return next();
}

/** Record a schedule's due occurrences, returning the wait until its next one. */
async function fire(
    current: Schedule,
    reconciliation: ObjectReconciliation,
): Promise<number | undefined> {
    // leave the schedule of an installation this cell does not serve now
    const database = reconciliation.database;
    if ((await Installation.serving(database, current.installation)) === undefined) {
        return undefined;
    }

    // find the latest occurrences due since the latest recorded one, or within the deadline
    const now = reconciliation.now;
    const [latest] = await database
        .select({ scheduledAt: run.table.scheduledAt })
        .from(run.table)
        .where(eq(run.table.schedule, current.id))
        .orderBy(desc(run.table.scheduledAt))
        .limit(1);
    const after = latest?.scheduledAt ?? now - current.deadline - 1;
    const { due, earlier } = recent(current.timing, after, now, MAX_MISSED_OCCURRENCES);

    // record the missed occurrences skipped, summarising the earlier ones in the first
    const last = due.at(-1);
    const isDue = (scheduledAt: number) =>
        now - scheduledAt <= current.deadline && scheduledAt === last;
    const skipped = due
        .map((scheduledAt, index) => ({ scheduledAt, index }))
        .filter(({ scheduledAt }) => !isDue(scheduledAt));
    if (skipped.length > 0) {
        await reconciliation.server.executeAsSystem(
            run,
            "create",
            skipped.map(({ scheduledAt, index }) => ({
                scope: current.scope,
                input: {
                    ...occurrence(current, scheduledAt),
                    state: "skipped",
                    finishedAt: now,
                    error: missed(now - scheduledAt > current.deadline, index === 0 ? earlier : 0),
                },
            })),
            now,
        );
    }

    // record the latest occurrence within its deadline for an attempt
    if (last !== undefined && isDue(last)) {
        await occur(current, last, reconciliation);
    }

    // look again at the next occurrence
    const next = following(current.timing, now);

    return next === undefined ? undefined : next - now;
}

/** Record a schedule's occurrence, skipping or replacing the run claiming the schedule. */
async function occur(
    current: Schedule,
    scheduledAt: number,
    reconciliation: ObjectReconciliation,
): Promise<void> {
    const now = reconciliation.now;
    await reconciliation.database.transaction(
        async (transaction) => {
            // find the run claiming a schedule refusing overlaps
            const [claimant] =
                current.concurrency === "allow"
                    ? []
                    : await transaction
                          .select()
                          .from(run.table)
                          .where(
                              and(
                                  eq(run.table.schedule, current.id),
                                  inArray(run.table.state, [...ACTIVE_RUN_STATES]),
                                  ne(run.table.concurrency, "allow"),
                              ),
                          );

            // skip an occurrence overlapping the run claiming the schedule
            const invoke = (name: string, input: Readonly<Record<string, unknown>>) =>
                reconciliation.server.invoke(transaction, current.scope, run, name, input, now);
            if (claimant !== undefined && current.concurrency === "forbid") {
                await invoke("create", {
                    ...occurrence(current, scheduledAt),
                    state: "skipped",
                    finishedAt: now,
                    error: { code: "OVERLAP", message: "an earlier occurrence is still running" },
                });

                return;
            }
            // fail the run a replacing occurrence displaces, at the revision read
            else if (claimant !== undefined) {
                await invoke("update", {
                    id: claimant.id,
                    revision: claimant.revision,
                    state: "failed",
                    error: { code: "REPLACED", message: "a later occurrence replaced the run" },
                    finishedAt: now,
                });
            }

            // record the occurrence for an attempt
            await invoke("create", { ...occurrence(current, scheduledAt), state: "pending" });
        },
        { isolationLevel: "read committed" },
    );
}

/** The fields of a new run of a schedule's occurrence. */
function occurrence(current: Schedule, scheduledAt: number) {
    return {
        installation: current.installation,
        call: current.call,
        at: scheduledAt,
        cause: "schedule",
        schedule: current.id,
        scheduledAt,
        concurrency: current.concurrency,
        attempts: 0,
    };
}

/** Describe a skipped occurrence: missed past its deadline or superseded, summarising the earlier ones it stands for. */
function missed(isLate: boolean, earlier: number | "many"): RunError {
    // summarise the occurrences omitted before the first recorded one
    if (earlier === "many") {
        return {
            code: "MISSED_OCCURRENCES",
            message: `over ${MAX_MISSED_OCCURRENCES} occurrences were missed up to this one`,
        };
    } else if (earlier > 0) {
        return {
            code: "MISSED_OCCURRENCES",
            message: `${earlier + 1} occurrences were missed up to this one`,
        };
    }
    // describe a late or superseded occurrence
    else if (isLate) {
        return {
            code: "DEADLINE_EXCEEDED",
            message: "the occurrence was missed beyond its deadline",
        };
    }

    return { code: "SUPERSEDED", message: "a later occurrence runs in its place" };
}

/** List the latest occurrences of a timing after one time and up to another, at most a limit, and how many earlier ones fell in between. */
function recent(
    timing: ScheduleTiming,
    after: number,
    until: number,
    limit: number,
): { readonly due: number[]; readonly earlier: number | "many" } {
    // count fixed intervals arithmetically, within the end
    if (timing.timing === "interval") {
        const end = timing.endsAt === undefined ? until : Math.min(until, timing.endsAt - 1);
        const first = Math.max(0, Math.floor((after - timing.startsAt) / timing.interval) + 1);
        const last = Math.floor((end - timing.startsAt) / timing.interval);
        const kept = Math.max(first, last - limit + 1);
        const due = Array.from(
            { length: Math.max(0, last - kept + 1) },
            (_, index) => timing.startsAt + (kept + index) * timing.interval,
        );

        return { due, earlier: Math.max(0, kept - first) };
    }
    // occur once
    else if (timing.timing === "once") {
        const isDue = timing.startsAt > after && timing.startsAt <= until;

        return { due: isDue ? [timing.startsAt] : [], earlier: 0 };
    }

    // walk the calendar back from the end, within its bounds, one past the limit to learn whether more were missed
    const end = timing.endsAt === undefined ? until : Math.min(until, timing.endsAt - 1);
    const start = timing.startsAt === undefined ? after : Math.max(after, timing.startsAt - 1);
    const calendar = CronExpressionParser.parse(timing.cron, {
        currentDate: end + 1,
        tz: timing.timezone,
    });
    const walked: number[] = [];
    while (walked.length <= limit) {
        const previous = calendar.prev().getTime();
        if (previous <= start) {
            break;
        }
        walked.push(previous);
    }
    const isMore = walked.length > limit;

    return { due: walked.slice(0, limit).reverse(), earlier: isMore ? "many" : 0 };
}

/** Find a timing's next occurrence after a time, absent once none follows. */
function following(timing: ScheduleTiming, after: number): number | undefined {
    // step to the next interval, within the end
    if (timing.timing === "interval") {
        const index = Math.max(0, Math.floor((after - timing.startsAt) / timing.interval) + 1);
        const next = timing.startsAt + index * timing.interval;

        return timing.endsAt !== undefined && next >= timing.endsAt ? undefined : next;
    }
    // occur once
    else if (timing.timing === "once") {
        return timing.startsAt > after ? timing.startsAt : undefined;
    }

    // follow the calendar, within its bounds
    const start = timing.startsAt === undefined ? after : Math.max(after, timing.startsAt - 1);
    const next = CronExpressionParser.parse(timing.cron, {
        currentDate: start,
        tz: timing.timezone,
    })
        .next()
        .getTime();

    return timing.endsAt !== undefined && next >= timing.endsAt ? undefined : next;
}
