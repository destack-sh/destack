import {
    and,
    Condition,
    type DatabaseConnection,
    eq,
    type JsonCondition,
    Filter,
    gt,
    not,
    sql,
} from "@destack/db";
import type { EventFilter, EventStore, Series } from "@destack/event";
import { type ObjectReconciliation, ServerCall } from "@destack/object";
import { Duration, present, schema } from "@destack/schema";
import { installation } from "@destack/space/object";
import { EventKey } from "../event/key.ts";
import { MetricFold } from "../metric/fold.ts";
import { FilterText } from "@destack/event/service";
import {
    alert,
    type Alert,
    alertRule,
    type AlertCondition,
    type AlertRule,
    issue,
    ISSUE_FILTER_FIELDS,
} from "../object/index.ts";

/** How often a space's alert rules are evaluated again without a change: every minute. */
const EVALUATION_MILLISECONDS = 60_000;

/** The narrowest window an events condition reads: a minute. */
const MINUTE_MILLISECONDS = 60_000;

/** The microseconds in a millisecond, between event times and clocks. */
const MICROSECONDS = 1000;

/** The values of the keys an events condition groups by. */
type Group = Readonly<Record<string, string | number | null>>;

/** One subject crossing a rule's condition, with the value that crossed. */
interface Crossing {
    /** The subject: `{ issue }` for an issue condition, the group's key values for an events condition. */
    readonly group: Group;
    /** The value that crossed. */
    readonly value: number;
    /** What crossed, as people read it. */
    readonly title: string;
}

/** What evaluating alert rules needs beside the objects: the space's events. */
export interface AlertOptions {
    /** The event store events conditions fold. */
    readonly events: EventStore;
}

/** Evaluate a space's alert rules: fire alerts for the subjects crossing each condition, settle those that cleared and report each rule's evaluation. */
export async function evaluateRules(
    reconciliation: ObjectReconciliation<typeof alertRule>,
    options: AlertOptions,
): Promise<number> {
    const { now, server } = reconciliation;
    for (const rule of reconciliation.rows) {
        // retire a deleted rule
        if (rule.deletionRequestedAt !== null) {
            await reconciliation.execute(alertRule, "finalize", [ServerCall.of(rule)]);
            continue;
        }

        // read the subjects crossing the condition, refusing an unreadable rule
        let crossings: Crossing[];
        try {
            const { condition } = rule;
            crossings =
                condition.kind === "issue"
                    ? await issueCrossings(server.database, rule, condition)
                    : await eventCrossings(options.events, rule, condition, now);
        } catch (error) {
            await observe(reconciliation, rule, now, "InvalidRule", String(error));
            continue;
        }

        // fire for the crossings, and settle the alerts whose subject cleared
        await fireAndSettle(reconciliation, rule, crossings);
        await observe(reconciliation, rule, now, "Evaluated", "");
    }

    return EVALUATION_MILLISECONDS;
}

/** Fire for crossing subjects without a firing alert or one within the rule's interval, call the rule's actions, and settle the alerts that cleared. */
async function fireAndSettle(
    reconciliation: ObjectReconciliation<typeof alertRule>,
    rule: AlertRule,
    crossings: readonly Crossing[],
): Promise<void> {
    // fire for crossing subjects without a firing alert or one within the interval
    const { now, server } = reconciliation;
    const alerts = await alertsOf(server.database, rule);
    const interval = Duration.milliseconds(rule.interval);
    const fired = crossings.filter((crossing) => {
        const latest = alerts.find((each) => subjectOf(each) === subjectOf(crossing));

        return (
            latest === undefined ||
            (latest.status !== "firing" && latest.createdAt <= now - interval)
        );
    });
    const created = await reconciliation.execute(
        alert,
        "fire",
        fired.map((crossing) => ({
            scope: rule.scope,
            input: {
                ruleId: rule.id,
                group: crossing.group,
                value: crossing.value,
                title: crossing.title.slice(0, 500),
            },
        })),
    );

    // call the installation methods the rule's actions name for each new alert
    for (const action of rule.actions) {
        if (action.kind === "call") {
            for (const each of created) {
                await call(reconciliation, each, action.method);
            }
        }
    }

    // settle alerts whose subject cleared, and an issue's change once fired
    const settled = [
        ...alerts.filter(
            (each) =>
                each.status === "firing" &&
                !crossings.some((crossing) => subjectOf(crossing) === subjectOf(each)),
        ),
        ...(rule.condition.kind === "issue" ? created : []),
    ];
    await reconciliation.execute(
        alert,
        "settle",
        settled.map((each) => ServerCall.of(each)),
    );
}

