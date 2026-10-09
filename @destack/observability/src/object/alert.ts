import { space } from "@destack/account/object";
import { index, type Select } from "@destack/db";
import { t } from "@destack/locale";
import { defineNotification } from "@destack/notification/declare";
import { defineObject, field, type ObjectType } from "@destack/object";
import { schema } from "@destack/schema";
import { alertRule } from "./alert-rule.ts";

/** The values of the keys naming an alert's subject: an issue condition's issue, or an events condition's group. */
export const AlertGroup = schema.record(
    schema.string(),
    schema.union([schema.string(), schema.number()]).nullable(),
);

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

/** A rule's subject crossing its condition: an issue that opened, regressed or escalated, or a group of events past a threshold. */
export const alert = defineObject({
    name: "alert",
    plural: "alerts",
    scope: space,
    fields: {
        /** The rule that fired it. */
        rule: field.reference("alert-rule", (): ObjectType => alertRule, { delete: "cascade" }),
        /** The subject: `{ issue }` for an issue condition, the group's key values for an events condition. */
        group: field.json(AlertGroup),
        /** The value that crossed the rule's condition: the issue's count, or the events' fold. */
        value: field.number(),
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
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        /** Fire an alert of a rule. */
        fire: method.create("detect", {
            isInternal: true,
            fields: ["ruleId", "group", "value", "title"],
        }),
    }),
});
/** An alert as its table stores it. */
export type Alert = Select<typeof alert.table>;
