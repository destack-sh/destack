import { space } from "@destack/account/object";
import { type Select } from "@destack/db";
import { activity, announcement, subscription } from "@destack/notification";
import {
    type MethodContext,
    defineObject,
    field,
    Method,
    type ObjectReconciliation,
    type ObjectType,
} from "@destack/object";

import {
    and,
    Condition,
    type DatabaseConnection,
    eq,
    type JsonCondition,
    Filter,
    gt,
    gte,
    ne,
    notExists,
    or,
    sql,
} from "@destack/db";
import { EventFilter, type EventStore, FilterText, Fold, SERIES_FOLDS } from "@destack/event";
import { ObjectReference, ObjectTypeReference } from "@destack/sync";
import { Digest, Duration, defineSchema, type Identifier, present, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { installation } from "@destack/stack/object";
import type { Step } from "@destack/workflow";
import {
    alert,
    type Alert,
    type AlertLabels,
    firingAlert,
    resolvedAlert,
    respond,
} from "./alert.ts";
import { Silence, silence } from "./silence.ts";
import { issue, ISSUE_FILTER_FIELDS } from "./issue.ts";

/** What an events condition with no events in its window does: count as clear, or fire. */
export const NO_DATA = ["ok", "alerting"] as const;

/** The issue changes an issue condition fires on. */
export const ISSUE_CHANGES = ["open", "regress", "escalate"] as const;

/** What fires an alert. */
export const AlertCondition = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** An issue the filter selects opens, regresses or escalates. */
            kind: schema.literal("issue"),
            /** The issue change. */
            on: schema.enum(ISSUE_CHANGES),
            /** The issues considered, as a filter over their fields, every issue when absent. */
            filter: FilterText.exactOptional(),
        }),
        schema.object({
            /** A fold of the events a filter selects crosses a threshold over a window. */
            kind: schema.literal("events"),
            /** The events type, such as observability's log, span or metric, or object's call. */
            event: ObjectTypeReference,
            /** The events folded, as a filter over their keys, every event when absent. */
            where: FilterText.exactOptional(),
            /** The numeric key folded, needed by every fold but count. */
            measure: schema.string().min(1).exactOptional(),
            /** The fold. */
            fold: schema.enum(SERIES_FOLDS),
            /** The keys whose values each get an alert of their own. */
            group: schema.array(schema.string().min(1)).max(4),
            /** The events the fold is a ratio of. */
            per: schema
                .object({
                    /** The events of the denominator, every event of the kind when absent. */
                    where: FilterText.exactOptional(),
                })
                .exactOptional(),
            /** The window the fold covers, at least a minute. */
            window: Duration.schema,
            /** Whether the alert fires above or below the threshold. */
            comparison: schema.enum(["above", "below"]),
            /** The threshold. */
            threshold: schema.number(),
        }),
    ]),
);
/** What fires an alert. */
export type AlertCondition = schema.Infer<typeof AlertCondition>;

/** The names of the targeted mutations people may call on an installation, such as `rollBack`. */
const [firstMethod, ...otherMethods] = Object.entries(installation.methods)
    .filter(([, method]) => method.mutates && Method.loads(method) && method.isInternal !== true)
    .map(([name]) => name);

/** The methods an alert calls on its installation. */
const InstallationMethod = schema.enum([
    present(firstMethod, "an installation method people may call"),
    ...otherMethods,
]);

/** What a firing alert does. */
export const AlertAction = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** Notify the people and endpoints subscribed to the rule. */
            kind: schema.literal("notify"),
        }),
        schema.object({
            /** Call a method of the alert's installation. */
            kind: schema.literal("call"),
            /** The installation method, such as `rollBack`. */
            method: InstallationMethod,
        }),
    ]),
);
/** What a firing alert does. */
export type AlertAction = schema.Infer<typeof AlertAction>;

