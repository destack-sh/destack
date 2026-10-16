import type { Call } from "@destack/sync";
import { TEST_DIALECTS } from "@destack/db/test";
import { aligned, present } from "@destack/schema";
import { subscription } from "@destack/notification";
import { expect, refusal, test } from "@destack/test";
import { comment } from "../src/index.ts";
import { article } from "./fixture/article.ts";
import { type Actor, actors, serveArticles } from "./fixture/server.ts";

test.for(TEST_DIALECTS)(
    "subscribe thread authors, repliers and mentioned principals to the thread, each principal's subscription private to it, on %s",
    async (dialect) => {
        const { call, list, host, as, scope } = await serveArticles(dialect);
        const subscriptions = async (actor: Actor) => {
            as(actor);

            return (await list(subscription)).map((row) => [row.parentType, row.reason]);
        };

        // subscribe the article's creator, and let a commenter subscribe before anyone mentions them
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
                relation: "viewer",
                subject: actors.carol,
            }),
        );
        await call(
            article.call("grant", {
                ...scope,
                id: draft.id,
                relation: "viewer",
                subject: actors.agent,
            }),
        );
        as("bob");
        await call(subscription.call("create", { ...scope, ...host(article, draft.id) }));
        expect(
            await refusal(
                call(
                    withReason(
                        subscription.call("create", { ...scope, ...host(article, draft.id) }),
                    ),
                ),
            ),
        ).toEqual(["BAD_REQUEST", 'invalid input: unrecognized key: "reason"']);

        // subscribe a thread's author to it and the mentioned to the article with their reasons
        as("alice");
        const text = "@bob, @carol and @dave, see the agent's notes";
        const first = await call(
            comment.call("create", {
                ...scope,
                ...host(article, draft.id),
                body: {
                    text,
                    mentions: [
                        { offset: 0, length: 4, principal: actors.bob },
                        { offset: 6, length: 6, principal: actors.carol },
                        { offset: 17, length: 5, principal: actors.dave },
                    ],
                },
            }),
        );
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
        const answer = await call(
            comment.call("create", {
                ...scope,
                ...host(article, draft.id),
                body: {
                    text: "@carol, agreed",
                    mentions: [{ offset: 0, length: 6, principal: actors.carol }],
                },
                threadId: first.id,
            }),
        );
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
        await call(
            subscription.call("delete", { ...scope, id: present(kept.id, "carol's subscription") }),
        );
        as("bob");
        await call(
            comment.call("update", {
                ...scope,
                id: answer.id,
                body: {
                    text: "@carol, agreed with @agent",
                    mentions: [
                        { offset: 0, length: 6, principal: actors.carol },
                        { offset: 20, length: 6, principal: actors.agent },
                    ],
                },
            }),
        );
        expect([await subscriptions("carol"), await subscriptions("agent")]).toEqual([
            [["article", "mention"]],
            [["comment", "mention"]],
        ]);

        // subscribe an unsubscribed principal to the thread again once a new reply mentions it
        as("bob");
        await call(
            comment.call("create", {
                ...scope,
                ...host(article, draft.id),
                body: {
                    text: "@carol, one more",
                    mentions: [{ offset: 0, length: 6, principal: actors.carol }],
                },
                threadId: first.id,
            }),
        );
        expect(await subscriptions("carol")).toEqual([
            ["article", "mention"],
            ["comment", "mention"],
        ]);
    },
);

/** Add a reason to a subscription's creation, as a client sending more than the method takes. */
function withReason<Result>(created: Call<Result>): Call<Result> {
    return { ...created, input: { ...created.input, reason: "author" } };
}
