import { and, Condition, eq, Filter, gt, not, sql, type DatabaseConnection } from "@destack/db";
import { SystemCall, type ObjectReconciliation } from "@destack/object";
import { Duration, type Identifier, present, schema } from "@destack/schema";
import { installation } from "@destack/space/object";
import type { AttributeValue, Series } from "../entry/index.ts";
import { Monitor } from "../monitor/index.ts";
import {
    alert,
    type Alert,
    alertRule,
    type AlertCondition,
    type AlertRule,
    issue,
    ISSUE_FILTER_FIELDS,
    type SERIES_AGGREGATES,
} from "../object/index.ts";
import { ISSUE_ATTRIBUTE } from "../issue/group.ts";

/** How often a space's alert rules are evaluated again without a change: every minute. */
const EVALUATION_MILLISECONDS = 60_000;

/** The narrowest window a series condition reads: a minute, the narrowest series step. */
const MINUTE_MILLISECONDS = 60_000;

/** One subject crossing a rule's condition: an issue or an installation, with the value that crossed. */
interface Crossing {
    /** The issue, absent for an installation. */
    readonly issue?: Identifier<"issue">;
    /** The installation whose metric crossed, absent for an issue. */
    readonly installation?: Identifier<"installation">;
    /** The value that crossed. */
    readonly value: number;
    /** What crossed, as people read it. */
    readonly title: string;
}

/** What evaluating alert rules needs beside the objects: the telemetry store series conditions read. */
export interface AlertOptions {
    /** The telemetry store whose series series conditions read. */
    readonly monitor: Pick<Monitor, "series" | "emitters">;
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
            await reconciliation.execute("finalize", [rule]);
            continue;
        }

        // read the subjects crossing the condition, refusing an unreadable filter
        let crossings: Crossing[];
        try {
            crossings =
                rule.condition.kind === "issue"
                    ? await issueCrossings(server.database, rule, rule.condition)
                    : await seriesCrossings(options.monitor, rule, rule.condition, now);
        } catch (error) {
            await observe(reconciliation, rule, now, "InvalidRule", String(error));
            continue;
        }

        // fire for crossing subjects without a firing alert or one within the interval, and settle the ones that cleared
        const alerts = await alertsOf(server.database, rule);
        const interval = Duration.milliseconds(rule.interval);
        const fired = crossings.filter((crossing) => {
            const latest = alerts.find((each) => subjectOf(each) === subjectOf(crossing));

            return (
                latest === undefined ||
                (latest.status !== "firing" && latest.createdAt <= now - interval)
            );
        });
        const created = await server.executeAsSystem(
            alert,
            "fire",
            fired.map((crossing) => ({
                scope: rule.scope,
                input: {
                    ruleId: rule.id,
                    ...(crossing.issue === undefined ? {} : { issueId: crossing.issue }),
                    ...(crossing.installation === undefined
                        ? {}
                        : { installationId: crossing.installation }),
                    value: crossing.value,
                    title: crossing.title.slice(0, 500),
                },
            })),
            now,
        );

        // call the installation methods the rule's actions name for each new alert
        for (const action of rule.actions) {
            if (action.kind === "call") {
                for (const each of created) {
                    await call(server, each, action.method, now);
                }
            }
        }

        // settle alerts whose subject cleared, and issue changes once fired
        const settled = [
            ...alerts.filter(
                (each) =>
                    each.status === "firing" &&
                    !crossings.some((crossing) => subjectOf(crossing) === subjectOf(each)),
            ),
            ...(rule.condition.kind === "issue" ? created : []),
        ];
        await server.executeAsSystem(
            alert,
            "settle",
            settled.map((each) => SystemCall.of(each)),
            now,
        );

        await observe(reconciliation, rule, now, "Evaluated", "");
    }

    return EVALUATION_MILLISECONDS;
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
    const answered = sql`EXISTS (SELECT 1 FROM ${alert.table} WHERE ${alert.table.ruleId} = ${rule.id} AND ${alert.table.issueId} = ${issue.table.id} AND ${alert.table.createdAt} >= ${changedAt})`;
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

    return rows.map((row) => ({ issue: row.id, value: row.count, title: row.title }));
}

