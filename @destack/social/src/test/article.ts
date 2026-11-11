import { space } from "@destack/account/object";
import { activity, announcement, Subscription, subscription } from "@destack/notification/object";
import { defineObject, field, Package } from "@destack/object";
import { implementPackage } from "@destack/object/server";
import { PackageVersion } from "@destack/package";
import { schema } from "@destack/schema";
import { comment, favourite, presence, reaction, receipt } from "../object/index.ts";
import { shareable } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** The articles application, whose articles carry every social attachment. */
export const ARTICLES = PackageVersion.parse({
    id: "package-019f5530-8000-7000-8000-0000000000e2",
    name: "@example/articles",
    version: "2026.10.0",
});

/** Articles with every social attachment, shared with editors, commenters and viewers. */
export const article = defineObject(
    {
        name: "article",
        plural: "articles",
        title: t`${plural(argument("count"), { one: "Article", other: "Articles" })}`,
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
        attachments: [
            comment.attach({ by: "comment" }),
            reaction.attach({ by: "comment" }),
            activity.attach({ by: "read" }),
            announcement.attach({ by: "read" }),
            subscription.attach({ by: "read" }),
            receipt.attach({ by: "read" }),
            favourite.attach({ by: "read" }),
            presence.attach({ by: "read" }),
        ],
        traits: [shareable({})],
    },
    (method) => ({
        create: method.create("manage"),
        update: method.update("write", { fields: ["title"] }),
        delete: method.delete("manage"),
    }),
    { package: ARTICLES },
);

/** The articles package's handle. */
export const articlesPackage = Package.of(ARTICLES, { objects: [article] });

/** The articles package's server, subscribing each article's creator to it. */
export const articlesServer = implementPackage(articlesPackage, {
    objects: [
        article.implement({
            methods: {
                create: async (call, next) => {
                    // subscribe the creator to the article
                    const row = await next();
                    const host = call.reference();
                    await call.mutate(call.caller, [
                        Subscription.subscribe(host, call.caller.subject, "author"),
                    ]);

                    return row;
                },
            },
        }),
    ],
});
