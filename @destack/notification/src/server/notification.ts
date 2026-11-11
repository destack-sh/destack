import { now, Snapshot } from "@destack/db";
import { Account } from "@destack/account/object";
import { Subject } from "@destack/sync";
import { Destination } from "@destack/message/object";
import {
    type MethodContextOf,
    type ObjectReconciliation,
    ObjectWatch,
    type ResultOf,
} from "@destack/object";
import { Duration, type Identifier, nth, present, schema } from "@destack/schema";
import { Recipient, Routing, Settings } from "../message/index.ts";
import { InboxEntry } from "../notification/inbox.ts";
import { Activity, activity } from "../object/activity.ts";
import { notification } from "../object/notification.ts";
import { summary } from "../object/summary.ts";
import { compose, createMessage, readMessageKey } from "./message.ts";
import type { NotificationServer } from "./server.ts";

/** Implement a home's notifications: deliver each occurrence to the destinations due. */
export function implementNotification(server: NotificationServer): typeof notification {
    return notification.implement({
        methods: { deliver: (context) => deliver(context, server) },
        projections: [
            {
                // plan a home's notification of an activity as the activity changes, and again once it is stale or missing
                watches: [ObjectWatch.of(activity.table, (row) => [{ activityId: row.id }])],
                project: (activityId, reconciliation) =>
                    plan(schema.identifier("activity").parse(activityId), reconciliation),
            },
        ],
        controllers: [
            {
                // deliver a home's due notifications in one mutation, one home at a time
                pending: { deliveryAt: { lte: now } },
                key: ["scope"],
                reconcile: async ({ rows, mutate }) => {
                    await mutate(
                        rows.map((row) => notification.call("deliver", notification.target(row))),
                    );
                },
            },
        ],
    });
}

/** Plan the delivery of an activity's notification once it occurs again, creating it first. */
async function plan(
    activityId: Identifier<"activity">,
    reconciliation: ObjectReconciliation<typeof notification>,
): Promise<void> {
    // read the activity, none once it left the home
    const view = Snapshot.live(reconciliation.database);
    const occurred = await view.read(activity.table).findFirst({ where: { id: activityId } });
    if (occurred === undefined) {
        return;
    }

    // plan only the activities of the person whose home the store serves
    const home = present(reconciliation.store.space, "the served space");
    const person = await Account.homeOwner(view, home);
    if (person === undefined || !Subject.same(Activity.recipient(occurred), person)) {
        return;
    }

    // create the notification, or plan the delivery of its new occurrence to every destination, recording which occurrence it planned
    const [state] = reconciliation.rows;
    const planned = { plannedAt: occurred.occurredAt, deliveryAt: reconciliation.now };
    if (state === undefined) {
        await reconciliation.mutate([
            notification.call("create", { ...notification.in(home), activityId, ...planned }),
        ]);
    } else if (!new InboxEntry(state, occurred, reconciliation.now).isPlanned) {
        await reconciliation.mutate([
            notification.call("update", {
                ...notification.target(state),
                ...planned,
                delivered: [],
            }),
        ]);
    }
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
    else if (!entry.isPlanned) {
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
        throw new TypeError(
            `'${row.id}' has more than one summary time: '${[...times].join(", ")}'`,
        );
    }
    const [summarized] = times;
    if (summarized !== undefined) {
        await context.mutate(context.self, [
            summary.call("findOrCreate", { ...summary.in(context.scope), at: summarized }),
        ]);
    }

    // look again at the earliest deferral, and finish once no destination waits
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
