import { space } from "@destack/account/object";
import {
    and,
    Condition,
    type DatabaseConnection,
    eq,
    Filter,
    gt,
    gte,
    type JsonCondition,
    ne,
    notExists,
    or,
    type Select,
    sql,
    isNull,
} from "@destack/db";
import { EventFilter, FilterText, Fold, SERIES_FOLDS } from "@destack/event";
import { activity, announcement, subscription } from "@destack/notification";
import {
    defineObject,
    field,
    Method,
    type MethodContext,
    type ObjectReconciliation,
    type ObjectType,
    Observation,
} from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import {
    Digest,
    type DigestCrypto,
    Duration,
    type Identifier,
    present,
    schema,
} from "@destack/schema";
import { rollout } from "@destack/stack/object";
import { ObjectReference, ObjectTypeReference } from "@destack/sync";
import type { Step, WaitOf } from "@destack/workflow";
import { ObservabilityError } from "../error/error.ts";
import {
    alert,
    type Alert,
    ALERT_TITLE_LENGTH,
    type AlertLabels,
    firingAlert,
    RESOLVED_RETENTION,
    resolvedAlert,
    respond,
    RULE_TITLE_LENGTH,
} from "./alert.ts";
import { issue, ISSUE_FILTER_FIELDS } from "./issue.ts";
import { Silence, silence } from "./silence.ts";

/** The most keys a rule groups by: four, as each key multiplies the alerts one rule can raise. */
const MAX_GROUP_KEYS = 4;

/** The most actions a rule takes on firing: ten, a notice plus a few calls. */
const MAX_ACTIONS = 10;

/** The least time between two alerts of a rule for one subject: half an hour, at most 48 a day. */
const REPEAT_INTERVAL = { minutes: 30 };

/** The narrowest window an events condition reads: a minute. */
const MINUTE_MILLISECONDS = 60_000;

/** How often a space's alert rules are evaluated again without a change: every minute. */
export const EVALUATION_MILLISECONDS = MINUTE_MILLISECONDS;

/** The microseconds in a millisecond, between event times and clocks. */
const MICROSECONDS = 1000;

/** The capability of running alert rules' call actions, such as rolling back an installation. */
export const RESPOND_CAPABILITY = "respond";

/** The pending period a rule declares by default: none, firing at the first crossing. */
export const NO_PENDING: Duration = { seconds: 0 };

/** What an events condition with no events in its window does: count as clear, or fire. */
export const NO_DATA = ["ok", "alerting"] as const;

/** The issue changes an issue condition fires on. */
export const ISSUE_CHANGES = ["open", "regress", "escalate"] as const;

/** What fires an alert. */
export const AlertCondition = schema.discriminatedUnion("kind", [
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
        group: schema.array(schema.string().min(1)).max(MAX_GROUP_KEYS),
        /** The events the fold is a ratio of. */
        per: schema
            .object({
                /** The events of the denominator, every event of the kind when absent. */
                where: FilterText.exactOptional(),
            })
            .exactOptional(),
        /** The window the fold covers, at least a minute. */
        window: Duration.schema,
        /** Whether the alert fires above or below the threshold; an empty group has no series. */
        comparison: schema.enum(["above", "below"]),
        /** The threshold. */
        threshold: schema.number(),
    }),
]);
/** What fires an alert. */
export type AlertCondition = schema.Infer<typeof AlertCondition>;

/** The rollout methods an alert calls on its installation, which the respond capability grants. */
const RolloutMethod = schema.enum(
    Object.entries(rollout.methods)
        .filter(
            ([, method]) =>
                method.mutates &&
                Method.loads(method) &&
                Method.isRouted(method) &&
                method.permission === "apply",
        )
        .map(([name]) => name)
        .filter((name): name is keyof typeof rollout.methods =>
            Object.hasOwn(rollout.methods, name),
        ),
);

/** What a firing alert does. */
export const AlertAction = schema.discriminatedUnion("kind", [
    schema.object({
        /** Notify the people and endpoints subscribed to the rule. */
        kind: schema.literal("notify"),
    }),
    schema.object({
        /** Call a method of the alert's installation. */
        kind: schema.literal("call"),
        /** The installation method, such as `rollBack`. */
        method: RolloutMethod,
    }),
]);
/** What a firing alert does. */
export type AlertAction = schema.Infer<typeof AlertAction>;

