import { PUSH_CAPABILITY } from "./message/recipient.ts";
import { client, user } from "@destack/account/object";
import { message, messageAttempt } from "@destack/message";
import { definePackage } from "@destack/runtime";
import { activity } from "./object/activity.ts";
import { announcement } from "./object/announcement.ts";
import { deliver, notification, scheduled, summarize } from "./object/notification.ts";
import { subscription } from "./object/subscription.ts";

/** The handle stacks install notifications by, and clients call a space's inbox through. */
export default definePackage({
    objects: {
        activity,
        announcement,
        subscription,
        notification,
        ...deliver.objects,
        ...summarize.objects,
        ...scheduled.objects,
    },
    permissions: {
        installation: {
            // plan notifications, expand announcements and send their messages in the space
            space: [
                notification.permission("plan"),
                announcement.permission("expand"),
                message.permission("send"),
                message.permission("read"),
                message.permission("control"),
                messageAttempt.permission("record"),
            ],
            // read where people receive notifications
            universe: [user.permission("recipient")],
        },
    },
    capabilities: {
        // push to people's browsers, dropping those their push services forgot
        [PUSH_CAPABILITY]: {
            reason: "Send notifications to your browsers and devices as pushes",
            optional: true,
            permissions: { installation: { universe: [client.permission("unsubscribe")] } },
        },
    },
});
