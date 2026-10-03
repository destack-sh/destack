import { TEST_DIALECTS } from "@destack/db/test";
import { aligned } from "@destack/schema";
import { subscription } from "@destack/notification";
import { expect, refusal, test } from "@destack/test";
import { comment } from "../src/index.ts";
import { article } from "./fixture/article.ts";
import { type Actor, actors, serveArticles } from "./fixture/server.ts";

test.for(TEST_DIALECTS)(
    "subscribe thread authors, repliers and mentioned principals to the thread, keeping each principal's own subscription private, on %s",
    async (dialect) => {
        const { call, list, host, as } = await serveArticles(dialect);
        const subscriptions = async (actor: Actor) => {
            as(actor);

            return (await list(subscription)).map((row) => [row.parentType, row.reason]);
        };

        // subscribe the article's creator, and let a commenter subscribe before anyone mentions them
        const draft = await call(article, "create", { title: "Launch plan" });
        await call(article, "grant", { id: draft.id, relation: "commenter", subject: actors.bob });
        await call(article, "grant", { id: draft.id, relation: "viewer", subject: actors.carol });
        await call(article, "grant", { id: draft.id, relation: "viewer", subject: actors.agent });
        as("bob");
        await call(subscription, "create", host(article, draft.id));
        expect(
            await refusal(
                call(subscription, "create", { ...host(article, draft.id), reason: "author" }),
            ),
        ).toEqual(["BAD_REQUEST", "invalid input to subscription.create"]);

        // subscribe a thread's author to it and the mentioned to the article with their reasons
        as("alice");
        const text = "@bob, @carol and @dave, see the agent's notes";
        const first = await call(comment, "create", {
            ...host(article, draft.id),
            body: {
                text,
                mentions: [
                    { offset: 0, length: 4, principal: actors.bob },
                    { offset: 6, length: 6, principal: actors.carol },
                    { offset: 17, length: 5, principal: actors.dave },
                ],
            },
        });
        expect([
            await subscriptions("alice"),
            await subscriptions("bob"),
            await subscriptions("carol"),
            await subscriptions("dave"),
        ]).toEqual([
            [
                ["article", "author"],
                ["comment", "author"],
            ],
            [["article", "subscribed"]],
            [["article", "mention"]],
            [],
        ]);

        // subscribe a replier and whom the reply mentions to the thread
        as("bob");
        const answer = await call(comment, "create", {
            ...host(article, draft.id),
            body: {
                text: "@carol, agreed",
                mentions: [{ offset: 0, length: 6, principal: actors.carol }],
            },
            threadId: first.id,
        });
        expect([await subscriptions("bob"), await subscriptions("carol")]).toEqual([
            [
                ["article", "subscribed"],
                ["comment", "participating"],
            ],
            [
                ["article", "mention"],
                ["comment", "mention"],
            ],
        ]);

        // unsubscribe by deleting, and subscribe only principals an edit mentions for the first time
        as("carol");
        const kept = aligned(await list(subscription), 1);
        await call(subscription, "delete", { id: kept.id });
        as("bob");
        await call(comment, "update", {
            id: answer.id,
            body: {
                text: "@carol, agreed with @agent",
                mentions: [
                    { offset: 0, length: 6, principal: actors.carol },
                    { offset: 20, length: 6, principal: actors.agent },
                ],
            },
        });
        expect([await subscriptions("carol"), await subscriptions("agent")]).toEqual([
            [["article", "mention"]],
            [["comment", "mention"]],
        ]);

        // subscribe an unsubscribed principal to the thread again once a new reply mentions it
        as("bob");
        await call(comment, "create", {
            ...host(article, draft.id),
            body: {
                text: "@carol, one more",
                mentions: [{ offset: 0, length: 6, principal: actors.carol }],
            },
            threadId: first.id,
        });
        expect(await subscriptions("carol")).toEqual([
            ["article", "mention"],
            ["comment", "mention"],
        ]);
    },
);