/** A rule its controller evaluates against a space's issues and events. */
export const alertRule = defineObject(
    {
        name: "alert-rule",
        plural: "alertRules",
        scope: space,
        controlled: true,
        fields: {
            /** The principal who last set the condition or actions, whom its calls act as. */
            author: field.subject().caller(),
            /** The rule's title, shown to people. */
            title: field.string(schema.string().min(1).max(RULE_TITLE_LENGTH)),
            /** What fires the rule. */
            condition: field.json(AlertCondition),
            /** What a firing alert does. */
            actions: field.json(schema.array(AlertAction).min(1).max(MAX_ACTIONS)),
            /** The least time between two alerts of the rule for the same subject. */
            interval: field.json(Duration.schema).default(REPEAT_INTERVAL),
            /** How long an events condition stays crossed before its alert fires. */
            pending: field.json(Duration.schema).default(NO_PENDING),
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
/** One subject crossing a rule's condition, with the value that crossed. */
interface Crossing {
    /** The subject's key values. */
    readonly labels: AlertLabels;
    /** The digest of the labels. */
    readonly fingerprint: Digest;
    /** The issue an issue condition, or an events condition grouped by issue, crossed for. */
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

/** The fields a rule's check reads: its scope, condition, pending period and actions. */
type RuleDefinition = Pick<Select<typeof alertRule.table>, "condition" | "pending" | "actions"> & {
    /** The space the rule watches. */
    readonly scope: string;
};

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

    /** Refuse a rule whose condition has fields, types or folds the space cannot evaluate. */
    static require(rule: RuleDefinition, objects: readonly ObjectType[]): void {
        // require a filter over the issues' fields, firing at once on an issue's change
        const { condition } = rule;
        if (condition.kind === "issue") {
            if (condition.filter !== undefined) {
                Filter.parse(condition.filter, { fields: ISSUE_FILTER_FIELDS });
            }
            if (Duration.milliseconds(rule.pending) > 0) {
                throw ObservabilityError.of(
                    "ISSUE_PENDING_UNSUPPORTED",
                    "an issue condition takes no pending time",
                );
            }
        }
        // require an events type the space serves, filters over its keys, and a fold it takes
        else {
            AlertRule.#requireEvents(rule, condition, eventsTypeOf(objects, condition.event));
        }
    }

    /** Refuse call actions without the respond capability, or by a writer not managing the rule. */
    static async requireCalls(
        rule: Pick<Select<typeof alertRule.table>, "scope" | "id" | "actions">,
        writing: Pick<MethodContext, "require" | "enabled">,
    ): Promise<void> {
        // let only a rule's managers have it call installations, where the space enables responses
        if (rule.actions.some((action) => action.kind === "call")) {
            if (!writing.enabled(RESPOND_CAPABILITY)) {
                throw ObservabilityError.of(
                    "RESPOND_CAPABILITY_MISSING",
                    `a call action needs observability's ${RESPOND_CAPABILITY} capability`,
                );
            }
            const reference = alertRule.reference(rule.scope, rule.id);
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
        // raise, fire and resolve the alerts of the subjects crossing the condition, and report
        else {
            const crossings =
                condition.kind === "issue"
                    ? await this.#issueCrossings(store.database, condition, now)
                    : await this.#eventCrossings(store, condition, now);
            await this.#advance(reconciliation, crossings);
            await this.#announce(reconciliation);
            await this.#observe(reconciliation);
        }
    }

    /** Announce the rule's firing alerts a silence held back, once it no longer covers them. */
    async #announce(reconciliation: ObjectReconciliation<typeof alertRule>): Promise<void> {
        // read the rule's firing alerts held back so far
        const held = await reconciliation.store.database
            .select()
            .from(alert.table)
            .where(
                and(
                    eq(alert.table.ruleId, this.row.id),
                    eq(alert.table.status, "firing"),
                    isNull(alert.table.announcedAt),
                ),
            );
        if (held.length === 0) {
            return;
        }

        // announce only the alerts no active silence covers any longer
        const now = reconciliation.store.clock.now();
        const active = await reconciliation.store.database
            .select()
            .from(silence.table)
            .where(and(eq(silence.table.scope, this.row.scope), gt(silence.table.endsAt, now)));
        const silences = active.map((row) => new Silence(row));
        const released = held.filter((row) => !silences.some((each) => each.covers(row, now)));
        if (released.length > 0) {
            await reconciliation.mutate(
                released.map((row) => alert.call("announce", alert.target(row))),
            );
        }
    }

    /** Read the space's active silences once, for announcing and responding to one change. */
    async silences(context: MethodContext): Promise<Silence[]> {
        const rows = await context
            .read(silence)
            .findMany({ where: { scope: this.row.scope, endsAt: { gt: context.now } } });

        return rows.map((each) => new Silence(each));
    }

    /** Announce a fired or resolved alert to the rule's followers, unless a silence covers it. */
    async announce(
        context: MethodContext,
        row: Alert,
        change: "fired" | "resolved",
        silences: readonly Silence[],
    ): Promise<void> {
        // announce only for a notifying rule, a resolution only of an events alert that fired
        const isNotifying = this.row.actions.some((action) => action.kind === "notify");
        const isUnannounced =
            change === "resolved" &&
            (this.row.condition.kind === "issue" || row.announcedAt === null);
        if (
            !isNotifying ||
            isUnannounced ||
            silences.some((each) => each.covers(row, context.now))
        ) {
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
    async respond(context: MethodContext, row: Alert, silences: readonly Silence[]): Promise<void> {
        // respond only for a calling rule
        if (!this.#isCalling || silences.some((each) => each.covers(row, context.now))) {
            return;
        }

        // run the calls as the rule's author, within their grant to observability
        const actor = ObjectReference.at("author", this.row);
        await context.mutate(context.self, [
            respond.start({ alert: row.id }, { key: row.id, actor }),
        ]);
    }

    /** Make the rule's calls on a fired alert's installation, each a step of its response. */
    async act(step: Step<WaitOf<never>>, row: Alert): Promise<void> {
        // read the installation the labels name, else the issue's
        const named = schema.identifier("installation").safeParse(row.labels["installationId"]);
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
        const target = rollout.target({ scope: this.row.scope, id: installationId });
        for (const [index, action] of this.row.actions.entries()) {
            if (action.kind === "call") {
                await step.call(`call.${index}`, rollout.call(action.method, target));
            }
        }
    }

    /** Whether the rule calls installation methods. */
    get #isCalling(): boolean {
        return this.row.actions.some((action) => action.kind === "call");
    }

    /** Raise alerts for new crossings, fire those past the pending period, resolve cleared ones. */
    async #advance(
        reconciliation: ObjectReconciliation<typeof alertRule>,
        crossings: readonly Crossing[],
    ): Promise<void> {
        // raise an alert for each crossing without a live alert or one within the interval
        const { now } = reconciliation;
        const interval = Duration.milliseconds(this.row.interval);
        const alerts = await this.#alerts(reconciliation.database, now - interval);
        const alerted = new Set(alerts.map((each) => each.fingerprint));
        const fresh = crossings.filter((crossing) => !alerted.has(crossing.fingerprint));
        const raised = await reconciliation.mutate(
            fresh.map((crossing) =>
                alert.call("open", {
                    ...alert.in(this.row.scope),
                    ruleId: this.row.id,
                    labels: crossing.labels,
                    fingerprint: crossing.fingerprint,
                    ...(crossing.issue === undefined ? {} : { issueId: crossing.issue }),
                    value: crossing.value,
                    title: crossing.title.slice(0, ALERT_TITLE_LENGTH),
                }),
            ),
        );

        // fire the pending alerts still crossing past the pending period, the raised ones without one
        const pending = Duration.milliseconds(this.row.pending);
        const crossed = new Set(crossings.map((crossing) => crossing.fingerprint));
        const isCrossing = (row: Alert) => crossed.has(row.fingerprint);
        const due = [
            ...alerts.filter(
                (row) =>
                    row.status === "pending" && isCrossing(row) && row.createdAt <= now - pending,
            ),
            ...(pending === 0 ? raised : []),
        ];
        const fired = await reconciliation.mutate(
            due.map((row) => alert.call("fire", alert.target(row))),
        );

        // resolve the live alerts whose subject cleared, and an issue's alert once fired
        const resolved = [
            ...alerts.filter(
                (row) => (row.status === "pending" || row.status === "firing") && !isCrossing(row),
            ),
            ...(this.row.condition.kind === "issue" ? fired : []),
        ];
        await reconciliation.mutate(
            resolved.map((row) => alert.call("resolve", alert.target(row))),
        );
    }

    /** Read the filtered issues that opened, regressed or escalated without an alert since. */
    async #issueCrossings(
        database: DatabaseConnection,
        condition: IssueCondition,
        now: number,
    ): Promise<Crossing[]> {
        // hash fingerprints with the database's WebCrypto
        const { crypto } = database.state.environment;

        // select the filtered issues of the rule's space
        const filter =
            condition.filter === undefined
                ? undefined
                : Condition.render(
                      Filter.parse(condition.filter, { fields: ISSUE_FILTER_FIELDS }),
                      issue.table,
                  );

        // select the unanswered changes since the rule began, within its alerts' retention
        const since = Math.max(this.row.createdAt, now - Duration.milliseconds(RESOLVED_RETENTION));
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
                    gte(changedAt, since),
                    notExists(answered),
                ),
            );

        return Promise.all(
            rows.map(async (row) => {
                const labels = { issue: row.id };

                return {
                    labels,
                    fingerprint: await fingerprintOf(labels, crypto),
                    issue: row.id,
                    value: row.count,
                    title: row.title,
                };
            }),
        );
    }

    /** Read the groups whose fold of the events over the window crosses the threshold. */
    async #eventCrossings(
        store: ObjectStore,
        condition: EventsCondition,
        now: number,
    ): Promise<Crossing[]> {
        // read the store's events, objects and WebCrypto
        const { events, objects } = store;
        const { crypto } = store.environment;

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
                                      await fingerprintOf(series.group, crypto),
                                      series.steps[0]?.value,
                                  ] as const,
                          ),
                      ),
                  );

        // fire a no-data alert for an empty window when the rule says so
        const isEmpty = counted.every((series) => series.steps.every((step) => step.events === 0));
        if (isEmpty && this.row.noData === "alerting") {
            const title = `${this.row.title} no data`;

            return [
                {
                    labels: {},
                    fingerprint: await fingerprintOf({}, crypto),
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
                const fingerprint = await fingerprintOf(series.group, crypto);
                const value = series.steps[0]?.value;
                const compared = whole === undefined ? value : ratio(value, whole.get(fingerprint));
                const grouped = schema.identifier("issue").safeParse(series.group["issue"]);
                const crossing = {
                    labels: series.group,
                    fingerprint,
                    issue: grouped.success ? grouped.data : undefined,
                    title: titleOf(this.row.title, series.group),
                };

                // treat a subject without a value as no data, as an empty window
                if (compared === undefined) {
                    return this.row.noData === "alerting"
                        ? { ...crossing, value: 0, title: `${crossing.title} no data` }
                        : undefined;
                }
                // cross above or below the threshold
                else {
                    const isCrossing =
                        condition.comparison === "above"
                            ? compared > condition.threshold
                            : compared < condition.threshold;

                    return isCrossing ? { ...crossing, value: compared } : undefined;
                }
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
        const latest = new Map<string, Alert>();
        for (const row of rows) {
            if (!latest.has(row.fingerprint)) {
                latest.set(row.fingerprint, row);
            }
        }

        return [...latest.values()];
    }

    /** Report the rule evaluated at its generation, once per generation. */
    async #observe(reconciliation: ObjectReconciliation<typeof alertRule>): Promise<void> {
        // report only a new generation or a changed readiness
        const ready = this.row.conditions["Ready"];
        const isReported =
            Observation.isCurrent(this.row, "Ready") && ready?.reason === "Evaluated";
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
    static #requireEvents(
        rule: RuleDefinition,
        condition: EventsCondition,
        type: ObjectType,
    ): void {
        // require filters over the type's keys
        const kind = type.eventKind();
        for (const where of [condition.where, condition.per?.where]) {
            const parsed = FilterText.condition(where);
            if (parsed !== undefined) {
                EventFilter.check(kind, { scopes: [rule.scope], where: parsed });
            }
        }

        // require each fold's input: a distribution for quantiles, a measure but for counts, a group for calls
        const isQuantile = Fold.isQuantile(condition.fold);
        const isDistributed = kind.distribution !== undefined && condition.measure === undefined;
        if (isQuantile && (!isDistributed || condition.per !== undefined)) {
            throw ObservabilityError.of(
                "DISTRIBUTION_MISSING",
                `a ${condition.fold} fold can only read a kind's distributions`,
            );
        } else if (condition.measure === undefined && condition.fold !== "count" && !isQuantile) {
            throw ObservabilityError.of(
                "MEASURE_MISSING",
                `a ${condition.fold} fold needs a measured key`,
            );
        } else if (
            rule.actions.some((action) => action.kind === "call") &&
            !condition.group.includes("installationId") &&
            !condition.group.includes("issue")
        ) {
            throw ObservabilityError.of(
                "CALL_GROUP_MISSING",
                "a call action needs a group by installation or issue",
            );
        }
    }
}

/** Divide a value by its denominator, absent as no data for a zero or missing one. */
function ratio(value: number | undefined, total: number | undefined): number | undefined {
    return value === undefined || total === undefined || total === 0 ? undefined : value / total;
}

/** Digest a subject's labels in key order, its fingerprint across evaluations. */
function fingerprintOf(labels: AlertLabels, crypto: DigestCrypto): Promise<Digest> {
    return Digest.json(
        Object.entries(labels).toSorted(([left], [right]) => left.localeCompare(right)),
        crypto,
    );
}

/** Describe a subject crossing an events condition: the rule's title and the subject's labels. */
function titleOf(rule: string, labels: AlertLabels): string {
    const written = Object.entries(labels).map(([key, label]) => `${key}=${String(label)}`);

    return [rule, ...written].join(" ");
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
        throw ObservabilityError.of(
            "EVENTS_TYPE_UNKNOWN",
            `the space has no events type ${reference.type}`,
        );
    }

    return found;
}
