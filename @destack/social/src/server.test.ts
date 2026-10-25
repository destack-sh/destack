import { nth, present } from "@destack/schema";
import { activity, subscription } from "@destack/notification";
import { expect, refusal, test } from "@destack/test";
import { Body, comment, type Mention } from "./index.ts";
import { type Actor, article, TestSocial } from "./test/index.ts";

test("record activities for mentioned principals, announce threads and replies to their subscribers, and answer a mention with a reply", async () => {
    await using kit = await TestSocial.open();
    const { scope, actors } = kit;
    const activities = async (actor: Actor) =>
        (await kit.list(actor, activity)).map((row) => [row.name, row.parentType, row.reason]);

    // share an article with two commenters and a viewer subscribing to it
    const draft = await kit.call(
        "alice",
        article.call("create", { ...scope, title: "Launch plan" }),
    );
    await kit.call(
        "alice",
        article.call("grant", {
            ...scope,
            id: draft.id,
            relation: "commenter",
            subject: actors.bob,
        }),
    );
    await kit.call(
        "alice",
        article.call("grant", {
            ...scope,
            id: draft.id,
            relation: "commenter",
            subject: actors.carol,
        }),
    );
    await kit.call(
        "alice",
        article.call("grant", {
            ...scope,
            id: draft.id,
            relation: "viewer",
            subject: actors.dave,
        }),
    );
    await kit.call(
        "dave",
        subscription.call("create", { ...scope, ...TestSocial.host(article, draft.id) }),
    );

    // start a thread mentioning a commenter, announcing it to the article's other subscribers for their reasons
    const first = await kit.call(
        "bob",
        comment.call("create", {
            ...scope,
            ...TestSocial.host(article, draft.id),
            body: {
                text: "@carol, tighten the intro",
                mentions: [{ offset: 0, length: 6, principal: actors.carol }],
            },
        }),
    );
    await kit.settle();
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
    const mentioned = nth(await kit.list("carol", activity), 0);
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
    await kit.call("carol", comment.call("reply", { ...scope, id: first.id, text: "Agreed" }));
    await kit.settle();
    await kit.call(
        "alice",
        comment.call("reply", {
            ...scope,
            id: first.id,
            text: "@bob, ship it",
            mentions: [{ offset: 0, length: 4, principal: actors.bob }],
        }),
    );
    await kit.settle();
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
    const bobMentioned = present(
        (await kit.list("bob", activity)).find((row) => row.name === "mention"),
        "bob's mention",
    );
    await kit.call(
        "bob",
        activity.call("act", {
            ...scope,
            id: bobMentioned.id,
            action: "reply",
            text: "Shipping",
        }),
    );
    const thread = (await kit.list("bob", comment)).map((row) => [
        Body.parse(row.body).text,
        row.authorId,
        row.threadId,
    ]);
    expect(thread).toEqual([
        ["@carol, tighten the intro", actors.bob.id, null],
        ["Agreed", actors.carol.id, first.id],
        ["@bob, ship it", actors.alice.id, first.id],
        ["Shipping", actors.bob.id, first.id],
    ]);
});

test("notify a principal mentioned twice in one comment once, and answer a reply with the reply it creates", async () => {
    await using kit = await TestSocial.open();
    const { scope, actors } = kit;

    // share an article with a commenter, and mention them twice in one comment
    const draft = await kit.call(
        "alice",
        article.call("create", { ...scope, title: "Launch plan" }),
    );
    await kit.call(
        "alice",
        article.call("grant", {
            ...scope,
            id: draft.id,
            relation: "commenter",
            subject: actors.bob,
        }),
    );
    const first = await kit.call(
        "alice",
        comment.call("create", {
            ...scope,
            ...TestSocial.host(article, draft.id),
            body: {
                text: "@bob, @bob: tighten the intro",
                mentions: [
                    { offset: 0, length: 4, principal: actors.bob },
                    { offset: 6, length: 4, principal: actors.bob },
                ],
            },
        }),
    );
    await kit.settle();

    // give bob one mention activity
    const activities = (await kit.list("bob", activity)).map((row) => [
        row.name,
        row.parentType,
        row.reason,
        row.payload,
    ]);
    expect(activities).toEqual([
        [
            "mention",
            "article",
            "mention",
            {
                author: actors.alice,
                comment: first.id,
                thread: first.id,
                text: "@bob, @bob: tighten the intro",
            },
        ],
    ]);

    // reply in the thread, answered with the new reply and not the thread's first comment
    const replied = await kit.call(
        "bob",
        comment.call("reply", { ...scope, id: first.id, text: "Done" }),
    );
    expect([replied.threadId, Body.parse(replied.body).text]).toEqual([first.id, "Done"]);
});

