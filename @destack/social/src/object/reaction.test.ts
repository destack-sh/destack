import { article, TestSocial } from "../test/index.ts";
import { expect, refusal, test } from "@destack/test";
import { comment, reaction } from "./index.ts";

test("let commenters react once per emoji to articles and their comments, counted on each", async () => {
    await using fixture = await TestSocial.open();
    const { scope, actors } = fixture;

    // share an article with a commenter, an agent commenting on it and a viewer, and start a thread
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
            subject: actors.agent,
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
    const first = await fixture.call(
        "alice",
        comment.call("create", {
            ...scope,
            ...TestSocial.host(article, draft.id),
            body: { text: "Ship it?", mentions: [] },
        }),
    );

    // let the commenter react to the article once per emoji, and the agent too
    const thumbs = await fixture.call(
        "bob",
        reaction.call("create", { ...scope, ...TestSocial.host(article, draft.id), emoji: "👍" }),
    );
    expect(
        await refusal(
            fixture.call(
                "bob",
                reaction.call("create", {
                    ...scope,
                    ...TestSocial.host(article, draft.id),
                    emoji: "👍",
                }),
            ),
        ),
    ).toEqual(["CONFLICT", "DUPLICATE", "another reaction has the same parent, author, and emoji"]);
    await fixture.call(
        "bob",
        reaction.call("create", { ...scope, ...TestSocial.host(article, draft.id), emoji: "👨🏻‍❤️‍💋‍👨🏼" }),
    );
    await fixture.call(
        "agent",
        reaction.call("create", { ...scope, ...TestSocial.host(article, draft.id), emoji: "👍" }),
    );
    await fixture.call(
        "agent",
        reaction.call("create", { ...scope, ...TestSocial.host(comment, first.id), emoji: "🎉" }),
    );

    // refuse the viewer a reaction to the article and to its comment, since reactions follow commenting
    expect(
        await refusal(
            fixture.call(
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
            fixture.call(
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
        (await fixture.call("carol", article.call("get", { ...scope, id: draft.id })))
            .reactionCount,
        (await fixture.call("carol", comment.call("get", { ...scope, id: first.id })))
            .reactionCount,
    ]).toEqual([3, 1]);

    // take back a reaction only as its author
    expect(
        await refusal(fixture.call("agent", reaction.call("delete", { ...scope, id: thumbs.id }))),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: write"]);
    await fixture.call("bob", reaction.call("delete", { ...scope, id: thumbs.id }));
    const reactions = (await fixture.list("bob", reaction)).map((row) => [
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
        (await fixture.call("bob", article.call("get", { ...scope, id: draft.id }))).reactionCount,
    ).toBe(2);
});
