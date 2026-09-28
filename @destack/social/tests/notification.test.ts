import { TEST_DIALECTS } from "@destack/db/test";
import { notification, subscription } from "@destack/notification";
import { expect, test } from "@destack/test";
import { comment, mention, receipt } from "../src/index.ts";
import { article } from "./fixture/article.ts";
import { type Actor, actors, serveArticles } from "./fixture/server.ts";

test.for(TEST_DIALECTS)(
    "notify mentioned principals and announce threads and replies to their subscribers, reading them with a receipt, on %s",
    async (dialect) => {
        const { call, list, host, as, expand, spaceId } = await serveArticles(dialect);
        const inbox = async (actor: Actor) => {
            as(actor);

            return (await list(notification)).map((row) => [
                row.name,
                row.parentType,
                row.reason,
                row.readAt === null ? "unread" : "read",
            ]);
        };

        // share an article with two commenters and a viewer subscribing to it
        const draft = await call(article, "create", { title: "Launch plan" });
        await call(article, "grant", { id: draft.id, relation: "commenter", subject: actors.bob });
        await call(article, "grant", {
            id: draft.id,
            relation: "commenter",
            subject: actors.carol,
        });
        await call(article, "grant", { id: draft.id, relation: "viewer", subject: actors.dave });
        as("dave");
        await call(subscription, "create", host(article, draft.id));

        // start a thread mentioning a commenter, announcing it to the article's other subscribers for their reasons
        as("bob");
        const first = await call(comment, "create", {
            ...host(article, draft.id),
            body: {
                text: "@carol, tighten the intro",
                mentions: [{ offset: 0, length: 6, principal: actors.carol }],
            },
        });
        await expand();
        expect([
            await inbox("alice"),
            await inbox("bob"),
            await inbox("carol"),
            await inbox("dave"),
        ]).toEqual([
            [["thread", "article", "author", "unread"]],
            [],
            [["mention", "article", "mention", "unread"]],
            [["thread", "article", "subscribed", "unread"]],
        ]);

        // carry an excerpt the mentioned principal answers in the thread from the notification
        as("carol");
        const [mentioned] = await list(notification);
        expect([
            mentioned!.payload,
            mentioned!.thread,
            mention.respond(mentioned as never, "reply", "On it"),
        ]).toEqual([
            {
                author: actors.bob,
                comment: first.id,
                thread: first.id,
                text: "@carol, tighten the intro",
            },
            draft.id,
            [
                {
                    method: "comment.create",
                    input: {
                        spaceId,
                        ...host(article, draft.id),
                        body: { text: "On it", mentions: [] },
                        thread: first.id,
                    },
                },
                { method: "notification.read", input: { spaceId, id: mentioned!.id } },
            ],
        ]);

        // announce replies to the thread's subscribers for their reasons, leaving the mentioned to their mention
        await call(comment, "create", {
            ...host(article, draft.id),
            body: { text: "Agreed", mentions: [] },
            thread: first.id,
        });
        await expand();
        as("alice");
        await call(comment, "create", {
            ...host(article, draft.id),
            body: {
                text: "@bob, ship it",
                mentions: [{ offset: 0, length: 4, principal: actors.bob }],
            },
            thread: first.id,
        });
        await expand();
        expect([
            await inbox("alice"),
            await inbox("bob"),
            await inbox("carol"),
            await inbox("dave"),
        ]).toEqual([
            [["thread", "article", "author", "unread"]],
            [
                ["reply", "comment", "author", "unread"],
                ["mention", "comment", "mention", "unread"],
            ],
            [
                ["mention", "article", "mention", "unread"],
                ["reply", "comment", "participating", "unread"],
            ],
            [["thread", "article", "subscribed", "unread"]],
        ]);

        // read every notification threaded on the article with its receipt, leaving others' unread
        as("carol");
        const read = await call(receipt, "create", host(article, draft.id));
        await call(receipt, "update", { id: read.id });
        expect([await inbox("carol"), await inbox("bob")]).toEqual([
            [
                ["mention", "article", "mention", "read"],
                ["reply", "comment", "participating", "read"],
            ],
            [
                ["reply", "comment", "author", "unread"],
                ["mention", "comment", "mention", "unread"],
            ],
        ]);
    },
);