test("subscribe thread authors, repliers and mentioned principals to the thread, each principal's subscription private to it", async () => {
    await using kit = await TestSocial.open();
    const { scope, actors } = kit;
    const subscriptions = async (actor: Actor) =>
        (await kit.list(actor, subscription)).map((row) => [row.parentType, row.reason]);

    // subscribe the article's creator, and let a commenter subscribe before anyone mentions them
    const draft = await kit.call(
        "alice",
        article.call("create", { ...scope, title: "Launch plan" }),
    );
    await kit.call(
        "alice",
        article.call("grant", {
            ...scope,
            id: draft.id,
            relation: "commenter",
            subject: actors.bob,
        }),
    );
    await kit.call(
        "alice",
        article.call("grant", {
            ...scope,
            id: draft.id,
            relation: "viewer",
            subject: actors.carol,
        }),
    );
    await kit.call(
        "alice",
        article.call("grant", {
            ...scope,
            id: draft.id,
            relation: "viewer",
            subject: actors.agent,
        }),
    );
    await kit.call(
        "bob",
        subscription.call("create", { ...scope, ...TestSocial.host(article, draft.id) }),
    );

    // refuse mentioning someone who cannot read the article
    const text = "@bob, @carol and @dave, see the agent's notes";
    const create = (mentions: readonly Mention[]) =>
        kit.call(
            "alice",
            comment.call("create", {
                ...scope,
                ...TestSocial.host(article, draft.id),
                body: { text, mentions: [...mentions] },
            }),
        );
    const bob = { offset: 0, length: 4, principal: actors.bob };
    const carol = { offset: 6, length: 6, principal: actors.carol };
    const dave = { offset: 17, length: 5, principal: actors.dave };
    const strangers = await refusal(create([bob, carol, dave]));

    // subscribe a thread's author to it and the mentioned to the article with their reasons
    const first = await create([bob, carol]);
    expect([
        strangers,
        await subscriptions("alice"),
        await subscriptions("bob"),
        await subscriptions("carol"),
        await subscriptions("dave"),
    ]).toEqual([
        [
            "FORBIDDEN",
            "RECIPIENT_FORBIDDEN",
            `user ${actors.dave.id} cannot read article ${draft.id}`,
        ],
        [
            ["article", "author"],
            ["comment", "author"],
        ],
        [["article", "subscribed"]],
        [["article", "mention"]],
        [],
    ]);

    // subscribe a replier and whom the reply mentions to the thread
    const answer = await kit.call(
        "bob",
        comment.call("reply", {
            ...scope,
            id: first.id,
            text: "@carol, agreed",
            mentions: [{ offset: 0, length: 6, principal: actors.carol }],
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

    // unsubscribe from the thread by deleting, and subscribe only principals an edit mentions for the first time
    const kept = nth(await kit.list("carol", subscription), 1);
    await kit.call(
        "carol",
        subscription.call("delete", {
            ...scope,
            id: present(kept.id, "carol's thread subscription"),
        }),
    );
    await kit.call(
        "bob",
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
    await kit.call(
        "bob",
        comment.call("reply", {
            ...scope,
            id: first.id,
            text: "@carol, one more",
            mentions: [{ offset: 0, length: 6, principal: actors.carol }],
        }),
    );
    expect(await subscriptions("carol")).toEqual([
        ["article", "mention"],
        ["comment", "mention"],
    ]);
});
