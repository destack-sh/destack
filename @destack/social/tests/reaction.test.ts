import { TEST_DIALECTS } from "@destack/db/test";
import { expect, refusal, test } from "@destack/test";
import { comment, reaction } from "../src/index.ts";
import { article } from "./fixture/article.ts";
import { actors, serveArticles } from "./fixture/server.ts";

test.for(TEST_DIALECTS)(
    "let commenters react once per emoji to articles and their comments, counted on each, on %s",
    async (dialect) => {
        const { call, list, host, as, scope } = await serveArticles(dialect);

        // share an article with a commenter, an agent commenting on it and a viewer, and start a thread
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
                subject: actors.agent,
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
        const first = await call(
            comment.call("create", {
                ...scope,
                ...host(article, draft.id),
                body: { text: "Ship it?", mentions: [] },
            }),
        );

        // let the commenter react to the article once per emoji, and the agent too
        as("bob");
        const thumbs = await call(
            reaction.call("create", { ...scope, ...host(article, draft.id), emoji: "👍" }),
        );
        expect(
            await refusal(
                call(
                    reaction.call("create", { ...scope, ...host(article, draft.id), emoji: "👍" }),
                ),
            ),
        ).toEqual(["CONFLICT", "another reaction has the same parent, author, and emoji"]);
        await call(reaction.call("create", { ...scope, ...host(article, draft.id), emoji: "👨🏻‍❤️‍💋‍👨🏼" }));
        expect(
            await refusal(
                call(
                    reaction.call("create", { ...scope, ...host(article, draft.id), emoji: "yes" }),
                ),
            ),
        ).toEqual(["BAD_REQUEST", "invalid input: emoji: invalid emoji"]);
        as("agent");
        await call(reaction.call("create", { ...scope, ...host(article, draft.id), emoji: "👍" }));
        await call(reaction.call("create", { ...scope, ...host(comment, first.id), emoji: "🎉" }));

        // refuse the viewer a reaction to the article and to its comment, since reactions follow commenting
        as("carol");
        expect(
            await refusal(
                call(
                    reaction.call("create", { ...scope, ...host(article, draft.id), emoji: "🎉" }),
                ),
            ),
        ).toEqual(["FORBIDDEN", "permission denied: react"]);
        expect(
            await refusal(
                call(
                    reaction.call("create", { ...scope, ...host(comment, first.id), emoji: "🎉" }),
                ),
            ),
        ).toEqual(["FORBIDDEN", "permission denied: react"]);

        // count the reactions on the article and on the comment
        expect([
            (await call(article.call("get", { ...scope, id: draft.id }))).reactionCount,
            (await call(comment.call("get", { ...scope, id: first.id }))).reactionCount,
        ]).toEqual([3, 1]);

        // take back a reaction only as its author
        as("agent");
        expect(await refusal(call(reaction.call("delete", { ...scope, id: thumbs.id })))).toEqual([
            "FORBIDDEN",
            "permission denied: write",
        ]);
        as("bob");
        await call(reaction.call("delete", { ...scope, id: thumbs.id }));
        expect(
            (await list(reaction)).map((row) => [row.authorId, row.parentType, row.emoji]),
        ).toEqual([
            [actors.bob.id, "article", "👨🏻‍❤️‍💋‍👨🏼"],
            [actors.agent.id, "article", "👍"],
            [actors.agent.id, "comment", "🎉"],
        ]);
        expect((await call(article.call("get", { ...scope, id: draft.id }))).reactionCount).toBe(2);
    },
);
