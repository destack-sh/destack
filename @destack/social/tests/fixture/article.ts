import { announcement, notification, Subscription, subscription } from "@destack/notification";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { space } from "@destack/space/object";
import { comment, favourite, presence, reaction, receipt } from "../../src/index.ts";

/** Articles with every social attachment, shared with editors, commenters and viewers. */
export const article = defineObject({
    name: "article",
    plural: "articles",
    scope: space,
    fields: {
        /** The article title. */
        title: field.string(schema.string().min(1).max(200)),
        /** The article text. */
        body: field.text(),
        /** The number of comments on the article. */
        commentCount: field.count(),
        /** The number of reactions to the article. */
        reactionCount: field.count(),
    },
    shareable: {},
    attachments: [
        comment.attach({ by: "comment" }),
        reaction.attach({ by: "comment" }),
        notification.attach({ by: "read" }),
        announcement.attach({ by: "read" }),
        subscription.attach({ by: "read" }),
        receipt.attach({ by: "read" }),
        favourite.attach({ by: "read" }),
        presence.attach({ by: "read" }),
    ],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("manage"),
        update: method.update("edit", { fields: ["title"] }),
        delete: method.delete("manage"),
    }),
}).handle({
    create: async (call, next) => {
        // subscribe the owner to the article it creates
        const row = await next();
        await Subscription.add(call, call.reference(), call.requireCaller(), "author");

        return row;
    },
});