/** An alert rule as a stack declares it. */
export const AlertRuleDefinition = defineSchema(
    schema.object({
        /** The rule's title, shown to people. */
        title: schema.string().min(1).max(100),
        /** What fires the rule. */
        condition: AlertCondition,
        /** What a firing alert does. */
        actions: schema.array(AlertAction).min(1).max(10),
        /** The least time between two alerts of the rule for the same subject. */
        interval: Duration.schema.exactOptional(),
        /** How long an events condition stays crossed before its alert fires. */
        pending: Duration.schema.exactOptional(),
        /** Whether an events condition with no events in its window fires. */
        noData: schema.enum(NO_DATA).exactOptional(),
    }),
);
/** An alert rule as a stack declares it. */
export type AlertRuleDefinition = schema.Infer<typeof AlertRuleDefinition>;

/** A rule its controller evaluates against a space's issues and events. */
export const alertRule = defineObject(
    {
        name: "alert-rule",
        plural: "alertRules",
        scope: space,
        controlled: true,
        declarable: { schema: AlertRuleDefinition },
        fields: {
            /** The principal who created the rule, whom its calls act as. */
            author: field.subject().caller(),
            /** The rule's title, shown to people. */
            title: field.string(schema.string().min(1).max(100)),
            /** What fires the rule. */
            condition: field.json(AlertCondition),
            /** What a firing alert does. */
            actions: field.json(schema.array(AlertAction).min(1).max(10)),
            /** The least time between two alerts of the rule for the same subject. */
            interval: field.json(Duration.schema).default({ minutes: 30 }),
            /** How long an events condition stays crossed before its alert fires. */
            pending: field.json(Duration.schema).default({ seconds: 0 }),
            /** Whether an events condition with an empty window fires, or counts as clear. */
            noData: field.enum(NO_DATA).default("ok"),
        },
        permissions: ["read", "write", "manage"],
        attachments: [
            activity.attach({ by: "read" }),
            announcement.attach({ by: "read" }),
            subscription.attach({ by: "read" }),
        ],
    },
    (method) => ({
        create: method.create("write", {
            fields: ["title", "condition", "actions", "interval", "pending", "noData"],
        }),
        update: method.update("write", {
            fields: ["title", "condition", "actions", "interval", "pending", "noData"],
        }),
        delete: method.delete("manage"),
    }),
);
/** How often a space's alert rules are evaluated again without a change: every minute. */
export const EVALUATION_MILLISECONDS = 60_000;

/** The narrowest window an events condition reads: a minute. */
const MINUTE_MILLISECONDS = 60_000;

/** The microseconds in a millisecond, between event times and clocks. */
const MICROSECONDS = 1000;

/** The capability of running alert rules' call actions, such as rolling back an installation. */
export const RESPOND_CAPABILITY = "respond";

/** One subject crossing a rule's condition, with the value that crossed. */
interface Crossing {
    /** The subject's key values. */
    readonly labels: AlertLabels;
    /** The digest of the labels. */
    readonly fingerprint: Digest;
    /** The issue an issue condition crossed for. */
    readonly issue: Identifier<"issue"> | undefined;
    /** The value that crossed. */
    readonly value: number;
    /** What crossed, as people read it. */
    readonly title: string;
}

/** An issue condition. */
type IssueCondition = Extract<AlertCondition, { kind: "issue" }>;

/** An events condition. */
type EventsCondition = Extract<AlertCondition, { kind: "events" }>;

/** A rule a space's alerts fire by, as its row stores it. */
export class AlertRule {
    /** The rule's row. */
    readonly row: Select<typeof alertRule.table>;

    /** Read a rule from its row. */
    constructor(row: Select<typeof alertRule.table>) {
        this.row = row;
    }

    /** Read the rule that raised an alert. */
    static async read(
        context: Pick<MethodContext, "read">,
        raised: Pick<Alert, "ruleId">,
    ): Promise<AlertRule> {
        const row = await context.read(alertRule).findFirst({ where: { id: raised.ruleId } });

        return new AlertRule(present(row, `alert rule ${raised.ruleId}`));
    }

