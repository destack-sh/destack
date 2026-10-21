import { TEST_DIALECTS } from "@destack/db/test";
import { nth, present } from "@destack/schema";
import { activity, subscription } from "@destack/notification";
import { expect, test } from "@destack/test";
import { Body, comment } from "../src/index.ts";
import { article } from "./fixture/article.ts";
import { type Actor, actors, serveArticles } from "./fixture/server.ts";

test.for(TEST_DIALECTS)(
    "record activities for mentioned principals, announce threads and replies to their subscribers, and answer a mention with a reply, on %s",
    async (dialect) => {
        const { call, list, host, as, expand, scope } = await serveArticles(dialect);
        const activities = async (actor: Actor) => {
            as(actor);

            return (await list(activity)).map((row) => [row.name, row.parentType, row.reason]);
        };

        // share an article with two commenters and a viewer subscribing to it
        const draft = await call(article.call("create", { ...scope, title: "Launch plan" }));
        await call(
            article.call("grant", {
                ...scope,
                id: draft.id,
                relation: "commenter",
                subject: actors.bob,
            }),
        );
        await call(
            article.call("grant", {
                ...scope,
                id: draft.id,
                relation: "commenter",
                subject: actors.carol,
            }),
        );
        await call(
            article.call("grant", {
                ...scope,
                id: draft.id,
                relation: "viewer",
                subject: actors.dave,
            }),
        );
        as("dave");
        await call(subscription.call("create", { ...scope, ...host(article, draft.id) }));

        // start a thread mentioning a commenter, announcing it to the article's other subscribers for their reasons
        as("bob");
        const first = await call(
            comment.call("create", {
                ...scope,
                ...host(article, draft.id),
                body: {
                    text: "@carol, tighten the intro",
                    mentions: [{ offset: 0, length: 6, principal: actors.carol }],
                },
            }),
        );
        await expand();
        expect([
            await activities("alice"),
            await activities("bob"),
            await activities("carol"),
            await activities("dave"),
        ]).toEqual([
            [["thread", "article", "author"]],
            [],
            [["mention", "article", "mention"]],
            [["thread", "article", "subscribed"]],
        ]);

        // carry an excerpt of the mentioning comment and its thread
        as("carol");
        const mentioned = nth(await list(activity), 0);
        expect([mentioned.payload, mentioned.thread]).toEqual([
            {
                author: actors.bob,
                comment: first.id,
                thread: first.id,
                text: "@carol, tighten the intro",
            },
            draft.id,
        ]);

        // announce replies to the thread's subscribers for their reasons except the mentioned
        await call(
            comment.call("create", {
                ...scope,
                ...host(article, draft.id),
                body: { text: "Agreed", mentions: [] },
                threadId: first.id,
            }),
        );
        await expand();
        as("alice");
        await call(
            comment.call("create", {
                ...scope,
                ...host(article, draft.id),
                body: {
                    text: "@bob, ship it",
                    mentions: [{ offset: 0, length: 4, principal: actors.bob }],
                },
                threadId: first.id,
            }),
        );
        await expand();
        expect([
            await activities("alice"),
            await activities("bob"),
            await activities("carol"),
            await activities("dave"),
        ]).toEqual([
            [["thread", "article", "author"]],
            [
                ["reply", "comment", "author"],
                ["mention", "comment", "mention"],
            ],
            [
                ["mention", "article", "mention"],
                ["reply", "comment", "participating"],
            ],
            [["thread", "article", "subscribed"]],
        ]);

        // answer bob's mention from its activity, replying in its thread
        as("bob");
        const bobMentioned = (await list(activity)).find((row) => row.name === "mention");
        if (bobMentioned === undefined) {
            throw new TypeError("bob has no mention");
        }
        await call(
            activity.call("act", {
                ...scope,
                id: present(bobMentioned.id, "bob's mention"),
                action: "reply",
                text: "Shipping",
            }),
        );
        const replies = (await list(comment))
            .filter((row) => row.threadId === first.id)
            .map((row) => [row.authorId, Body.parse(row.body).text]);
        expect(replies.at(-1)).toEqual([actors.bob.id, "Shipping"]);
    },
);

test.for(TEST_DIALECTS)(
    "notify a principal mentioned twice in one comment once, and answer a reply with the reply it creates, on %s",
    async (dialect) => {
        const { call, list, host, as, expand, scope } = await serveArticles(dialect);

        // share an article with a commenter, and mention them twice in one comment
        const draft = await call(article.call("create", { ...scope, title: "Launch plan" }));
        await call(
            article.call("grant", {
                ...scope,
                id: draft.id,
                relation: "commenter",
                subject: actors.bob,
            }),
        );
        const first = await call(
            comment.call("create", {
                ...scope,
                ...host(article, draft.id),
                body: {
                    text: "@bob, @bob: tighten the intro",
                    mentions: [
                        { offset: 0, length: 4, principal: actors.bob },
                        { offset: 6, length: 4, principal: actors.bob },
                    ],
                },
            }),
        );
        await expand();

        // give bob one mention activity
        as("bob");
        const mentions = (await list(activity)).filter((row) => row.name === "mention");
        expect(mentions.length).toBe(1);

        // reply in the thread, answered with the new reply and not the thread's first comment
        const replied = await call(comment.call("reply", { ...scope, id: first.id, text: "Done" }));
        expect([replied.threadId, Body.parse(replied.body).text]).toEqual([first.id, "Done"]);
    },
);