/** Read the issues a filter selects that opened, regressed or escalated since the rule's last evaluation. */
async function issueCrossings(
    database: DatabaseConnection,
    rule: AlertRule,
    condition: Extract<AlertCondition, { kind: "issue" }>,
): Promise<Crossing[]> {
    // select the filtered issues of the rule's space
    const filter =
        condition.filter === undefined
            ? undefined
            : Condition.render(
                  Filter.parse(condition.filter, { fields: ISSUE_FILTER_FIELDS }),
                  issue.table,
              );

    // select the changes since the rule began that none of its alerts answered: opened, regressed, or escalated back to unresolved
    const changedAt = condition.on === "open" ? issue.table.createdAt : issue.table.reopenedAt;
    const changed =
        condition.on === "open"
            ? undefined
            : eq(issue.table.status, condition.on === "regress" ? "regressed" : "unresolved");
    const answered = sql`EXISTS (SELECT 1 FROM ${alert.table} WHERE ${alert.table.ruleId} = ${rule.id} AND ${alert.table.group} ->> 'issue' = ${issue.table.id} AND ${alert.table.createdAt} >= ${changedAt})`;
    const rows = await database
        .select()
        .from(issue.table)
        .where(
            and(
                eq(issue.table.scope, rule.scope),
                filter,
                changed,
                gt(changedAt, rule.createdAt),
                not(answered),
            ),
        );

    return rows.map((row) => ({ group: { issue: row.id }, value: row.count, title: row.title }));
}

/** Read the groups whose fold of the events over the window, or its ratio to another selection's, crosses the threshold. */
async function eventCrossings(
    events: EventStore,
    rule: AlertRule,
    condition: Extract<AlertCondition, { kind: "events" }>,
    now: number,
): Promise<Crossing[]> {
    // read the window as one step
    const width = Math.max(Duration.milliseconds(condition.window), MINUTE_MILLISECONDS);
    const range = {
        scope: rule.scope,
        from: (now - width) * MICROSECONDS,
        before: now * MICROSECONDS,
    };
    const fold = (where: string | undefined, isDenominator: boolean) =>
        foldOf(
            events,
            condition,
            { ...range, ...conditionOf(FilterText.condition(where)) },
            isDenominator,
        );

    // fold the selection, and the denominator of a ratio
    const counted = await fold(condition.where, false);
    const whole = condition.per === undefined ? undefined : await fold(condition.per.where, true);

    // compare each group's value, as a share of the same group's denominator when given
    return counted.flatMap((series): Crossing[] => {
        // read the group's value and its denominator's, comparing their ratio when given
        const value = series.steps[0]?.value;
        const total =
            whole === undefined
                ? undefined
                : whole.find(
                      (other) => JSON.stringify(other.group) === JSON.stringify(series.group),
                  )?.steps[0]?.value;
        const compared =
            value === undefined
                ? undefined
                : whole === undefined
                  ? value
                  : total === undefined || total === 0
                    ? undefined
                    : value / total;
        const isCrossing =
            compared !== undefined &&
            (condition.comparison === "above"
                ? compared > condition.threshold
                : compared < condition.threshold);

        return isCrossing ? [crossingOf(condition, series.group, compared)] : [];
    });
}

