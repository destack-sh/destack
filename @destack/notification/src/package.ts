import { DELIVER_CAPABILITY, PUSH_CAPABILITY } from "./message/recipient.ts";
import { user } from "@destack/account/object";
import { message, messageAttempt } from "@destack/message";
import { build, catalog, declaration } from "@destack/stack/object";
import { definePackage } from "@destack/object";
import { NotificationError } from "./error/error.ts";
import { activity } from "./object/activity.ts";
import { announcement } from "./object/announcement.ts";
import { notification } from "./object/notification.ts";
import { summary } from "./object/summary.ts";
import { subscription } from "./object/subscription.ts";

/** The handle stacks install notifications by, and clients call a space's inbox through. */
export default definePackage({
    objects: {
        activity,
        announcement,
        subscription,
        notification,
        summary,
    },
    permissions: {
        installation: {
            // plan notifications and summaries, expand announcements and send their messages in the space
            space: [
                notification.permission("plan"),
                summary.permission("plan"),
                announcement.permission("expand"),
                message.permission("send"),
                message.permission("read"),
                message.permission("control"),
                messageAttempt.permission("record"),
                // read the notifications installed builds declare, from their index
                build.permission("read"),
                declaration.permission("read"),
                catalog.permission("read"),
            ],
        },
    },
    capabilities: {
        // deliver a person's notifications from their home
        [DELIVER_CAPABILITY]: {
            reason: "Deliver your notifications by email, push and on your desktops",
            optional: true,
            permissions: { installation: { universe: [user.permission("recipient")] } },
        },
        // push to people's browsers
        [PUSH_CAPABILITY]: {
            reason: "Send notifications to your browsers and devices as pushes",
            optional: true,
        },
    },
    errors: NotificationError,
});