    /** Refuse a condition with fields, types or folds the space cannot evaluate. */
    require(objects: readonly ObjectType[]): void {
        // require a filter over the issues' fields, firing at once on an issue's change
        const { condition } = this.row;
        if (condition.kind === "issue") {
            if (condition.filter !== undefined) {
                Filter.parse(condition.filter, { fields: ISSUE_FILTER_FIELDS });
            }
            if (Duration.milliseconds(this.row.pending) > 0) {
                throw new CallError("BAD_REQUEST", {
                    message: "an issue condition fires at once on the issue's change",
                });
            }
        }
        // require an events type the space serves, filters over its keys, and a fold it takes
        else {
            this.#requireEvents(condition, eventsTypeOf(objects, condition.event));
        }
    }

    /** Refuse call actions without the respond capability, or by a writer not managing the rule. */
    async requireCalls(writing: Pick<MethodContext, "require" | "enabled">): Promise<void> {
        // let only a rule's managers have it call installations, where the space enables responses
        if (this.#isCalling) {
            if (!writing.enabled(RESPOND_CAPABILITY)) {
                throw new CallError("PRECONDITION_FAILED", {
                    message: `a call action needs observability's ${RESPOND_CAPABILITY} capability`,
                });
            }
            const reference = alertRule.reference(this.row.scope, this.row.id);
            await writing.require(alertRule.permission("manage"), reference);
        }
    }

    /** Evaluate the rule: retire it once deleted, else advance its alerts and report. */
    async evaluate(reconciliation: ObjectReconciliation<typeof alertRule>): Promise<void> {
        // retire a deleted rule
        const { now, store } = reconciliation;
        const { condition } = this.row;
        if (this.row.deletedAt !== null) {
            await reconciliation.mutate([alertRule.call("finalize", alertRule.target(this.row))]);
        }
        // raise, fire and settle the alerts of the subjects crossing the condition, and report
        else {
            const crossings =
                condition.kind === "issue"
                    ? await this.#issueCrossings(store.database, condition)
                    : await this.#eventCrossings(store.events, store.objects, condition, now);
            await this.#advance(reconciliation, crossings);
            await this.#observe(reconciliation);
        }
    }

    /** Announce a fired or resolved alert to the rule's followers, unless silenced. */
    async announce(
        context: MethodContext,
        row: Alert,
        change: "fired" | "resolved",
    ): Promise<void> {
        // announce only for a notifying rule, a resolution only of an events alert that fired
        const isNotifying = this.row.actions.some((action) => action.kind === "notify");
        const isUnannounced =
            change === "resolved" && (this.row.condition.kind === "issue" || row.firedAt === null);
        if (!isNotifying || isUnannounced || (await this.#isSilenced(context, row))) {
            return;
        }

        // announce the change on the rule
        const announced = change === "fired" ? firingAlert : resolvedAlert;
        await context.mutate(context.caller, [
            announced.call({
                source: alertRule.reference(this.row.scope, this.row.id),
                audience: { kind: "subscribers" },
                payload: { alert: row.id, rule: this.row.title, title: row.title },
            }),
        ]);
    }

    /** Start the response to a fired alert as the rule's author, unless silenced. */
    async respond(context: MethodContext, row: Alert): Promise<void> {
        // respond only for a calling rule
        if (!this.#isCalling || (await this.#isSilenced(context, row))) {
            return;
        }

        // run the calls as the rule's author, within their grant to observability
        const actor = ObjectReference.at("author", this.row);
        await context.mutate(context.self, [
            respond.start({ alert: row.id }, { key: row.id, actor }),
        ]);
    }

    /** Make the rule's calls on a fired alert's installation, each a step of its response. */
    async act(step: Step, row: Alert): Promise<void> {
        // read the installation the labels name, else the issue's
        const named = schema
            .identifier("installation")
            .safeParse(labelValue(row.labels, "installationId"));
        const installationId = named.success
            ? named.data
            : (
                  await step.query(
                      "issue",
                      issue.call(
                          "get",
                          issue.target({
                              scope: this.row.scope,
                              id: present(row.issueId, `the issue of alert ${row.id}`),
                          }),
                      ),
                  )
              ).installationId;

        // make each call action's call on the installation
        const target = installation.target({ scope: this.row.scope, id: installationId });
        for (const [index, action] of this.row.actions.entries()) {
            if (action.kind === "call") {
                if (!isInstallationMethod(action.method)) {
                    throw new TypeError(`installation takes no method ${action.method}`);
                }
                await step.call(`call.${index}`, installation.call(action.method, target));
            }
        }
    }

    /** Whether the rule calls installation methods. */
    get #isCalling(): boolean {
        return this.row.actions.some((action) => action.kind === "call");
    }

