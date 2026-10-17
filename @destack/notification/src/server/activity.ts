import { Account } from "@destack/account/object";
import { Snapshot } from "@destack/db";
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
            pending: {},
            concurrency: server.concurrency,
            reconcile: plan,
        },
        methods: {
            act: async (call) => {
                // answer as the declaration of the release the space installs
                const { packageId, name } = call.target;
                const declared = (await server.catalog(call, packageId)).get(name);

                return declared.act(call.input.action, call.input.text, call);
            },
            notify: async (call) => {
                // render in each person's locale with the declaring release's catalogs
                const catalog = await server.catalog(call, call.input.packageId);
                const localize = (person: Subject) => catalog.localize(call, person);

                return catalog.get(call.input.name).notify(localize, call);
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

    // create the notification and start the delivery of a new occurrence
    const id = Notification.of(row.id);
    const state = await view.read(notification.table).findFirst({ where: { id } });
    const target = notification.target({ scope: home, id });
    if (!Notification.isPlanned(state, row)) {
        await reconciliation.mutate([
            ...(state === undefined
                ? [notification.call("create", { ...target, plannedAt: reconciliation.now })]
                : []),
            notification.call("plan", target),
        ]);
    }

    return undefined;
}
