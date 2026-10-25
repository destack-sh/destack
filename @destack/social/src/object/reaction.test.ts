import { article, TestSocial } from "../test/index.ts";
import { expect, refusal, test } from "@destack/test";
import { comment, reaction } from "./index.ts";

test("let commenters react once per emoji to articles and their comments, counted on each", async () => {
    await using kit = await TestSocial.open();
    const { scope, actors } = kit;

    // share an article with a commenter, an agent commenting on it and a viewer, and start a thread
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
            subject: actors.agent,
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
    const first = await kit.call(
        "alice",
        comment.call("create", {
            ...scope,
            ...TestSocial.host(article, draft.id),
            body: { text: "Ship it?", mentions: [] },
        }),
    );

    // let the commenter react to the article once per emoji, and the agent too
    const thumbs = await kit.call(
        "bob",
        reaction.call("create", { ...scope, ...TestSocial.host(article, draft.id), emoji: "👍" }),
    );
    expect(
        await refusal(
            kit.call(
                "bob",
                reaction.call("create", {
                    ...scope,
                    ...TestSocial.host(article, draft.id),
                    emoji: "👍",
                }),
            ),
        ),
    ).toEqual(["CONFLICT", "DUPLICATE", "another reaction has the same parent, author, and emoji"]);
    await kit.call(
        "bob",
        reaction.call("create", { ...scope, ...TestSocial.host(article, draft.id), emoji: "👨🏻‍❤️‍💋‍👨🏼" }),
    );
    await kit.call(
        "agent",
        reaction.call("create", { ...scope, ...TestSocial.host(article, draft.id), emoji: "👍" }),
    );
    await kit.call(
        "agent",
        reaction.call("create", { ...scope, ...TestSocial.host(comment, first.id), emoji: "🎉" }),
    );

    // refuse the viewer a reaction to the article and to its comment, since reactions follow commenting
    expect(
        await refusal(
            kit.call(
                "carol",
                reaction.call("create", {
                    ...scope,
                    ...TestSocial.host(article, draft.id),
                    emoji: "🎉",
                }),
            ),
        ),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: react"]);
    expect(
        await refusal(
            kit.call(
                "carol",
                reaction.call("create", {
                    ...scope,
                    ...TestSocial.host(comment, first.id),
                    emoji: "🎉",
                }),
            ),
        ),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: react"]);

    // count the reactions on the article and on the comment
    expect([
        (await kit.call("carol", article.call("get", { ...scope, id: draft.id }))).reactionCount,
        (await kit.call("carol", comment.call("get", { ...scope, id: first.id }))).reactionCount,
    ]).toEqual([3, 1]);

    // take back a reaction only as its author
    expect(
        await refusal(kit.call("agent", reaction.call("delete", { ...scope, id: thumbs.id }))),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: write"]);
    await kit.call("bob", reaction.call("delete", { ...scope, id: thumbs.id }));
    const reactions = (await kit.list("bob", reaction)).map((row) => [
        row.parentType,
        row.emoji,
        row.authorId,
    ]);
    expect(reactions).toEqual([
        ["article", "👨🏻‍❤️‍💋‍👨🏼", actors.bob.id],
        ["article", "👍", actors.agent.id],
        ["comment", "🎉", actors.agent.id],
    ]);
    expect(
        (await kit.call("bob", article.call("get", { ...scope, id: draft.id }))).reactionCount,
    ).toBe(2);
});