/** Read the installations and groups whose metric, or its share of another selection, crosses the threshold over the window. */
async function seriesCrossings(
    monitor: AlertOptions["monitor"],
    rule: AlertRule,
    condition: Extract<AlertCondition, { kind: "series" }>,
    now: number,
): Promise<Crossing[]> {
    // read the window as one step
    const width = Math.max(Duration.milliseconds(condition.window), MINUTE_MILLISECONDS) * 1000;
    const before = now * 1000;
    const window = { name: condition.name, from: before - width, before, step: width };

    // compare each group of each installation of the space emitting the metric
    const crossings: Crossing[] = [];
    for (const key of await monitor.emitters()) {
        const { scope, installation: installationId } = Monitor.emitter(key);
        if (scope !== rule.scope || installationId === undefined) {
            continue;
        }
        const read = (filter: string | undefined) =>
            monitor.series(scope, {
                scope,
                installation: installationId,
                ...window,
                ...(filter === undefined ? {} : { filter }),
                group: condition.group ?? [],
            });
        const counted = await read(condition.filter);
        const whole = condition.per === undefined ? undefined : await read(condition.per);

        // take each group's aggregate, as a share of the same group's whole when given
        for (const group of counted.series) {
            const value = aggregated(group, condition.aggregate);
            const total = whole?.series.find(
                (other) => JSON.stringify(other.attributes) === JSON.stringify(group.attributes),
            );
            const compared =
                whole === undefined
                    ? value
                    : total === undefined || aggregated(total, condition.aggregate) === 0
                      ? undefined
                      : value / aggregated(total, condition.aggregate);
            const isCrossing =
                compared !== undefined &&
                (condition.comparison === "above"
                    ? compared > condition.threshold
                    : compared < condition.threshold);
            if (isCrossing) {
                crossings.push(crossingOf(condition, installationId, group.attributes, compared));
            }
        }
    }

    return crossings;
}

/** Describe a series group crossing a condition: the issue in its `destack.issue`, else its installation. */
function crossingOf(
    condition: Extract<AlertCondition, { kind: "series" }>,
    installationId: Identifier<"installation">,
    attributes: Readonly<Record<string, AttributeValue>>,
    value: number,
): Crossing {
    const issueId = attributes[ISSUE_ATTRIBUTE];
    const named = Object.entries(attributes)
        .map(([key, attribute]) => `${key}=${String(attribute)}`)
        .join(" ");

    return {
        ...(typeof issueId === "string"
            ? { issue: schema.identifier("issue").parse(issueId) }
            : { installation: installationId }),
        value,
        title: `${condition.name} ${condition.aggregate} ${value} ${named}`.trim(),
    };
}

/** Aggregate a series group's steps: summing counts, taking the highest percentile. */
function aggregated(
    group: Series["series"][number],
    aggregate: (typeof SERIES_AGGREGATES)[number],
): number {
    const values = group.steps.map((step) => step[aggregate] ?? 0);

    return aggregate.startsWith("p")
        ? Math.max(0, ...values)
        : values.reduce((total, value) => total + value, 0);
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

/** Key an alert or a crossing by the issue or installation it names. */
function subjectOf(named: Pick<Alert, "issueId" | "installationId"> | Crossing): string {
    const [issueId, installationId] =
        "issueId" in named
            ? [named.issueId, named.installationId]
            : [named.issue, named.installation];

    return `${issueId ?? ""}\0${installationId ?? ""}`;
}

/** Call a method of a fired alert's installation, found directly or through its issue. */
async function call(
    server: ObjectReconciliation<typeof alertRule>["server"],
    fired: Alert,
    method: string,
    now: number,
): Promise<void> {
    // find the installation the alert is about
    const [subject] =
        fired.installationId === null
            ? await server.database
                  .select({ installationId: issue.table.installationId })
                  .from(issue.table)
                  .where(
                      eq(
                          issue.table.id,
                          present(fired.issueId, "an alert's issue or installation"),
                      ),
                  )
            : [{ installationId: fired.installationId }];
    const id = present(subject, "the alert's subject").installationId;

    // send the call to the installation's home
    await server.database.transaction(async (transaction) => {
        const context = { database: transaction, scope: fired.scope, now };
        await server.change(context, installation, method, { id });
    });
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
    await reconciliation.server.executeAsSystem(
        alertRule,
        "observe",
        [
            SystemCall.of(rule, {
                observedGeneration: rule.generation,
                conditions: {
                    Ready: { status: reason === "Evaluated" ? "true" : "false", reason, message },
                },
            }),
        ],
        now,
    );
}
