import { space } from "@destack/account/object";
import { type Select } from "@destack/db";
import { activity, announcement, subscription } from "@destack/notification";
import { defineObject, field } from "@destack/object";
import { Duration, defineSchema, present, schema } from "@destack/schema";
import { installation } from "@destack/space/object";
import { FilterText } from "@destack/event/service";
import { EVENT_FOLDS } from "../query/query.ts";

/** The issue changes an issue condition fires on. */
export const ISSUE_CHANGES = ["open", "regress", "escalate"] as const;

/** What fires an alert: a change of the issues a filter selects, or a fold of events crossing a threshold over a window. */
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
            /** A fold of the events a filter selects, or its ratio to another selection's, crosses a threshold over a window, per group. */
            kind: schema.literal("events"),
            /** The event kind's name, such as log, span, metric or call. */
            event: schema.string().min(1),
            /** The events folded, as a filter over their keys, every event when absent. */
            where: FilterText.exactOptional(),
            /** The numeric key folded, needed by every fold but count. */
            measure: schema.string().min(1).exactOptional(),
            /** The fold. */
            fold: schema.enum(EVENT_FOLDS),
            /** The keys each value of gets an alert of its own, such as installation or attributes.http.route. */
            group: schema.array(schema.string().min(1)).max(4),
            /** The events the fold is a ratio of: counted for a count, else folded alike, absent for the fold itself. */
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
    .filter(([, method]) => method.mutates && method.target && method.isInternal !== true)
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

/** A rule its controller evaluates against a space's issues and events, firing alerts and running their actions. */
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
        /** The least time between two alerts of the rule for the same subject. */
        interval: field.json(Duration.schema).default({ minutes: 30 }),
    },
    permissions: ["read", "edit", "manage"],
    attachments: [
        activity.attach({ by: "read" }),
        announcement.attach({ by: "read" }),
        subscription.attach({ by: "read" }),
    ],
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