/** Fold a selection of a condition's events over its range: percentiles of histogram points in observability, every other fold in the store, a ratio's denominator counted for a count. */
async function foldOf(
    events: EventStore,
    condition: Extract<AlertCondition, { kind: "events" }>,
    filter: EventFilter,
    isDenominator: boolean,
): Promise<Series[]> {
    // fold percentiles of the metric kind's histograms
    const { fold, group, measure } = condition;
    const kind = events.kind(condition.event);
    if (MetricFold.isQuantile(fold)) {
        if (kind.name !== "metric" || isDenominator) {
            throw new TypeError(`a ${fold} fold reads metric histograms and takes no denominator`);
        }

        return MetricFold.series(events, filter, { fold, group });
    }

    // fold every other kind in the store
    return events.series(kind, filter, {
        fold,
        group,
        ...(measure === undefined ? {} : { measure }),
    });
}

/** Wrap an optional condition as a filter's where. */
function conditionOf(where: JsonCondition | undefined): Pick<EventFilter, "where"> {
    return where === undefined ? {} : { where };
}

/** Describe a group crossing an events condition as people read it. */
function crossingOf(
    condition: Extract<AlertCondition, { kind: "events" }>,
    group: Group,
    value: number,
): Crossing {
    // name the group's key values and the fold
    const named = Object.entries(group)
        .map(([key, held]) => `${key}=${String(held)}`)
        .join(" ");
    const measured = condition.measure === undefined ? "" : `(${condition.measure})`;

    return {
        group,
        value,
        title: `${condition.event} ${condition.fold}${measured} ${value} ${named}`.trim(),
    };
}

/** Read a rule's latest alert of each subject, newest first. */
async function alertsOf(database: DatabaseConnection, rule: AlertRule): Promise<Alert[]> {
    const rows = await database
        .select()
        .from(alert.table)
        .where(and(eq(alert.table.scope, rule.scope), eq(alert.table.ruleId, rule.id)))
        .orderBy(sql`${alert.table.createdAt} desc`);

    return rows.filter(
        (row, index) => rows.findIndex((other) => subjectOf(other) === subjectOf(row)) === index,
    );
}

/** Key an alert or a crossing by its subject's key values, in key order. */
function subjectOf(named: Pick<Alert, "group"> | Crossing): string {
    return JSON.stringify(
        Object.entries(named.group).toSorted(([left], [right]) => left.localeCompare(right)),
    );
}

/** Call a method of a fired alert's installation, named by its subject or through its subject's issue. */
async function call(
    reconciliation: ObjectReconciliation<typeof alertRule>,
    fired: Alert,
    method: string,
): Promise<void> {
    // read the installation the subject names, else its issue's
    const named = schema
        .identifier("installation")
        .safeParse(EventKey.read(fired.group, "installation"));
    const issueId = schema.identifier("issue").parse(EventKey.read(fired.group, "issue"));
    const [subject] = named.success
        ? [{ installationId: named.data }]
        : await reconciliation.database
              .select({ installationId: issue.table.installationId })
              .from(issue.table)
              .where(eq(issue.table.id, issueId));
    const id = present(subject, `issue ${issueId}`).installationId;

    // call the method on the space's installation as the space
    const [row] = await reconciliation.database
        .select()
        .from(installation.table)
        .where(eq(installation.table.id, id));
    if (!isInstallationMethod(method)) {
        throw new TypeError(`installation takes no method ${method}`);
    }
    await reconciliation.execute(installation, method, [
        ServerCall.of(present(row, `installation ${id}`)),
    ]);
}

/** Report whether a name is a method of installations. */
function isInstallationMethod(name: string): name is keyof typeof installation.methods {
    return Object.hasOwn(installation.methods, name);
}

/** Report a rule's evaluation at its generation, up to a time. */
async function observe(
    reconciliation: ObjectReconciliation<typeof alertRule>,
    rule: AlertRule,
    now: number,
    reason: string,
    message: string,
): Promise<void> {
    // report only a new generation or a changed readiness, so the report wakes no evaluation of its own
    const ready = rule.conditions["Ready"];
    if (
        rule.observedGeneration === rule.generation &&
        ready?.reason === reason &&
        ready.message === message
    ) {
        return;
    }
    await reconciliation.execute(alertRule, "observe", [
        ServerCall.of(rule, {
            observedGeneration: rule.generation,
            conditions: {
                Ready: { status: reason === "Evaluated" ? "true" : "false", reason, message },
            },
        }),
    ]);
}
