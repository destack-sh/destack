import { present } from "@destack/schema";
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
                // address only the recipients who can read the source
                const { parent, reason } = call.input;
                const source = { ...parent, scope: call.scope };
                const host = present(
                    call.objects.find((object) => object.policy.is(source)),
                    `the type of ${parent.type} ${parent.id}`,
                );
                const readable = await Promise.all(
                    call.input.recipients.map((recipient) =>
                        call.permits(recipient, host.permission("read"), source),
                    ),
                );
                const recipients = call.input.recipients.filter(
                    (_, index) => readable[index] === true,
                );
                if (recipients.length === 0) {
                    return {};
                }

                // subscribe each one, as a mention subscribes on code forges
                await call.mutate(
                    call.caller,
                    recipients.map((recipient) =>
                        Subscription.subscribe(parent, recipient, reason),
                    ),
                );

                // notify them as of now
                await server.notify(
                    call,
                    { ...call.input, recipients },
                    {
                        occurredAt: call.now,
                        actor: call.caller.subject,
                    },
                );

                return {};
            },
        },
    });
}
