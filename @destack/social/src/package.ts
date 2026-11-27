import { t } from "@destack/locale";
import { definePackage, textChunk } from "@destack/object";
import { activity, subscription } from "@destack/notification/object";
import { SocialError } from "./error/error.ts";
import { comment, favourite, mentioning, presence, reaction, receipt } from "./object/index.ts";

/** The social package: comments, reactions, read receipts, favourites and presence. */
export default definePackage({
    title: t.context("title")`Social`,
    description: t`Attach comments, reactions, receipts, favourites and presence to any object, notifying mentioned and participating principals.`,
    icon: "chats-teardrop",
    color: "purple",
    objects: [comment, reaction, receipt, favourite, presence, ...mentioning.objects],
    access: {
        permissions: {
            // read the text a change mentions someone in, and notify them, for the editor whose change it is
            user: {
                space: [
                    textChunk.permission("read"),
                    comment.permission("read"),
                    subscription.permission("subscribe"),
                    activity.permission("post"),
                ],
            },
        },
    },
    errors: SocialError,
});
