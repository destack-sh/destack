import { none, permission, union } from "@destack/access";
import { space } from "@destack/account/object";
import { type Select } from "@destack/db";
import { argument, plural, t } from "@destack/locale";
import { defineNotification } from "@destack/notification/declare";
import { type ObjectOf, defineObject, field } from "@destack/object";
import { Digest, schema } from "@destack/schema";
import { defineWorkflow, NO_OUTPUT } from "@destack/workflow";
import { alertRule } from "./alert-rule.ts";
import { issue } from "./issue.ts";

/** The longest alert title: 500 characters, room for an issue's title and its labels. */
export const ALERT_TITLE_LENGTH = 500;

/** The longest rule title: 100 characters, a notification's subject line. */
export const RULE_TITLE_LENGTH = 100;

/** How long a resolved alert stays readable: 120 hours, Alertmanager's default data retention. */
export const RESOLVED_RETENTION = { hours: 120 };

/** The key values identifying an alert's subject, such as its installation and route. */
export const AlertLabels = schema.record(
    schema.string(),
    schema.union([schema.string(), schema.number()]).nullable(),
);
/** The key values identifying an alert's subject. */
export type AlertLabels = schema.Infer<typeof AlertLabels>;

/** An alert as its notifications carry it. */
export const AlertExcerpt = schema.object({
    /** The alert. */
    alert: schema.identifier("alert"),
    /** The rule's name. */
    rule: schema.string().max(RULE_TITLE_LENGTH),
    /** The alert's title. */
    title: schema.string().max(ALERT_TITLE_LENGTH),
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
    content: { title: t`${argument("rule")}`, body: t`${argument("title")}` },
    summary: t`${plural(argument("count"), { one: "# alert", other: "# alerts" })}`,
});

/** A firing alert resolves. */
export const resolvedAlert = defineNotification({
    name: "resolvedAlert",
    title: "Resolved alerts",
    description: "An alert of a rule you follow resolves.",
    payload: AlertExcerpt,
    interruption: "passive",
    preference: { channels: ["desktop"], delivery: "immediate" },
    content: { title: t`Resolved: ${argument("rule")}`, body: t`${argument("title")}` },
    summary: t`${plural(argument("count"), { one: "# alert", other: "# alerts" })} resolved`,
});

/** A rule's subject crossing its condition: pending, then firing, then resolved. */
export const alert = defineObject(
    {
        name: "alert",
        plural: "alerts",
        scope: space,
        expiring: [{ after: RESOLVED_RETENTION, from: "resolvedAt" }],
        fields: {
            /** The rule that raised it. */
            rule: field.reference((): ObjectOf<{ identity: "alert-rule" }> => alertRule, {
                delete: "cascade",
            }),
            /** The subject's key values. */
            labels: field.json(AlertLabels),
            /** The digest of the labels, one series of alerts per rule and subject. */
            fingerprint: field.string(Digest),
            /** The issue an issue condition raised it for. */
            issue: field.reference(issue, { delete: "cascade" }).optional(),
            /** The value crossing the rule's condition: the issue's count or the events' fold. */
            value: field.number(),
            /** What is wrong, such as an issue's title. */
            title: field.string(schema.string().min(1).max(ALERT_TITLE_LENGTH)),
            /** Whether it waits out the rule's pending period, fires, or resolved. */
            status: field.state({
                initial: "pending",
                transitions: {
                    fire: { from: ["pending"], to: "firing", permission: "detect" },
                    resolve: { from: ["pending", "firing"], to: "resolved", permission: "resolve" },
                },
            }),
            /** When it fired. */
            firedAt: field.time().optional(),
            /** When its firing was announced and its response started, absent while silenced. */
            announcedAt: field.time().optional(),
            /** When it resolved. */
            resolvedAt: field.time().optional(),
        },
        permissions: {
            read: none(),
            write: none(),
            manage: none(),
            detect: none(),
            // let people who write alerts and the rules' evaluation resolve them alike
            resolve: union(permission("write"), permission("detect")),
        },
        constraints: {
            series: { kind: "index", on: ["scope", "ruleId", "fingerprint", "createdAt"] },
            ruleIssue: { kind: "index", on: ["ruleId", "issueId", "createdAt"] },
        },
    },
    (method) => ({
        create: false,
        update: false,
        delete: false,
        /** Raise an alert of a rule for a subject, pending until it fires. */
        open: method.create("detect", {
            visibility: "package",
            fields: ["ruleId", "labels", "fingerprint", "issueId", "value", "title"],
        }),
        /** Announce a firing alert and start its response once, after any silence on it ends. */
        announce: method.mutation({
            permission: "detect",
            visibility: "package",
            input: schema.object({}),
        }),
    }),
);
/** An alert as its table stores it. */
export type Alert = Select<typeof alert.table>;

/** The input of a fired alert's response: the alert. */
const RespondInput = schema.object({
    /** The fired alert. */
    alert: schema.identifier("alert"),
});

/** Run a fired alert's calls as the rule's author, each waiting on any approval it needs. */
export const respond = defineWorkflow({ name: "respond", input: RespondInput, output: NO_OUTPUT });
