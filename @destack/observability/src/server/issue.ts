import { eq, Filter } from "@destack/db";
import { EventFilter, type EventStore } from "@destack/event";
import type { CallOf } from "@destack/object";
import { Subscription, subscription } from "@destack/notification";
import { serveActivities } from "@destack/notification/server";
import * as social from "@destack/social";
import { reaction } from "@destack/social";
import * as socialServer from "@destack/social/server";
import { ObjectWatch } from "@destack/object";
import { ServiceError } from "@destack/service/error";
import * as base from "../object/index.ts";
import { ISSUE_FILTER_FIELDS } from "../object/index.ts";
import { MetricFold } from "../metric/fold.ts";
import { FilterText } from "@destack/event/service";
import { evaluateRules } from "./alert.ts";

/** The activities and announcements of issues, alert rules and their comments, under their notifications. */
const activities = serveActivities({
    notifications: [
        base.regressedIssue,
        base.resolvedIssue,
        base.firingAlert,
        social.mention,
        social.thread,
        social.reply,
    ],
});

/** What serving issues and alert rules needs: the space's events. */
export interface IssueOptions {
    /** The event store events conditions fold. */
    readonly events: EventStore;
}

/** Serve issues announcing resolutions and regressions, alert rules a controller evaluates, alerts announced on their rule, and their comments and notifications. */
export function serveIssues(options: IssueOptions) {
    const alertRule = alertRuleOf(options);

    return {
        issue,
        alertRule: alertRule.control({
            pending: {},
            key: (row) => ({ scope: row.scope }),
            watches: [ObjectWatch.of(base.issue.table, (row) => [{ scope: row.scope }])],
            reconcile: (reconciliation) => evaluateRules(reconciliation, options),
        }),
        alert,
        comment: socialServer.comment,
        reaction,
        subscription,
        activity: activities.activity,
        announcement: activities.announcement,
    };
}

/** Issues announcing their resolutions and regressions, clearing their condition once reopened. */
const issue = base.issue.handle({
    resolve: async (call, next) => {
        const row = await next();
        await base.resolvedIssue.notify(call, noticeOf(row));

        return row;
    },
    regress: async (call) => {
        const row = await call.update({
            status: "regressed",
            reopenedAt: call.now,
            until: { kind: "event" },
        });
        await base.regressedIssue.notify(call, noticeOf(row));

        return row;
    },
    escalate: (call) =>
        call.update({ status: "unresolved", reopenedAt: call.now, until: { kind: "event" } }),
});

/** Alert rules refusing conditions they cannot evaluate or their author may not read, and subscribing their author. */
function alertRuleOf(options: IssueOptions) {
    return base.alertRule.handle({
        create: async (call, next) => {
            // refuse an unreadable rule, then follow the rule as its author
            requireRule(call, options);
            const row = await next();
            await Subscription.add(
                call,
                base.alertRule.reference(row.scope, row.id),
                call.requireCaller(),
                "author",
            );

            return row;
        },
        update: async (call, next) => {
            requireRule(call, options);

            return next();
        },
    });
}

/** Alerts announced to the subscribers of their rule. */
const alert = base.alert.handle({
    fire: async (call, next) => {
        // announce the alert on its rule when the rule notifies
        const row = await next();
        const [rule] =
            row.ruleId === null
                ? []
                : await call.database
                      .select()
                      .from(base.alertRule.table)
                      .where(eq(base.alertRule.table.id, row.ruleId));
        if (rule?.actions.some((action) => action.kind === "notify") === true) {
            await base.firingAlert.notify(call, {
                source: base.alertRule.reference(rule.scope, rule.id),
                audience: { kind: "subscribers" },
                payload: { alert: row.id, rule: rule.name, title: row.title },
            });
        }

        return row;
    },
});

/** The announcement of an issue to its subscribers. */
function noticeOf(row: base.Issue) {
    return {
        source: base.issue.reference(row.scope, row.id),
        audience: { kind: "subscribers" },
        payload: { issue: row.id, title: row.title },
    } as const;
}

/** A created or updated rule as its checks read it. */
type RuleCall = CallOf<typeof base.alertRule, "create"> | CallOf<typeof base.alertRule, "update">;

/** Refuse a rule whose condition names fields, kinds or folds it cannot evaluate, or whose call action names no installation. */
function requireRule(call: RuleCall, options: IssueOptions): void {
    // read the condition and whether an action calls an installation
    const { condition, actions = [] } = call.input;
    const isCalling = actions.some((action) => action.kind === "call");
    try {
        // require a filter over the issues' fields
        if (condition?.kind === "issue") {
            requireFilter(condition.filter, ISSUE_FILTER_FIELDS);
        }
        // require a kind the space keeps, filters over its keys, and a fold it takes
        else if (condition?.kind === "events") {
            requireEvents(condition, isCalling, options);
        }
    } catch (error) {
        throw new ServiceError("BAD_REQUEST", {
            message: error instanceof Error ? error.message : String(error),
        });
    }
}

/** Refuse a filter naming fields a type lacks or failing to parse. */
function requireFilter(filter: string | undefined, fields: ReadonlySet<string>): void {
    if (filter !== undefined) {
        Filter.parse(filter, { fields });
    }
}

/** Refuse an events condition over a kind the space lacks, keys the kind lacks, a fold it cannot take, or a call naming no installation. */
function requireEvents(
    condition: Extract<base.AlertCondition, { kind: "events" }>,
    isCalling: boolean,
    options: IssueOptions,
): void {
    // require filters over the kind's keys
    const kind = options.events.kind(condition.event);
    for (const where of [condition.where, condition.per?.where]) {
        const parsed = FilterText.condition(where);
        if (parsed !== undefined) {
            EventFilter.check(kind, { scope: "", where: parsed });
        }
    }

    // require percentiles of metric histograms alone, a measure for folds but count, and a group naming what a call calls
    const isQuantile = MetricFold.isQuantile(condition.fold);
    if (isQuantile && (kind.name !== "metric" || condition.per !== undefined)) {
        throw new TypeError(`a ${condition.fold} fold reads metric histograms alone`);
    } else if (condition.measure === undefined && condition.fold !== "count" && !isQuantile) {
        throw new TypeError(`a ${condition.fold} fold measures a key`);
    } else if (
        isCalling &&
        !condition.group.includes("installation") &&
        !condition.group.includes("issue")
    ) {
        throw new TypeError("a call action needs a group naming an installation or an issue");
    }
}
