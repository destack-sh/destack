import { principal } from "@destack/access";
import { announcement, notification, Subscription, subscription } from "@destack/notification";
import { defineObject, field, method } from "@destack/object";
import { schema } from "@destack/schema";
import { space } from "@destack/space/object";
import { comment, favourite, hostRoles, presence, reaction, receipt } from "../../src/index.ts";

/** The standard roles articles are shared through, with people and agent installations alike. */
const roles = hostRoles([principal.user, principal.installation]);

/** Articles with every social attachment, shared with editors, commenters and viewers. */
export const article = defineObject({
    name: "article",
    plural: "articles",
    scope: space,
    fields: {
        ...roles.fields,
        /** The article title. */
        title: field.string(schema.string().min(1).max(200)),
        /** The article text. */
        body: field.text(),
        /** The number of comments on the article. */
        commentCount: field.count(),
        /** The number of reactions to the article. */
        reactionCount: field.count(),
    },
    relations: roles.relations,
    permissions: roles.permissions,
    shareable: { by: roles.grantedBy },
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
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("manage"),
        update: method.update("edit", { fields: ["title"] }),
        delete: method.delete("manage"),
    },
}).handle({
    create: async (call, next) => {
        // subscribe the owner to the article it creates
        const row = await next();
        await Subscription.add(call, call.reference(), call.caller!, "author");

        return row;
    },
});
