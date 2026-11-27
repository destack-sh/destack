import { nth, present, schema } from "@destack/schema";
import { activity, subscription } from "@destack/notification/object";
import { expect, test } from "@destack/test";
import { TextSpan } from "@destack/object";
import { comment } from "./object/index.ts";
import { type Actor, article, type Mentioned, TestSocial } from "./test/index.ts";

test("record activities for mentioned principals, announce threads and replies to their subscribers, and answer a mention with a reply", async () => {
    await using fixture = await TestSocial.open();
    const { scope, actors } = fixture;
    const activities = async (actor: Actor) =>
        (await fixture.list(actor, activity)).map((row) => [row.name, row.parentType, row.reason]);

    // share an article with two commenters and a viewer subscribing to it
    const draft = await fixture.call(
        "alice",
        article.call("create", { ...scope, title: "Launch plan" }),
    );
    await fixture.call(
        "alice",
        article.call("grant", {
            ...scope,
            id: draft.id,
            relation: "commenter",
            subject: actors.bob,
        }),
    );
    await fixture.call(
        "alice",
        article.call("grant", {
            ...scope,
            id: draft.id,
            relation: "commenter",
            subject: actors.carol,
        }),
    );
    await fixture.call(
        "alice",
        article.call("grant", {
            ...scope,
            id: draft.id,
            relation: "viewer",
            subject: actors.dave,
        }),
    );
    await fixture.call(
        "dave",
        subscription.call("create", { ...scope, ...TestSocial.host(article, draft.id) }),
    );

    // start a thread mentioning a commenter, announcing it to the article's other subscribers for their reasons
    const first = await fixture.call(
        "bob",
        comment.call("create", {
            ...scope,
            ...TestSocial.host(article, draft.id),
            body: [mentionOf(actors.carol), { text: ", tighten the intro" }],
        }),
    );
    await fixture.simulation.idle();
    expect([
        await activities("alice"),
        await activities("bob"),
        await activities("carol"),
        await activities("dave"),
    ]).toEqual([
        [["thread", "article", "author"]],
        [],
        [["mentioned", "article", "mention"]],
        [["thread", "article", "subscribed"]],
    ]);

    // carry who mentioned carol and the start of the comment's text
    const mentioned = nth(await fixture.list("carol", activity), 0);
    expect([mentioned.payload, mentioned.thread]).toEqual([
        { author: actors.bob, text: "\uFFFC, tighten the intro" },
        draft.id,
    ]);

    // announce replies to the thread's subscribers for their reasons except the mentioned
    await fixture.call(
        "carol",
        comment.call("reply", { ...scope, id: first.id, body: [{ text: "Agreed" }] }),
    );
    await fixture.simulation.idle();
    await fixture.call(
        "alice",
        comment.call("reply", {
            ...scope,
            id: first.id,
            body: [mentionOf(actors.bob), { text: ", ship it" }],
        }),
    );
    await fixture.simulation.idle();
    expect([
        await activities("alice"),
        await activities("bob"),
        await activities("carol"),
        await activities("dave"),
    ]).toEqual([
        [["thread", "article", "author"]],
        [
            ["reply", "comment", "author"],
            ["mentioned", "comment", "mention"],
        ],
        [
            ["mentioned", "article", "mention"],
            ["reply", "comment", "participating"],
        ],
        [["thread", "article", "subscribed"]],
    ]);

    // answer the reply from bob's activity, replying in its thread
    const bobReply = present(
        (await fixture.list("bob", activity)).find((row) => row.name === "reply"),
        "bob's reply notice",
    );
    await fixture.call(
        "bob",
        activity.call("act", {
            ...scope,
            id: bobReply.id,
            action: "comment.reply",
            text: "Shipping",
        }),
    );
    const thread = (await fixture.list("bob", comment)).map((row) => [
        TextSpan.plain(schema.array(TextSpan).parse(row["body"])),
        row.authorId,
        row.threadId,
    ]);
    expect(thread).toEqual([
        ["\uFFFC, tighten the intro", actors.bob.id, null],
        ["Agreed", actors.carol.id, first.id],
        ["\uFFFC, ship it", actors.alice.id, first.id],
        ["Shipping", actors.bob.id, first.id],
    ]);
});