    /** Raise alerts for new crossings, fire those past the pending period, settle cleared ones. */
    async #advance(
        reconciliation: ObjectReconciliation<typeof alertRule>,
        crossings: readonly Crossing[],
    ): Promise<void> {
        // raise an alert for each crossing without a live alert or one within the interval
        const { now } = reconciliation;
        const interval = Duration.milliseconds(this.row.interval);
        const alerts = await this.#alerts(reconciliation.database, now - interval);
        const fresh = crossings.filter(
            (crossing) => !alerts.some((each) => each.fingerprint === crossing.fingerprint),
        );
        const raised = await reconciliation.mutate(
            fresh.map((crossing) =>
                alert.call("open", {
                    ...alert.in(this.row.scope),
                    ruleId: this.row.id,
                    labels: crossing.labels,
                    fingerprint: crossing.fingerprint,
                    ...(crossing.issue === undefined ? {} : { issueId: crossing.issue }),
                    value: crossing.value,
                    title: crossing.title.slice(0, 500),
                }),
            ),
        );

        // fire the pending alerts still crossing past the pending period, the raised ones without one
        const pending = Duration.milliseconds(this.row.pending);
        const isCrossing = (row: Alert) =>
            crossings.some((crossing) => crossing.fingerprint === row.fingerprint);
        const due = [
            ...alerts.filter(
                (row) =>
                    row.status === "pending" && isCrossing(row) && row.createdAt <= now - pending,
            ),
            ...(pending === 0 ? raised : []),
        ];
        const fired = await reconciliation.mutate(
            due.map((row) => alert.call("fire", { ...alert.target(row), firedAt: now })),
        );

        // settle the live alerts whose subject cleared, and an issue's alert once fired
        const settled = [
            ...alerts.filter(
                (row) => (row.status === "pending" || row.status === "firing") && !isCrossing(row),
            ),
            ...(this.row.condition.kind === "issue" ? fired : []),
        ];
        await reconciliation.mutate(settled.map((row) => alert.call("settle", alert.target(row))));
    }

    /** Read the filtered issues that opened, regressed or escalated without an alert since. */
    async #issueCrossings(
        database: DatabaseConnection,
        condition: IssueCondition,
    ): Promise<Crossing[]> {
        // select the filtered issues of the rule's space
        const filter =
            condition.filter === undefined
                ? undefined
                : Condition.render(
                      Filter.parse(condition.filter, { fields: ISSUE_FILTER_FIELDS }),
                      issue.table,
                  );

        // select the changes since the rule began that none of its alerts answered
        const changedAt = condition.on === "open" ? issue.table.createdAt : issue.table.reopenedAt;
        const changed =
            condition.on === "open"
                ? undefined
                : eq(issue.table.status, condition.on === "regress" ? "regressed" : "unresolved");
        const answered = database
            .select({ id: alert.table.id })
            .from(alert.table)
            .where(
                and(
                    eq(alert.table.ruleId, this.row.id),
                    eq(alert.table.issueId, issue.table.id),
                    gte(alert.table.createdAt, changedAt),
                ),
            );
        const rows = await database
            .select()
            .from(issue.table)
            .where(
                and(
                    eq(issue.table.scope, this.row.scope),
                    filter,
                    changed,
                    gt(changedAt, this.row.createdAt),
                    notExists(answered),
                ),
            );

        return Promise.all(
            rows.map(async (row) => {
                const labels = { issue: row.id };

                return {
                    labels,
                    fingerprint: await fingerprintOf(labels),
                    issue: row.id,
                    value: row.count,
                    title: row.title,
                };
            }),
        );
    }

    /** Read the groups whose fold of the events over the window crosses the threshold. */
    async #eventCrossings(
        events: EventStore,
        objects: readonly ObjectType[],
        condition: EventsCondition,
        now: number,
    ): Promise<Crossing[]> {
        // read the window as one step
        const width = Math.max(Duration.milliseconds(condition.window), MINUTE_MILLISECONDS);
        const range: JsonCondition = {
            time: { gte: (now - width) * MICROSECONDS, lt: now * MICROSECONDS },
        };
        const kind = eventsTypeOf(objects, condition.event).eventKind();
        const { group, measure } = condition;
        const fold = (where: string | undefined) => {
            const selected = FilterText.condition(where);

            return events.series(
                kind,
                {
                    scopes: [this.row.scope],
                    where: selected === undefined ? range : { AND: [range, selected] },
                },
                { fold: condition.fold, group, ...(measure === undefined ? {} : { measure }) },
            );
        };

        // fold the selection, and the denominator of a ratio by subject
        const counted = await fold(condition.where);
        const whole =
            condition.per === undefined
                ? undefined
                : new Map(
                      await Promise.all(
                          (await fold(condition.per.where)).map(
                              async (series) =>
                                  [
                                      await fingerprintOf(series.group),
                                      series.steps[0]?.value,
                                  ] as const,
                          ),
                      ),
                  );

        // fire a no-data alert for an empty window when the rule says so
        const isEmpty = counted.every((series) => series.steps.every((step) => step.events === 0));
        if (isEmpty && this.row.noData === "alerting") {
            const title = `${condition.event.type} no data`;

            return [
                {
                    labels: {},
                    fingerprint: await fingerprintOf({}),
                    issue: undefined,
                    value: 0,
                    title,
                },
            ];
        }

        // compare each subject's value, as a share of the same subject's denominator when given
        const crossings = await Promise.all(
            counted.map(async (series): Promise<Crossing | undefined> => {
                // read the subject's value, as a ratio of its denominator when given
                const fingerprint = await fingerprintOf(series.group);
                const value = series.steps[0]?.value;
                const total = whole?.get(fingerprint);
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

                return isCrossing
                    ? {
                          labels: series.group,
                          fingerprint,
                          issue: undefined,
                          value: compared,
                          title: titleOf(condition, series.group, compared),
                      }
                    : undefined;
            }),
        );

        return crossings.filter((crossing) => crossing !== undefined);
    }

    /** Read the rule's live alerts, and those raised after a time, the latest of each subject. */
    async #alerts(database: DatabaseConnection, since: number): Promise<Alert[]> {
        // read the pending and firing alerts and those raised within the interval, newest first
        const rows = await database
            .select()
            .from(alert.table)
            .where(
                and(
                    eq(alert.table.scope, this.row.scope),
                    eq(alert.table.ruleId, this.row.id),
                    or(ne(alert.table.status, "resolved"), gt(alert.table.createdAt, since)),
                ),
            )
            .orderBy(sql`${alert.table.createdAt} desc`);

        // take the latest of each subject
        return rows.filter(
            (row, index) =>
                rows.findIndex((other) => other.fingerprint === row.fingerprint) === index,
        );
    }

    /** Report whether an active silence covers an alert. */
    async #isSilenced(context: MethodContext, row: Alert): Promise<boolean> {
        const silences = await context
            .read(silence)
            .findMany({ where: { scope: this.row.scope, endsAt: { gt: context.now } } });

        return silences.some((each) => new Silence(each).covers(row, context.now));
    }

    /** Report the rule evaluated at its generation, once per generation. */
    async #observe(reconciliation: ObjectReconciliation<typeof alertRule>): Promise<void> {
        // report only a new generation or a changed readiness, so the report wakes no evaluation of its own
        const ready = this.row.conditions["Ready"];
        const isReported =
            this.row.observedGeneration === this.row.generation && ready?.reason === "Evaluated";
        if (!isReported) {
            await reconciliation.mutate([
                alertRule.call("observe", {
                    ...alertRule.target(this.row),
                    observedGeneration: this.row.generation,
                    conditions: { Ready: { status: "true", reason: "Evaluated", message: "" } },
                }),
            ]);
        }
    }

    /** Refuse an events condition with keys its type lacks or a fold it cannot take. */
    #requireEvents(condition: EventsCondition, type: ObjectType): void {
        // require filters over the type's keys
        const kind = type.eventKind();
        for (const where of [condition.where, condition.per?.where]) {
            const parsed = FilterText.condition(where);
            if (parsed !== undefined) {
                EventFilter.check(kind, { scopes: [this.row.scope], where: parsed });
            }
        }

        // require quantiles of only a kind's distributions, a measure for other folds but count, and a group for a call
        const isQuantile = Fold.isQuantile(condition.fold);
        const isDistributed = kind.distribution !== undefined && condition.measure === undefined;
        if (isQuantile && (!isDistributed || condition.per !== undefined)) {
            throw new CallError("BAD_REQUEST", {
                message: `a ${condition.fold} fold reads only a kind's distributions`,
            });
        } else if (condition.measure === undefined && condition.fold !== "count" && !isQuantile) {
            throw new CallError("BAD_REQUEST", {
                message: `a ${condition.fold} fold needs a measured key`,
            });
        } else if (
            this.#isCalling &&
            !condition.group.includes("installationId") &&
            !condition.group.includes("issue")
        ) {
            throw new CallError("BAD_REQUEST", {
                message: "a call action needs a group by installation or issue",
            });
        }
    }
}

