import { t } from "@destack/locale";
import type { MethodContextOf, ResultOf } from "@destack/object";
import { Duration, nth, TimeZone } from "@destack/schema";
import { Recipient, Routing, Settings } from "../message/index.ts";
import type { NotificationContent } from "../notification/content.ts";
import { Inbox } from "../notification/inbox.ts";
import { Activity, activity } from "../object/activity.ts";
import { Notification, notification } from "../object/notification.ts";
import { summary } from "../object/summary.ts";
import { compose, createMessage, readMessageKey } from "./message.ts";
import type { NotificationServer } from "./server.ts";

/** Implement a home's scheduled summaries: send each once due. */
export function implementSummary(server: NotificationServer): typeof summary {
    return summary.implement({
        methods: {
            findOrCreate: async (context, next) => {
                // answer the summary scheduled at the time, or schedule it
                const [known] = await context
                    .read(summary)
                    .findMany({ where: { at: context.input.at } });

                return known ?? next();
            },
            send: (context) => send(context, server),
        },
        controller: {
            // send each summary once due
            pending: { sentAt: { isNull: true } },
            reconcile: async (reconciliation) => {
                // wait for the summary's time, then send it
                const row = nth(reconciliation.rows, 0);
                if (row.at > reconciliation.now) {
                    return row.at - reconciliation.now;
                }
                await reconciliation.mutate([summary.call("send", summary.target(row))]);

                return undefined;
            },
        },
    });
}

/** Send a scheduled summary to each summary destination, once. */
async function send(
    context: MethodContextOf<typeof summary, "send">,
    server: NotificationServer,
): Promise<ResultOf<typeof summary, "send">> {
    // read the notifications waiting for the summary, with their activities
    const { at } = context.target;
    const states = await context.read(notification).findMany({ where: { summaryAt: at } });
    const activities = await context.read(activity).findMany({
        where: { id: { in: states.map((state) => Notification.activity(state.id)) } },
    });
    const { entries } = new Inbox(states, activities, context.now);
    const [lead] = entries;
    if (lead === undefined) {
        await context.mutate(context.self, [release(context.scope, at)]);

        return context.update({ sentAt: context.now });
    }

    // read where the person receives notifications, and each entry's settings
    const { person } = lead;
    const recipient = await Recipient.read(context, person);
    const settings = await Settings.read(
        context,
        person,
        entries.map((entry) => entry.activity),
    );

    // send each destination the entries it takes in the summary, in the person's language
    const locale = await server.localization(
        context,
        import.meta.destack.package.id,
        recipient.locale,
    );
    const options = { emailDelay: Duration.milliseconds(server.emailDelay), summaryAt: at };
    const routings = entries.map(
        (entry, index) => new Routing(entry, recipient, nth(settings, index), options, context.now),
    );
    const decided = recipient.reachable(context).map((to) => ({
        to,
        decisions: routings.map((routing) => routing.decide(to)),
    }));
    const key = await readMessageKey(context);
    for (const { to, decisions } of decided) {
        // send the entries the destination takes, for the latest of them
        const included = entries.filter((_entry, index) => nth(decisions, index).action === "send");
        const [latest] = included;
        if (latest !== undefined) {
            const content: NotificationContent = {
                title: locale.render(t`Scheduled summary`),
                body: Activity.summarize(included.map((entry) => entry.activity)).join("\n"),
            };
            const composed = compose(server, to, content, undefined, context.scope);
            const source = notification.reference(context.scope, latest.notification.id);
            await createMessage(context, key, to, composed, source);
        }
    }

    // schedule a snoozed entry for the summary after its snooze, and release the rest
    const next = entries.flatMap((entry, index) => {
        // read the latest time a destination defers the entry to
        const deferred = decided.flatMap(({ decisions }) => {
            const decision = nth(decisions, index);

            return decision.action === "defer" ? [decision.until] : [];
        });
        if (deferred.length === 0) {
            return [];
        }

        // join the first summary after the latest deferral
        const after = Math.max(...deferred);
        const nextAt = TimeZone.next(
            recipient.timeZone,
            nth(settings, index).summarySchedule.times,
            after,
        );

        return [
            summary.call("findOrCreate", { ...summary.in(context.scope), at: nextAt }),
            notification.call("update", {
                ...notification.target(entry.notification),
                summaryAt: nextAt,
            }),
        ];
    });
    await context.mutate(context.self, [...next, release(context.scope, at)]);

    return context.update({ sentAt: context.now });
}

/** Release the notifications still waiting for a summary at a time. */
function release(scope: string, at: number) {
    return notification.call("reschedule", {
        ...notification.in(scope),
        where: { summaryAt: at },
        summaryAt: null,
    });
}
