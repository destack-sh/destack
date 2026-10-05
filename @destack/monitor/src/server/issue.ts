import { eq, Filter } from "@destack/db";
import { Subscription, subscription } from "@destack/notification";
import { serveNotifications } from "@destack/notification/server";
import * as social from "@destack/social";
import { reaction } from "@destack/social";
import * as socialServer from "@destack/social/server";
import { ObjectWatch } from "@destack/object";
import { ServiceError } from "@destack/service/error";
import * as base from "../object/index.ts";
import { ISSUE_FILTER_FIELDS } from "../object/index.ts";
import { type AlertOptions, evaluateRules } from "./alert.ts";

/** The activities and announcements of issues, alert rules and their comments, under their notifications. */
const activities = serveNotifications({
    notifications: [
        base.regressedIssue,
        base.resolvedIssue,
        base.firingAlert,
        social.mention,
        social.thread,
        social.reply,
    ],
});

/** Serve issues announcing resolutions and regressions, alert rules a controller evaluates, alerts announced on their rule, and their comments and notifications. */
export function serveIssues(options: AlertOptions) {
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
        await base.resolvedIssue.announce(call, noticeOf(row));

        return row;
    },
    regress: async (call) => {
        const row = await call.update({ status: "regressed", reopenedAt: call.now, until: null });
        await base.regressedIssue.announce(call, noticeOf(row));

        return row;
    },
    escalate: (call) => call.update({ status: "unresolved", reopenedAt: call.now, until: null }),
});

/** Alert rules refusing filters over unknown fields and subscribing their author. */
const alertRule = base.alertRule
    .handle({
        create: async (call, next) => {
            // refuse an unreadable filter, then follow the rule as its author
            requireFilter(call.input.condition);
            const row = await next();
            await Subscription.add(
                call,
                base.alertRule.reference(row.scope, row.id),
                call.requireCaller(),
                "author",
            );

            return row;
        },
        update: (call, next) => {
            requireFilter(call.input.condition);

            return next();
        },
});

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
            await base.firingAlert.announce(call, {
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

/** Refuse an issue condition's filter that names fields issues lack or does not parse. */
function requireFilter(condition: base.AlertCondition | undefined): void {
    if (condition?.kind !== "issue" || condition.filter === undefined) {
        return;
    }
    try {
        Filter.parse(condition.filter, { fields: ISSUE_FILTER_FIELDS });
    } catch (error) {
        throw new ServiceError("BAD_REQUEST", {
            message: error instanceof Error ? error.message : String(error),
        });
    }
}
