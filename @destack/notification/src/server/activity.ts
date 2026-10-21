import { Account } from "@destack/account/object";
import { eq, isNull, lt, or, Snapshot } from "@destack/db";
import type { ObjectReconciliation } from "@destack/object";
import { nth, present } from "@destack/schema";
import { Subject } from "@destack/sync";
import { Activity, activity } from "../object/activity.ts";
import { Notification, notification } from "../object/notification.ts";
import type { NotificationServer } from "./server.ts";

/** Implement activities: notify their recipients, answer their actions, and plan a home's. */
export function implementActivity(server: NotificationServer): typeof activity {
    return activity.implement({
        controller: {
            // plan each activity whose notification is missing or older than its last occurrence
            pending: {},
            list: async (view) => {
                const { table } = notification;
                const rows = await view.database
                    .select({ id: activity.table.id })
                    .from(activity.table)
                    .leftJoin(table, eq(table.id, Notification.ofColumn(activity.table.id)))
                    .where(or(isNull(table.id), lt(table.plannedAt, activity.table.occurredAt)));

                return rows.map((row) => ({ id: row.id }));
            },
            concurrency: server.concurrency,
            reconcile: plan,
        },
        methods: {
            act: async (call) => {
                // answer as the declaration of the release the space installs
                const { packageId, name } = call.target;
                const declared = await server.notification(call, packageId, name);

                return declared.act(call.input.action, call.input.text, call);
            },
            notify: async (call) => {
                await server.notify(call, call.input, call.now);

                return {};
            },
        },
    });
}

/** Start the delivery of a home's activity's new occurrence. */
async function plan(
    reconciliation: ObjectReconciliation<typeof activity>,
): Promise<number | undefined> {
    // plan only the activities of the person whose home the store serves
    const row = nth(reconciliation.rows, 0);
    const view = Snapshot.live(reconciliation.database);
    const home = present(reconciliation.store.space, "the served space");
    const person = await Account.homeOwner(view, home);
    if (person === undefined || !Subject.same(Activity.recipient(row), person)) {
        return undefined;
    }

    // create the notification, or plan the delivery of its new occurrence to every destination
    const id = Notification.of(row.id);
    const state = await view.read(notification.table).findFirst({ where: { id } });
    const target = notification.target({ scope: home, id });
    const planned = { plannedAt: reconciliation.now, deliveryAt: reconciliation.now };
    if (state === undefined) {
        await reconciliation.mutate([notification.call("create", { ...target, ...planned })]);
    } else if (!Notification.isPlanned(state, row)) {
        await reconciliation.mutate([
            notification.call("update", { ...target, ...planned, delivered: [] }),
        ]);
    }

    return undefined;
}
