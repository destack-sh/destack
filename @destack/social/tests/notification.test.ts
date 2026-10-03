import { TEST_DIALECTS } from "@destack/db/test";
import { aligned } from "@destack/schema";
import { Subject } from "@destack/sync";
import { notification, subscription } from "@destack/notification";
import { expect, test } from "@destack/test";
import { Body, comment, receipt } from "../src/index.ts";
import { article } from "./fixture/article.ts";
import { type Actor, actors, serveArticles } from "./fixture/server.ts";

test.for(TEST_DIALECTS)(
    "notify mentioned principals and announce threads and replies to their subscribers, reading them with a receipt, on %s",
    async (dialect) => {
        const { call, list, host, as, expand } = await serveArticles(dialect);
        const inbox = async (actor: Actor) => {
            as(actor);

            return (await list(notification)).map((row) => [
                row["name"],
                row["parentType"],
                row["reason"],
                row["readAt"] === null ? "unread" : "read",
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

        // carry an excerpt of the mentioning comment and its thread
        as("carol");
        const mentioned = aligned(await list(notification), 0);
        expect([mentioned["payload"], mentioned["thread"]]).toEqual([
            {
                author: actors.bob,
                comment: first.id,
                thread: first.id,
                text: "@carol, tighten the intro",
            },
            draft.id,
        ]);

        // announce replies to the thread's subscribers for their reasons except the mentioned
        await call(comment, "create", {
            ...host(article, draft.id),
            body: { text: "Agreed", mentions: [] },
            threadId: first.id,
        });
        await expand();
        as("alice");
        await call(comment, "create", {
            ...host(article, draft.id),
            body: {
                text: "@bob, ship it",
                mentions: [{ offset: 0, length: 4, principal: actors.bob }],
            },
            threadId: first.id,
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

        // read every notification on the article with its receipt and leave others' unread
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

        // answer bob's mention from the notification, replying in its thread and reading it
        as("bob");
        const bobMentioned = (await list(notification)).find((row) => row["name"] === "mention");
        if (bobMentioned === undefined) {
            throw new TypeError("bob has no mention");
        }
        await call(notification, "act", {
            id: bobMentioned.id,
            action: "reply",
            text: "Shipping",
        });
        const replies = (await list(comment))
            .filter((row) => row["threadId"] === first.id)
            .map((row) => [row["author"], Body.parse(row["body"]).text]);
        expect([replies.at(-1), await inbox("bob")]).toEqual([
            [Subject.key(actors.bob), "Shipping"],
            [
                ["reply", "comment", "author", "unread"],
                ["mention", "comment", "mention", "read"],
            ],
        ]);
    },
);