test("notify a principal mentioned twice in one comment once, and answer a reply with the reply it creates", async () => {
    await using fixture = await TestSocial.open();
    const { scope, actors } = fixture;

    // share an article with a commenter, and mention them twice in one comment
    const draft = await fixture.call(
        "alice",
        article.call("create", { ...scope, title: "Launch plan" }),
    );
    await fixture.call(
        "alice",
        article.call("grant", {
            ...scope,
            id: draft.id,
            relation: "commenter",
            subject: actors.bob,
        }),
    );
    const first = await fixture.call(
        "alice",
        comment.call("create", {
            ...scope,
            ...TestSocial.host(article, draft.id),
            body: [
                mentionOf(actors.bob),
                { text: ", " },
                mentionOf(actors.bob),
                { text: ": tighten the intro" },
            ],
        }),
    );
    await fixture.simulation.idle();

    // give bob one mention activity
    const activities = (await fixture.list("bob", activity)).map((row) => [
        row.name,
        row.parentType,
        row.reason,
        row.payload,
    ]);
    expect(activities).toEqual([
        [
            "mentioned",
            "article",
            "mention",
            { author: actors.alice, text: "\uFFFC, \uFFFC: tighten the intro" },
        ],
    ]);

    // reply in the thread, answered with the new reply and not the thread's first comment
    const replied = await fixture.call(
        "bob",
        comment.call("reply", { ...scope, id: first.id, body: [{ text: "Done" }] }),
    );
    const body = schema.array(TextSpan).parse(Reflect.get(replied, "body"));
    expect([replied.threadId, TextSpan.plain(body)]).toEqual([first.id, "Done"]);
});

test("subscribe a thread's author, repliers and newly mentioned readers, each privately, skip a mentioned stranger, and resubscribe on a new mention", async () => {
    await using fixture = await TestSocial.open();
    const { scope, actors } = fixture;
    const subscriptions = async (actor: Actor) =>
        (await fixture.list(actor, subscription)).map((row) => [row.parentType, row.reason]);

    // subscribe the article's creator, and let a commenter subscribe before anyone mentions them
    const draft = await fixture.call(
        "alice",
        article.call("create", { ...scope, title: "Launch plan" }),
    );
    await fixture.call(
        "alice",
        article.call("grant", {
            ...scope,
            id: draft.id,
            relation: "commenter",
            subject: actors.bob,
        }),
    );
    await fixture.call(
        "alice",
        article.call("grant", {
            ...scope,
            id: draft.id,
            relation: "viewer",
            subject: actors.carol,
        }),
    );
    await fixture.call(
        "alice",
        article.call("grant", {
            ...scope,
            id: draft.id,
            relation: "viewer",
            subject: actors.agent,
        }),
    );
    await fixture.call(
        "bob",
        subscription.call("create", { ...scope, ...TestSocial.host(article, draft.id) }),
    );

    // write a comment mentioning someone who cannot read the article, notifying only its readers
    const first = await fixture.call(
        "alice",
        comment.call("create", {
            ...scope,
            ...TestSocial.host(article, draft.id),
            body: [
                mentionOf(actors.bob),
                { text: ", " },
                mentionOf(actors.carol),
                { text: " and " },
                mentionOf(actors.dave),
                { text: ", see the agent's notes" },
            ],
        }),
    );
    await fixture.simulation.idle();

    // subscribe a thread's author to it and the mentioned readers to the article with their reasons
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
    const answer = await fixture.call(
        "bob",
        comment.call("reply", {
            ...scope,
            id: first.id,
            body: [mentionOf(actors.carol), { text: ", agreed" }],
        }),
    );
    await fixture.simulation.idle();
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
    const kept = nth(await fixture.list("carol", subscription), 1);
    await fixture.call(
        "carol",
        subscription.call("delete", {
            ...scope,
            id: present(kept.id, "carol's thread subscription"),
        }),
    );
    await fixture.call(
        "bob",
        comment.call("edit", {
            ...scope,
            id: answer.id,
            field: "body",
            edits: [
                { insert: { type: "mention", value: actors.agent }, run: "bob.edit", start: 0 },
            ],
        }),
    );
    await fixture.simulation.idle();
    expect([await subscriptions("carol"), await subscriptions("agent")]).toEqual([
        [["article", "mention"]],
        [["comment", "mention"]],
    ]);

    // subscribe an unsubscribed principal to the thread again once a new reply mentions it
    await fixture.call(
        "bob",
        comment.call("reply", {
            ...scope,
            id: first.id,
            body: [mentionOf(actors.carol), { text: ", one more" }],
        }),
    );
    await fixture.simulation.idle();
    expect(await subscriptions("carol")).toEqual([
        ["article", "mention"],
        ["comment", "mention"],
    ]);
});

/** Mention a principal in a text, as a rich text's mention node. */
function mentionOf(subject: Mentioned): TextSpan {
    return { node: { type: "mention", value: subject } };
}