/** Digest a subject's labels in key order, its fingerprint across evaluations. */
function fingerprintOf(labels: AlertLabels): Promise<Digest> {
    return Digest.json(
        Object.entries(labels).toSorted(([left], [right]) => left.localeCompare(right)),
    );
}

/** Describe a subject crossing an events condition as people read it. */
function titleOf(condition: EventsCondition, labels: AlertLabels, value: number): string {
    // write the labels and the fold
    const written = Object.entries(labels)
        .map(([key, label]) => `${key}=${String(label)}`)
        .join(" ");
    const measured = condition.measure === undefined ? "" : `(${condition.measure})`;

    return `${condition.event.type} ${condition.fold}${measured} ${value} ${written}`.trim();
}

/** Read a label's value, or the entry of a map key under a dotted name. */
function labelValue(
    labels: Readonly<Record<string, unknown>>,
    name: string,
): string | number | null {
    // read a label itself, else the entry after the first dot of a map key
    const dot = name.indexOf(".");
    const map = dot === -1 ? undefined : labels[name.slice(0, dot)];
    const value: unknown =
        name in labels || typeof map !== "object" || map === null
            ? labels[name]
            : Reflect.get(map, name.slice(dot + 1));

    // answer text and numbers, writing a flag as text
    return typeof value === "string" || typeof value === "number"
        ? value
        : typeof value === "boolean"
          ? String(value)
          : null;
}

/** Report whether a name is a method of installations. */
function isInstallationMethod(name: string): name is keyof typeof installation.methods {
    return Object.hasOwn(installation.methods, name);
}

/** Find the events type a condition names among the types a space serves. */
function eventsTypeOf(objects: readonly ObjectType[], reference: ObjectTypeReference): ObjectType {
    // find the type among the events types
    const key = ObjectTypeReference.key(reference);
    const found = objects.find(
        (object) =>
            object.storage === "events" && ObjectTypeReference.key(object.typeReference) === key,
    );
    if (found === undefined) {
        throw new CallError("BAD_REQUEST", {
            message: `the space serves no events type ${reference.type}`,
        });
    }

    return found;
}
