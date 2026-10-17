import { space } from "@destack/account/object";
import { type Select } from "@destack/db";
import { type ObjectOf, defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { type Alert } from "./alert.ts";
import { alertRule } from "./alert-rule.ts";

/** The longest comment a silence carries, a maintenance note's worth. */
const COMMENT_LENGTH = 1000;

/** The most matchers one silence takes. */
const MATCHERS = 20;

/** How long an ended silence stays readable: 120 hours, Alertmanager's default data retention. */
const ENDED_RETENTION = { hours: 120 };

/** A condition on one label of an alert: equal to a value, or not. */
export const LabelMatcher = schema.object({
    /** The label. */
    label: schema.string().min(1),
    /** Whether the label equals the value, or differs from it. */
    operator: schema.enum(["=", "!="]),
    /** The value. */
    value: schema.string(),
});
/** A condition on one label of an alert. */
export type LabelMatcher = schema.Infer<typeof LabelMatcher>;

/** A window in which the alerts a rule and matchers select fire without notifying or acting. */
export const silence = defineObject(
    {
        name: "silence",
        plural: "silences",
        scope: space,
        expiring: [{ after: ENDED_RETENTION, from: "endsAt" }],
        fields: {
            /** The rule whose alerts it silences, every rule's when absent. */
            rule: field
                .reference((): ObjectOf<{ identity: "alert-rule" }> => alertRule, {
                    delete: "cascade",
                })
                .optional(),
            /** The label conditions an alert meets, every alert of the rule when none. */
            matchers: field.json(schema.array(LabelMatcher).max(MATCHERS)),
            /** When it starts. */
            startsAt: field.time(),
            /** When it ends. */
            endsAt: field.time(),
            /** Why, such as a maintenance window. */
            comment: field.string(schema.string().min(1).max(COMMENT_LENGTH)),
        },
        permissions: ["read", "write"],
        constraints: {
            window: { kind: "check", condition: { endsAt: { gt: { field: "startsAt" } } } },
            ends: { kind: "index", on: ["scope", "endsAt"] },
        },
    },
    (method) => ({
        create: method.create("write", {
            fields: ["ruleId", "matchers", "startsAt", "endsAt", "comment"],
        }),
        /** End a silence early or extend it, and change its comment. */
        update: method.update("write", { fields: ["endsAt", "comment"] }),
        delete: method.delete("write"),
    }),
);
/** A window in which matching alerts fire without notifying or acting, as its row stores it. */
export class Silence {
    /** The silence's row. */
    readonly row: Select<typeof silence.table>;

    /** Read a silence from its row. */
    constructor(row: Select<typeof silence.table>) {
        this.row = row;
    }

    /** Report whether the silence covers an alert at a time: window, rule and every matcher. */
    covers(covered: Pick<Alert, "ruleId" | "labels">, now: number): boolean {
        // cover alerts within the window, of the rule when it has one
        const isActive = this.row.startsAt <= now && now < this.row.endsAt;
        const isRule = this.row.ruleId === null || this.row.ruleId === covered.ruleId;

        // require every matcher to hold on the alert's labels
        const isMatched = this.row.matchers.every((matcher) => {
            const value = covered.labels[matcher.label];
            const isEqual =
                value !== undefined && value !== null && String(value) === matcher.value;

            return matcher.operator === "=" ? isEqual : !isEqual;
        });

        return isActive && isRule && isMatched;
    }
}
