import { index, type Select } from "@destack/db";
import { t } from "@destack/locale";
import { announcement, subscription } from "@destack/notification";
import { defineNotification } from "@destack/notification/declare";
import { defineObject, field, type ObjectType } from "@destack/object";
import { Duration, defineSchema, present, schema } from "@destack/schema";
import { installation, space } from "@destack/space/object";
import { issue } from "./issue.ts";

/** The issue changes an issue condition fires on. */
export const ISSUE_CHANGES = ["open", "regress", "escalate"] as const;

/** The step aggregates a series condition compares, as series steps report them. */
export const SERIES_AGGREGATES = ["value", "count", "sum", "p50", "p90", "p99"] as const;

/** What fires an alert: a change of the issues a filter selects, or a series crossing a threshold over a window. */
export const AlertCondition = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** An issue the filter selects opens, regresses or escalates. */
            kind: schema.literal("issue"),
            /** The issue change. */
            on: schema.enum(ISSUE_CHANGES),
            /** The issues considered, as a filter over their fields, every issue when absent. */
            filter: schema.string().min(1).max(1000).exactOptional(),
        }),
        schema.object({
            /** An installation's metric, or its share of another selection of the metric, crosses a threshold over a window. */
            kind: schema.literal("series"),
            /** The metric's name. */
            name: schema.string().min(1),
            /** The points counted, as a filter over their attributes, every point when absent. */
            filter: schema.string().min(1).max(1000).exactOptional(),
            /** The points the count is a share of, as a filter over their attributes, absent for the count itself. */
            per: schema.string().min(1).max(1000).exactOptional(),
            /** The attributes compared separately, such as `destack.issue` for each issue, each installation alone when absent. */
            group: schema.array(schema.string().min(1)).max(4).exactOptional(),
            /** The step aggregate compared. */
            aggregate: schema.enum(SERIES_AGGREGATES),
            /** The window the aggregate covers, at least a minute. */
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
    .filter(([, method]) => method.mutates && method.target && method.isSystem !== true)
    .map(([name]) => name);

/** The methods an alert calls on its installation. */
const InstallationMethod = schema.enum([
    present(firstMethod, "an installation method people may call"),
    ...otherMethods,
]);

/** What a firing alert does: notify the rule's subscribers, or call a method of its installation. */
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

/** An alert rule a package declares, which each installation of the package keeps in its space. */
export const AlertRuleDefinition = defineSchema(
    schema.object({
        /** The rule's name. */
        name: schema.string().min(1).max(100),
        /** What fires the rule. */
        condition: AlertCondition,
        /** What a firing alert does. */
        actions: schema.array(AlertAction).min(1).max(10),
        /** The least time between two alerts of the rule for the same subject, 30 minutes when absent. */
        interval: Duration.schema.exactOptional(),
    }),
);
/** An alert rule a package declares. */
export type AlertRuleDefinition = schema.Infer<typeof AlertRuleDefinition>;

/** An alert as its notifications carry it. */
export const AlertExcerpt = schema.object({
    /** The alert. */
    alert: schema.identifier("alert"),
    /** The rule's name. */
    rule: schema.string().max(100),
    /** The alert's title. */
    title: schema.string().max(500),
});
/** An alert as its notifications carry it. */
export type AlertExcerpt = schema.Infer<typeof AlertExcerpt>;

/** An alert rule fires. */
export const firingAlert = defineNotification({
    name: "firingAlert",
    title: "Alerts",
    description: "An alert rule you follow fires.",
    payload: AlertExcerpt,
    interruption: "timeSensitive",
    preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
    content: (payload) => ({ title: t`${payload.rule}`, body: payload.title }),
    summary: (count) => t`${count} alerts`,
});

/** A rule its controller evaluates against a space's issues and sessions, firing alerts and running their actions. */
export const alertRule = defineObject({
    name: "alert-rule",
    plural: "alertRules",
    scope: space,
    controlled: true,
    declarable: { schema: AlertRuleDefinition },
    fields: {
        /** The rule's name. */
        name: field.string(schema.string().min(1).max(100)),
        /** What fires the rule. */
        condition: field.json(AlertCondition),
        /** What a firing alert does. */
        actions: field.json(schema.array(AlertAction).min(1).max(10)),
        /** The least time between two alerts of the rule for the same issue or installation. */
        interval: field.json(Duration.schema).default({ minutes: 30 }),
    },
    permissions: ["read", "edit", "manage"],
    attachments: [announcement.attach({ by: "read" }), subscription.attach({ by: "read" })],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("edit", {
            fields: ["name", "condition", "actions", "interval"],
        }),
        update: method.update("edit", {
            fields: ["name", "condition", "actions", "interval"],
        }),
        delete: method.delete("manage"),
    }),
});
/** An alert rule as its table stores it. */
export type AlertRule = Select<typeof alertRule.table>;

/** Something wrong that people should act on: fired by a rule for an issue or an installation, or raised by hand. */
export const alert = defineObject({
    name: "alert",
    plural: "alerts",
    scope: space,
    fields: {
        /** The rule that fired it, absent for one raised by hand. */
        rule: field
            .reference("alert-rule", (): ObjectType => alertRule, { delete: "cascade" })
            .optional(),
        /** The issue it is about, absent for an installation's metric or a raised alert without one. */
        issue: field.reference("issue", (): ObjectType => issue, { delete: "cascade" }).optional(),
        /** The installation whose metric crossed, absent for an issue. */
        installation: field
            .reference("installation", (): ObjectType => installation, { delete: "cascade" })
            .optional(),
        /** The value that crossed the rule's condition, absent for a raised alert. */
        value: field.number().optional(),
        /** What is wrong, such as an issue's title. */
        title: field.string(schema.string().min(1).max(500)),
        /** Whether it fires, or settled once its condition cleared or someone resolved it. */
        status: field.state({
            initial: "firing",
            transitions: {
                resolve: { from: ["firing"], to: "resolved", permission: "edit" },
                settle: { from: ["firing"], to: "resolved", permission: "detect" },
            },
        }),
    },
    constraints: (entry) => [
        index("alert_rule_status").on(entry.scope, entry.ruleId, entry.status),
    ],
    permissions: ["read", "edit", "manage", "detect"],
    reserved: ["detect"],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        /** Raise an alert by hand. */
        create: method.create("edit", { fields: ["issueId", "installationId", "title"] }),
        /** Fire an alert of a rule. */
        fire: method.create(null, {
            isSystem: true,
            fields: ["ruleId", "issueId", "installationId", "value", "title"],
        }),
    }),
});
/** An alert as its table stores it. */
export type Alert = Select<typeof alert.table>;
