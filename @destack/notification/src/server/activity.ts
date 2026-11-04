import { activity } from "../object/activity.ts";
import { Subscription } from "../object/subscription.ts";
import type { NotificationServer } from "./server.ts";

/** Implement activities: notify their recipients and answer their actions. */
export function implementActivity(server: NotificationServer): typeof activity {
    return activity.implement({
        methods: {
            act: async (call) => {
                // answer as the declaration of the release the space installs
                const { packageId, name } = call.target;
                const declared = await server.notification(call, packageId, name);

                return declared.act(call.input.action, call.input.text, call);
            },
            notify: async (call) => {
                // subscribe each principal a source addresses, as a mention subscribes on code forges
                const { parent, recipients, reason } = call.input;
                await call.mutate(
                    call.caller,
                    recipients.map((recipient) =>
                        Subscription.subscribe(parent, recipient, reason),
                    ),
                );

                // notify them as of now
                await server.notify(call, call.input, {
                    occurredAt: call.now,
                    actor: call.caller.subject,
                });

                return {};
            },
        },
    });
}
