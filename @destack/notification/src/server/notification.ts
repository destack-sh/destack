import { Destination } from "@destack/message";
import type { MethodContextOf, ResultOf } from "@destack/object";
import { Duration, nth, present } from "@destack/schema";
import { Recipient, Routing, Settings } from "../message/index.ts";
import { InboxEntry } from "../notification/inbox.ts";
import { Notification, notification } from "../object/notification.ts";
import { summary } from "../object/summary.ts";
import { compose, createMessage, readMessageKey } from "./message.ts";
import type { NotificationServer } from "./server.ts";

/** Implement a home's notifications: deliver each occurrence to the destinations due. */
export function implementNotification(server: NotificationServer): typeof notification {
    return notification.implement({
        methods: { deliver: (context) => deliver(context, server) },
        controller: {
            // deliver a home's due notifications one home at a time, waiting for the next due one
            pending: { deliveryAt: { isNotNull: true } },
            key: (row) => ({ scope: row.scope }),
            reconcile: async (reconciliation) => {
                // deliver the due notifications in one mutation
                const { now } = reconciliation;
                const times = reconciliation.rows.map((row) =>
                    present(row.deliveryAt, "a pending delivery's time"),
                );
                const due = reconciliation.rows.filter((_row, index) => nth(times, index) <= now);
                await reconciliation.mutate(
                    due.map((row) => notification.call("deliver", notification.target(row))),
                );

                // look again at the next one due
                const waiting = times.filter((time) => time > now);

                return waiting.length === 0 ? undefined : Math.min(...waiting) - now;
            },
        },
    });
}

/** Deliver the last occurrence to each destination due now, deferring the others. */
async function deliver(
    context: MethodContextOf<typeof notification, "deliver">,
    server: NotificationServer,
): Promise<ResultOf<typeof notification, "deliver">> {
    // finish a delivery whose activity the home lost, as after a revoked share
    const row = context.target;
    const entry = await InboxEntry.read(context, row);
    if (entry === undefined) {
        return context.update({ deliveryAt: null });
    }
    // leave a newer occurrence to its planning, which starts its delivery afresh
    else if (!Notification.isPlanned(row, entry.activity)) {
        return row;
    }

    // decide each destination the occurrence has not reached, under the person's settings
    const recipient = await Recipient.read(context, entry.person);
    const settings = nth(await Settings.read(context, entry.person, [entry.activity]), 0);
    const emailDelay = Duration.milliseconds(server.emailDelay);
    const routing = new Routing(entry, recipient, settings, { emailDelay }, context.now);
    const decided = recipient
        .reachable(context)
        .filter((to) => !row.delivered.includes(Destination.key(to)))
        .map((to) => ({ to, decision: routing.decide(to) }));

    // send what goes out now, sealed to the home's key
    const sends = decided.filter(({ decision }) => decision.action === "send");
    const key = sends.length === 0 ? undefined : await readMessageKey(context);
    const notice = { notification: row.id, occurrence: entry.activity };
    const source = notification.reference(context.scope, row.id);
    for (const { to } of sends) {
        const content = compose(server, to, entry.activity.content, notice, context.scope);
        await createMessage(context, present(key, "the home's message key"), to, content, source);
    }

    // join the one summary the summarized destinations wait for, as one schedule decides them
    const times = new Set(
        decided.flatMap(({ decision }) => (decision.action === "summarize" ? [decision.at] : [])),
    );
    if (times.size > 1) {
        throw new TypeError(`${row.id} waits for summaries at ${[...times].join(", ")}`);
    }
    const [summarized] = times;
    if (summarized !== undefined) {
        await context.mutate(context.self, [
            summary.call("create", { ...summary.in(context.scope), at: summarized }),
        ]);
    }

    // keep the deferred destinations for the earliest deferral, and finish once none remains
    const deferred = decided.flatMap(({ decision }) =>
        decision.action === "defer" ? [decision.until] : [],
    );
    const settled = decided
        .filter(({ decision }) => decision.action !== "defer")
        .map(({ to }) => Destination.key(to));

    return context.update({
        delivered: [...row.delivered, ...settled],
        deliveryAt: deferred.length === 0 ? null : Math.min(...deferred),
        ...(summarized === undefined ? {} : { summaryAt: summarized }),
    });
}
